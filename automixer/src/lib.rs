use nih_plug::context::track::{HostTrackHandler, HostTrackUpdate};
use nih_plug::prelude::*;
use std::sync::{atomic::Ordering, Arc};

mod dsp;
mod network;
mod params;
mod ui;

pub struct Automixer {
    params: Arc<params::AutomixerParams>,
    registration: network::Registration,
    engine: dsp::Engine,
    vad: pleasant_voice::SpeechVad,
    sample_rate: f32,
    current_mic: usize,
}

impl Default for Automixer {
    fn default() -> Self {
        let network = network::shared_network();
        let registration = network.register();
        let params = Arc::new(params::AutomixerParams::new(
            network.params.clone(),
            registration.slot().cloned(),
            registration.token,
        ));
        Self {
            current_mic: params.mic_index(),
            params,
            registration,
            engine: dsp::Engine::default(),
            vad: pleasant_voice::SpeechVad::new(48_000.0),
            sample_rate: 48_000.0,
        }
    }
}

impl Plugin for Automixer {
    const NAME: &'static str = "Automixer";
    const VENDOR: &'static str = "Fergler";
    const URL: &'static str = "";
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
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn host_track_handler(&self) -> Option<HostTrackHandler> {
        let slot = self.registration.slot()?.clone();
        let token = self.registration.token;
        let index = self.registration.index?;
        let network = self.registration.network.clone();
        let params = self.params.clone();
        Some(Arc::new(move |update| {
            if matches!(update, HostTrackUpdate::Prepare) {
                params.prepare_mic_assignment(&network, index, token);
                return;
            }
            let mut labels = slot.labels.lock().unwrap_or_else(|e| e.into_inner());
            if slot.owner.load(Ordering::Acquire) != token {
                return;
            }
            match update {
                HostTrackUpdate::Name(name) => {
                    labels.name = name
                        .map(|name| name.trim().to_owned())
                        .filter(|name| !name.is_empty());
                }
                HostTrackUpdate::HardwareInput(input) => labels.input = input,
                HostTrackUpdate::Prepare => unreachable!(),
            }
        }))
    }

    fn editor(&mut self, _: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(
            self.params.clone(),
            self.registration.network.clone(),
            self.registration.index,
        )
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = config.sample_rate;
        self.engine = dsp::Engine::new(config.sample_rate, config.max_buffer_size);
        self.vad.set_sample_rate(config.sample_rate as f64);
        self.vad
            .start_worker_enabled(self.params.shared.assist.value() > 0.0);
        context.set_latency_samples(0);
        self.reset();
        true
    }
    fn reset(&mut self) {
        self.engine.reset();
        self.vad.reset();
        if let Some(slot) = self.registration.slot() {
            slot.withdraw();
            slot.state.store(network::WAITING, Ordering::Relaxed);
        }
    }
    fn deactivate(&mut self) {
        if let Some(slot) = self.registration.slot() {
            slot.withdraw();
            slot.state.store(network::WAITING, Ordering::Relaxed);
        }
        self.vad.set_enabled(false);
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let settings = dsp::Settings::from_params(&self.params.shared);
        let mic = self.params.mic_index();
        if mic != self.current_mic {
            self.reset();
            self.current_mic = mic;
        }
        let bypass = self.params.bypass.value();
        self.vad.set_enabled(settings.assist > 0.0 && !bypass);
        let samples = buffer.samples();
        let channels = buffer.channels().max(1) as f64;
        let mut next_event = context.next_event();
        for (index, mut frame) in buffer.iter_samples().enumerate() {
            while let Some(event) = next_event {
                if event.timing() > index as u32 {
                    break;
                }
                if let NoteEvent::MidiCC {
                    channel, cc, value, ..
                } = event
                {
                    self.params.shared.midi(channel, cc, value);
                }
                context.send_event(event);
                next_event = context.next_event();
            }
            let mut power = 0.0;
            let mut mono = 0.0_f32;
            for sample in frame.iter_mut() {
                if !sample.is_finite() {
                    *sample = 0.0;
                }
                power += (*sample as f64).powi(2);
                if sample.abs() > mono.abs() {
                    mono = *sample;
                }
            }
            self.vad.push_sample(mono.clamp(-16.0, 16.0));
            if self.engine.analyze(power / channels) {
                if let Some(slot) = self.registration.slot() {
                    self.engine.control(
                        &self.registration.network,
                        slot,
                        mic,
                        self.vad.speech_env(),
                        bypass,
                        settings,
                    );
                }
            }
            let gain = self
                .engine
                .gain(settings, bypass || self.registration.index.is_none());
            for sample in frame.iter_mut() {
                *sample *= gain;
            }
        }
        // Preserve events in empty buffers and any boundary events supplied by the host.
        while let Some(event) = next_event {
            if let NoteEvent::MidiCC {
                channel, cc, value, ..
            } = event
            {
                self.params.shared.midi(channel, cc, value);
            }
            context.send_event(event);
            next_event = context.next_event();
        }
        self.vad.end_block(samples as f32 / self.sample_rate);
        if let Some(slot) = self.registration.slot() {
            slot.updated_ms
                .store(self.registration.network.now_ms(), Ordering::Relaxed);
            slot.vad_ready.store(self.vad.ready(), Ordering::Relaxed);
            slot.chorus_mix
                .store(self.engine.chorus_mix.value, Ordering::Relaxed);
        }
        ProcessStatus::Normal
    }
}

impl ClapPlugin for Automixer {
    const CLAP_ID: &'static str = "com.fergler.automixer";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Linked microphone gain sharing and chorus protection");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Utility,
        ClapFeature::Mono,
        ClapFeature::Stereo,
    ];
}
impl Vst3Plugin for Automixer {
    const VST3_CLASS_ID: [u8; 16] = *b"FerglerAutomixer";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}
nih_export_clap!(Automixer);
nih_export_vst3!(Automixer);

#[cfg(test)]
mod tests;
