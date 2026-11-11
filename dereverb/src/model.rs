//! Embedded DPDFNet and its prepared ONNX tensors. No JUCE or C++ bridge.
use crate::params::Quality;
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
    AsPointer,
};
use std::{path::PathBuf, sync::OnceLock};

pub const BINS: usize = 481;
const MODEL: &[u8] = include_bytes!("../assets/dpdfnet2_48khz_hr.onnx");
const LIVE_MODEL: &[u8] = include_bytes!("../assets/dpdfnet2_48khz_live.onnx");

pub struct Model {
    session: Session,
    initial_state: Vec<f32>,
}

pub struct Tensors {
    pub spectrum: Tensor<f32>,
    pub enhanced: Tensor<f32>,
    state: Tensor<f32>,
    next_state: Tensor<f32>,
}

fn runtime_path() -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        // SAFETY: dladdr only inspects an address in this loaded image. It fills the
        // initialized Dl_info and its filename remains valid while this image is loaded.
        let mut info: libc::Dl_info = unsafe { std::mem::zeroed() };
        if unsafe { libc::dladdr(runtime_path as *const () as *const _, &mut info) } != 0
            && !info.dli_fname.is_null()
        {
            let path = unsafe { std::ffi::CStr::from_ptr(info.dli_fname) };
            let image = PathBuf::from(path.to_string_lossy().as_ref());
            if let Some(macos) = image
                .parent()
                .filter(|p| p.file_name().is_some_and(|n| n == "MacOS"))
            {
                return Ok(macos.join("../Frameworks/libonnxruntime.dylib"));
            }
        }
        // Cargo tests and unbundled development builds only. Installed bundles always
        // resolve their own Frameworks directory, even when its runtime is missing.
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runtime/libonnxruntime.dylib"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::env::var_os("ORT_DYLIB_PATH")
            .map(PathBuf::from)
            .ok_or_else(|| "Set ORT_DYLIB_PATH to ONNX Runtime 1.23.2 or later".into())
    }
}

pub(crate) fn load_session(bytes: &[u8]) -> Result<Session, String> {
    static RUNTIME: OnceLock<Result<(), String>> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            ort::init_from(runtime_path()?)
                .map_err(|e| e.to_string())?
                .with_name("Dereverb")
                .with_telemetry(false)
                .commit();
            Ok(())
        })
        .clone()?;
    (|| -> ort::Result<Session> {
        Session::builder()?
            .with_intra_threads(1)?
            .with_inter_threads(1)?
            .with_parallel_execution(false)?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .commit_from_memory(bytes)
    })()
    .map_err(|e| e.to_string())
}

pub(crate) fn run_prepared(
    session: &mut Session,
    input_names: [&std::ffi::CStr; 2],
    output_names: [&std::ffi::CStr; 2],
    inputs: [&Tensor<f32>; 2],
    outputs: [&mut Tensor<f32>; 2],
) -> bool {
    let inputs = inputs.map(|tensor| tensor.ptr());
    let mut outputs = outputs.map(|tensor| tensor.ptr_mut());
    let input_names = input_names.map(std::ffi::CStr::as_ptr);
    let output_names = output_names.map(std::ffi::CStr::as_ptr);
    let api = ort::api();
    // SAFETY: tensors own their CPU allocations, exclusive output/session borrows
    // prevent concurrent access, and names and pointer arrays outlive this call.
    // ORT writes into the prebound outputs. An error status is owned by this caller.
    unsafe {
        let status = (api.Run)(
            session.ptr_mut(),
            std::ptr::null(),
            input_names.as_ptr(),
            inputs.as_ptr(),
            inputs.len(),
            output_names.as_ptr(),
            outputs.len(),
            outputs.as_mut_ptr(),
        );
        if !status.0.is_null() {
            (api.ReleaseStatus)(status.0);
            return false;
        }
    }
    true
}

impl Model {
    pub fn new(quality: Quality) -> Result<Self, String> {
        let (model_bytes, state_size) = match quality {
            Quality::Live => (LIVE_MODEL, 15_590),
            Quality::HighQuality => (MODEL, 56_436),
        };
        let session = load_session(model_bytes)?;
        let mut initial_state = vec![0.0; state_size];
        let metadata = session.metadata().map_err(|e| e.to_string())?;
        for (key, offset) in [("erb_norm_init", 0), ("spec_norm_init", BINS)] {
            let values = metadata
                .custom(key)
                .ok_or_else(|| format!("Missing model metadata: {key}"))?;
            for (slot, value) in initial_state[offset..].iter_mut().zip(values.split(',')) {
                *slot = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("Invalid model metadata: {key}"))?;
            }
        }
        drop(metadata);
        Ok(Self {
            session,
            initial_state,
        })
    }

    pub fn tensors(&mut self) -> Result<Tensors, String> {
        let mut tensors = (|| -> ort::Result<Tensors> {
            Ok(Tensors {
                spectrum: Tensor::from_array(([1, 1, BINS, 2], vec![0.0; BINS * 2]))?,
                enhanced: Tensor::from_array(([1, 1, BINS, 2], vec![0.0; BINS * 2]))?,
                state: Tensor::from_array((
                    [self.initial_state.len()],
                    self.initial_state.clone(),
                ))?,
                next_state: Tensor::from_array((
                    [self.initial_state.len()],
                    vec![0.0; self.initial_state.len()],
                ))?,
            })
        })()
        .map_err(|e| e.to_string())?;
        // Allocate ORT's execution arena and memory pattern before the audio callback.
        for _ in 0..5 {
            if !self.run(&mut tensors) {
                return Err("Model warm-up failed".into());
            }
        }
        self.reset(&mut tensors);
        Ok(tensors)
    }

    pub fn reset(&self, tensors: &mut Tensors) {
        tensors
            .state
            .extract_tensor_mut()
            .1
            .copy_from_slice(&self.initial_state);
        tensors.next_state.extract_tensor_mut().1.fill(0.0);
    }

    pub fn run(&mut self, tensors: &mut Tensors) -> bool {
        if !run_prepared(
            &mut self.session,
            [c"spec", c"state_in"],
            [c"spec_e", c"state_out"],
            [&tensors.spectrum, &tensors.state],
            [&mut tensors.enhanced, &mut tensors.next_state],
        ) {
            return false;
        }
        let valid = tensors
            .enhanced
            .extract_tensor()
            .1
            .iter()
            .chain(tensors.next_state.extract_tensor().1)
            .all(|x| x.is_finite());
        if valid {
            std::mem::swap(&mut tensors.state, &mut tensors.next_state);
        }
        valid
    }
}
