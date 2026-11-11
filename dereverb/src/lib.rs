use nih_plug::{prelude::*, wrapper::state::ParamValue};
use std::sync::{atomic::Ordering, Arc};

mod dsp;
mod dtln;
mod model;
mod params;
mod resample;
mod telemetry;
mod transients;
mod ui;
mod weighting;
mod wpe;

const MODES: usize = params::EngineKind::MODES.len();

pub struct Dereverb {
    params: Arc<params::DereverbParams>,
    shared: Arc<telemetry::Telemetry>,
    engines: [Option<dsp::Engine>; MODES],
    active_quality: params::Quality,
    active_engine: params::EngineKind,
    latency: u32,
    input_paths: Vec<InputPath>,
    transition: Option<ModeTransition>,
    meter_energy: [f64; 2],
    meter_count: usize,
    meter_window: usize,
}

struct InputPath {
    dry: dsp::DelayLine,
    guard: Option<transients::Guard>,
}

struct ModeTransition {
    engine: params::EngineKind,
    quality: params::Quality,
    elapsed: usize,
}

impl Default for Dereverb {
    fn default() -> Self {
        Self {
            params: Arc::new(params::DereverbParams::default()),
            shared: Arc::default(),
            engines: Default::default(),
            active_quality: params::Quality::Live,
            active_engine: params::EngineKind::DpdfNet,
            latency: 0,
            input_paths: Vec::new(),
            transition: None,
            meter_energy: [0.0; 2],
            meter_count: 0,
            meter_window: 480,
        }
    }
}

impl Dereverb {
    fn publish_engine_state(&self) {
        let index = self.active_engine.index(self.active_quality);
        self.shared
            .quality
            .store(self.active_quality.index() as u32, Ordering::Relaxed);
        self.shared
            .engine
            .store(self.active_engine as u32, Ordering::Relaxed);
        self.shared.status.store(
            if self.engines[index].is_some() {
                telemetry::ACTIVE
            } else {
                telemetry::FAILED
            },
            Ordering::Relaxed,
        );
    }

    fn apply_selection(&mut self) {
        let quality = self.params.quality.value();
        let engine = self.params.engine.value();
        if self.transition.is_some()
            || engine.index(quality) == self.active_engine.index(self.active_quality)
        {
            return;
        }
        if let Some(target) = &mut self.engines[engine.index(quality)] {
            target.reset();
        }
        self.transition = Some(ModeTransition {
            engine,
            quality,
            elapsed: 0,
        });
    }

    fn process_frame<'a>(
        &mut self,
        samples: impl ExactSizeIterator<Item = &'a mut f32>,
        strength: f32,
    ) {
        let active = self.active_engine.index(self.active_quality);
        let latency = self.latency as usize;
        let channels = samples.len();
        let guard_amount = self.params.transient_guard.value() * 0.01;
        // Keep the old path audible while the new path fills, then crossfade
        // the latency-aligned outputs over one 10 ms meter window.
        let blend = self.transition.as_ref().map(|target| {
            let mix = target.elapsed.saturating_sub(latency) as f32 / self.meter_window as f32;
            (target.engine.index(target.quality), mix)
        });
        for (channel, sample) in samples.enumerate() {
            let input = if sample.is_finite() { *sample } else { 0.0 };
            let path = &mut self.input_paths[channel];
            let dry = path.dry.tick(input);
            let protection = path
                .guard
                .as_mut()
                .map_or(0.0, |guard| guard.tick(input, guard_amount));
            let mut output = self.engines[active].as_mut().map_or(dry, |engine| {
                engine.process_sample(channel, input, dry, strength)
            });
            if let Some((target, mix)) = blend {
                let wet = self.engines[target].as_mut().map_or(dry, |engine| {
                    engine.process_sample(channel, input, dry, strength)
                });
                output = output * (1.0 - mix) + wet * mix;
            }
            if protection > 0.0 && strength > 0.0 {
                // Restore aligned dry consonants after the mode crossfade.
                // Both engines continue running and keep their learned state.
                output = output * (1.0 - protection) + dry * protection;
            }
            *sample = output;
            self.meter_energy[0] += (dry as f64).powi(2);
            self.meter_energy[1] += (output as f64).powi(2);
        }
        if let Some(target) = &mut self.transition {
            target.elapsed += 1;
            if target.elapsed >= latency + self.meter_window {
                self.active_engine = target.engine;
                self.active_quality = target.quality;
                self.transition = None;
                self.publish_engine_state();
            }
        }
        self.meter_count += 1;
        if self.meter_count == self.meter_window {
            let count = (self.meter_window * channels.max(1)) as f64;
            self.shared.push(
                (self.meter_energy[0] / count).sqrt() as f32,
                (self.meter_energy[1] / count).sqrt() as f32,
            );
            if let Some(engine) = &self.engines[self.active_engine.index(self.active_quality)] {
                let (channel, level, weight) = engine.activity();
                let protection = self.input_paths[channel]
                    .guard
                    .as_ref()
                    .map_or(0.0, transients::Guard::amount);
                self.shared
                    .set_activity(level, weight, strength * weight * (1.0 - protection));
            }
            self.meter_count = 0;
            self.meter_energy = [0.0; 2];
        }
    }
}

const fn layout(channels: u32) -> AudioIOLayout {
    AudioIOLayout {
        main_input_channels: NonZeroU32::new(channels),
        main_output_channels: NonZeroU32::new(channels),
        ..AudioIOLayout::const_default()
    }
}

impl Plugin for Dereverb {
    const NAME: &'static str = "Dereverb";
    const VENDOR: &'static str = "Fergler";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        layout(2),
        layout(1),
        layout(3),
        layout(4),
        layout(5),
        layout(6),
        layout(7),
        layout(8),
        layout(9),
        layout(10),
        layout(11),
        layout(12),
        layout(13),
        layout(14),
        layout(15),
        layout(16),
    ];
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn filter_state(state: &mut PluginState) {
        if let Some(value @ ParamValue::Bool(_)) = state.params.get_mut("transient_guard") {
            let percent = if matches!(value, ParamValue::Bool(true)) {
                100.0
            } else {
                0.0
            };
            *value = ParamValue::F32(percent);
        }
    }
    fn editor(&mut self, _: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone(), self.shared.clone())
    }
    fn initialize(
        &mut self,
        layout: &AudioIOLayout,
        config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.shared
            .sample_rate
            .store(config.sample_rate, Ordering::Relaxed);
        let channels = layout.main_input_channels.map_or(0, |n| n.get() as usize);
        let mut errors = self.shared.errors.lock().unwrap_or_else(|e| e.into_inner());
        for (kind, quality) in params::EngineKind::MODES {
            let index = kind.index(quality);
            match dsp::Engine::prepare(config.sample_rate as f64, channels, kind, quality) {
                Ok(engine) => {
                    self.engines[index] = Some(engine);
                    errors[index].clear();
                }
                Err(error) => {
                    nih_error!("Dereverb {kind:?} {quality:?}: {error}");
                    errors[index] = error;
                    self.engines[index] = None;
                }
            }
        }
        drop(errors);
        // Hosts compensate one latency for the entire activation. Pad the
        // faster engines so changing modes cannot require a host restart.
        self.latency = self
            .engines
            .iter()
            .flatten()
            .map(dsp::Engine::latency)
            .max()
            .unwrap_or(0);
        for engine in self.engines.iter_mut().flatten() {
            engine.pad_to_latency(self.latency);
        }
        self.input_paths = (0..channels)
            .map(|_| InputPath {
                dry: dsp::DelayLine::new(self.latency as usize),
                guard: (self.latency != 0).then(|| {
                    transients::Guard::new(config.sample_rate as f64, self.latency as usize)
                }),
            })
            .collect();
        self.shared.latency.store(self.latency, Ordering::Relaxed);
        context.set_latency_samples(self.latency);
        self.meter_window = (config.sample_rate * 0.01).round().max(1.0) as usize;
        self.reset();
        true // Explicit, visible passthrough if the runtime/model could not load.
    }
    fn reset(&mut self) {
        self.transition = None;
        self.active_engine = self.params.engine.value();
        self.active_quality = self.params.quality.value();
        if let Some(engine) = &mut self.engines[self.active_engine.index(self.active_quality)] {
            engine.reset();
        }
        for path in &mut self.input_paths {
            path.dry.reset();
            if let Some(guard) = &mut path.guard {
                guard.reset();
            }
        }
        self.publish_engine_state();
        self.params
            .strength
            .smoothed
            .reset(self.params.strength.value());
        self.shared.clear();
        self.meter_energy = [0.0; 2];
        self.meter_count = 0;
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        _: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.apply_selection();
        let weighting = self.params.weighting();
        if let Some(engine) = &mut self.engines[self.active_engine.index(self.active_quality)] {
            engine.set_weighting(weighting);
        }
        let target_engine = self
            .transition
            .as_ref()
            .and_then(|target| self.engines[target.engine.index(target.quality)].as_mut());
        if let Some(engine) = target_engine {
            engine.set_weighting(weighting);
        }
        for mut frame in buffer.iter_samples() {
            let strength = self.params.strength.smoothed.next() * 0.01;
            self.process_frame(frame.iter_mut(), strength);
        }
        if let Some(engine) = &self.engines[self.active_engine.index(self.active_quality)] {
            self.shared.status.store(
                if engine.recovering() {
                    telemetry::RECOVERING
                } else {
                    telemetry::ACTIVE
                },
                Ordering::Relaxed,
            );
        }
        if self.latency == 0 {
            ProcessStatus::Normal
        } else {
            ProcessStatus::Tail(self.latency)
        }
    }
}
impl ClapPlugin for Dereverb {
    const CLAP_ID: &'static str = "com.fergler.dereverb";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Neural and weighted-prediction voice dereverberation");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Restoration,
        ClapFeature::Mono,
        ClapFeature::Stereo,
        ClapFeature::Surround,
    ];
}
impl Vst3Plugin for Dereverb {
    const VST3_CLASS_ID: [u8; 16] = *b"FerglerDereverb_";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Restoration];
}
nih_export_clap!(Dereverb);
nih_export_vst3!(Dereverb);

#[cfg(test)]
mod level_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod wpe_tests;
