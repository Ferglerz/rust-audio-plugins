use nih_plug::prelude::*;
use std::sync::Arc;
pub mod band;
pub mod dsp;
mod engine;
mod params;
mod processing;
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
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.shared.requested_config.store(
            self.params.processing_config().encode(),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.engine = Engine::new(self.shared.clone(), c.sample_rate as f64);
        context.set_latency_samples(self.engine.latency());
        true
    }
    fn reset(&mut self) {
        self.engine.reset();
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.shared.requested_config.store(
            self.params.processing_config().encode(),
            std::sync::atomic::Ordering::Relaxed,
        );
        self.engine.sync();
        context.set_latency_samples(self.engine.latency());
        for mut frame in buffer.iter_samples() {
            let mut x = [0.0; 2];
            for (i, s) in frame.iter_mut().enumerate() {
                x[i] = if s.is_finite() { *s as f64 } else { 0.0 };
            }
            if frame.len() == 1 {
                x[1] = x[0];
            }
            let settings = dsp::CompSettings {
                threshold: -(self.params.compression.smoothed.next() as f64),
                ratio: self.params.comp_ratio.smoothed.next() as f64,
                soft_knee: self.params.soft_knee.value(),
                auto_makeup: self.params.auto_makeup.value(),
                stereo_link: self.params.stereo_link.value(),
                gate: self.params.gate.smoothed.next() as f64,
                pse: dsp::PseSettings {
                    depth: self.params.pse_depth.smoothed.next() as f64,
                    hysteresis: self.params.pse_hysteresis.smoothed.next() as f64,
                    knee: self.params.pse_knee.smoothed.next() as f64,
                    peak: self.params.pse_peak.value(),
                    time: self.params.pse_time.value(),
                    listen: self.params.pse_listen.value(),
                },
                hpf: self.params.sc_hpf.smoothed.next() as f64,
                dry: self.params.dry.smoothed.next() as f64 / 100.0,
                wet: self.params.wet.smoothed.next() as f64 / 100.0,
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
        if self.engine.latency() > 0 {
            ProcessStatus::Tail(self.engine.latency() * 2)
        } else {
            ProcessStatus::Normal
        }
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
