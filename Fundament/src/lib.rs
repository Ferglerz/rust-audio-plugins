//! Fundament – polyphonic fundamental tracking, partial cuts, and additive resynthesis.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug::prelude::*;

use dsp::FundamentEngine;
use params::FundamentParams;
use telemetry::FundamentTelemetry;

pub mod dsp;
pub mod params;
pub mod telemetry;
pub mod ui;

pub struct Fundament {
    params: Arc<FundamentParams>,
    engine: FundamentEngine,
    pub telemetry: Arc<FundamentTelemetry>,
    last_frames: u64,
}

impl Default for Fundament {
    fn default() -> Self {
        Self {
            params: Arc::new(FundamentParams::default()),
            engine: FundamentEngine::new(),
            telemetry: FundamentTelemetry::new(),
            last_frames: 0,
        }
    }
}

impl Plugin for Fundament {
    const NAME: &'static str = "Fundament";
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

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type BackgroundTask = ();
    type SysExMessage = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        let channels = audio_io_layout
            .main_input_channels
            .map(NonZeroU32::get)
            .unwrap_or(1) as usize;
        self.engine.prepare(
            buffer_config.sample_rate,
            buffer_config.max_buffer_size as usize,
            channels,
        );
        context.set_latency_samples(self.engine.latency_samples());
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
        self.last_frames = 0;
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone(), self.telemetry.clone())
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let settings = self.params.settings();
        self.engine.set_settings(settings);
        self.telemetry
            .cut_depth_db
            .store(settings.cut_depth_db, Ordering::Relaxed);

        let input_peak = buffer_peak(buffer.as_slice());
        if self.params.bypass.value() {
            self.engine.process_bypass(buffer.as_slice());
        } else {
            self.engine.process_block(buffer.as_slice());
        }
        let output_peak = buffer_peak(buffer.as_slice());

        self.telemetry.publish_voices(self.engine.voices());
        self.telemetry.publish_levels(input_peak, output_peak);
        let frames = self.engine.analysis_frames();
        if frames != self.last_frames {
            self.telemetry
                .publish_spectrum(self.engine.analysis_magnitudes(), self.engine.bin_hz());
            self.last_frames = frames;
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Fundament {
    const CLAP_ID: &'static str = "com.fergler.fundament";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Polyphonic fundamental tracking, partial cuts, and additive resynthesis");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Custom("pitch-tracking"),
    ];
}

impl Vst3Plugin for Fundament {
    const VST3_CLASS_ID: [u8; 16] = *b"FerglerFundament";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Synth,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_clap!(Fundament);
nih_export_vst3!(Fundament);

fn buffer_peak(channels: &[&mut [f32]]) -> f32 {
    let mut peak = 0.0f32;
    for channel in channels {
        for sample in channel.iter() {
            peak = peak.max(sample.abs());
        }
    }
    peak
}
