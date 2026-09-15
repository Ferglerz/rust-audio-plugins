use nih_plug::prelude::*;
use std::sync::Arc;
pub mod band;
pub mod dsp;
mod engine;
mod params;
mod ui;
use engine::{Engine, Shared};
use params::StripParams;
pub struct Damian {
    params: Arc<StripParams>,
    engine: Engine,
    shared: Arc<Shared>,
}
impl Default for Damian {
    fn default() -> Self {
        let params = Arc::new(StripParams::default());
        let shared = Shared::new(params.bands.clone());
        Self {
            params,
            engine: Engine::new(shared.clone(), 44100.0),
            shared,
        }
    }
}
impl Plugin for Damian {
    const NAME: &'static str = "Damian Channel Strip";
    const VENDOR: &'static str = "Damian Birdsey";
    const URL: &'static str = "https://github.com/Ferglerz";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type BackgroundTask = ();
    type SysExMessage = ();
    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn editor(&mut self, _: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone(), self.shared.clone())
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        c: &BufferConfig,
        _: &mut impl InitContext<Self>,
    ) -> bool {
        self.engine = Engine::new(self.shared.clone(), c.sample_rate as f64);
        true
    }
    fn reset(&mut self) {
        self.engine.reset();
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        _: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.engine.sync();
        for mut frame in buffer.iter_samples() {
            let mut x = [0.0; 2];
            for (i, s) in frame.iter_mut().enumerate() {
                x[i] = if s.is_finite() { *s as f64 } else { 0.0 };
            }
            if frame.len() == 1 {
                x[1] = x[0];
            }
            let settings = dsp::CompSettings {
                amount: self.params.compression.smoothed.next() as f64,
                gate: self.params.gate.smoothed.next() as f64,
                hpf: self.params.sc_hpf.smoothed.next() as f64,
                mix: self.params.mix.smoothed.next() as f64 / 100.0,
            };
            let out = self.engine.tick(
                x,
                settings,
                self.params.output.smoothed.next() as f64,
                self.params.eq_on.value(),
                self.params.comp_on.value(),
                self.params.bypass.value(),
            );
            for (i, s) in frame.iter_mut().enumerate() {
                *s = out[i] as f32;
            }
        }
        ProcessStatus::Normal
    }
}
impl Vst3Plugin for Damian {
    const VST3_CLASS_ID: [u8; 16] = *b"DamianBirdseyCS1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Eq,
        Vst3SubCategory::Dynamics,
    ];
}
nih_export_vst3!(Damian);
