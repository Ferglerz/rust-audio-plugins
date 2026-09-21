//! Tape Stop on MIDI – 70s Analog Tape Brake Effect
//!
//! Cross-platform CLAP/VST3 plugin built with nih-plug.

use nih_plug::prelude::*;
use std::sync::Arc;

pub mod dsp;
pub mod params;
pub mod telemetry;
pub mod ui;

use dsp::TapeStopEngine;
use params::{MidiAssign, TapeStopParams};
use telemetry::TapeStopTelemetry;

pub struct TapeStop {
    params: Arc<TapeStopParams>,
    engine: TapeStopEngine,
    pub telemetry: Arc<TapeStopTelemetry>,
    last_midi_assign: MidiAssign,
}

impl Default for TapeStop {
    fn default() -> Self {
        Self {
            params: Arc::new(TapeStopParams::default()),
            engine: TapeStopEngine::new(),
            telemetry: TapeStopTelemetry::new(),
            last_midi_assign: MidiAssign::Cc,
        }
    }
}

impl Plugin for TapeStop {
    const NAME: &'static str = "Tape Stop";
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
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type BackgroundTask = ();
    type SysExMessage = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.engine.set_sample_rate(buffer_config.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone(), self.telemetry.clone())
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // 1. Sync manual trigger from UI telemetry
        self.engine
            .set_manual_trigger(self.telemetry.get_manual_trigger());

        // 2. Fetch parameter values
        let drop_time = self.params.drop_time.value();
        let xfade_ms = self.params.xfade_ms.value();
        let drop_curve = self.params.drop_curve.value();
        let stereo_div = self.params.stereo_div.value();
        let midi_assign = self.params.midi_assign.value();
        let override_cc = self.params.override_cc.value() as u8;
        let override_note = self.params.override_note.value() as u8;
        let auto_restart = self.params.auto_restart.value();
        let auto_restart_thresh = self.params.auto_restart_thresh.value();
        let power = self.params.power.value();

        let mut max_in_peak: f32 = 0.0;
        let mut max_out_peak: f32 = 0.0;

        let num_samples = buffer.samples();
        let num_channels = buffer.channels();

        // 3. Process sample-by-sample with sample-accurate MIDI event handling
        if midi_assign != self.last_midi_assign {
            self.engine.clear_held_notes();
            self.engine.clear_cc_override();
            self.last_midi_assign = midi_assign;
        }

        let mut next_event = context.next_event();

        for sample_idx in 0..num_samples {
            // Handle any MIDI events scheduled at this sample index
            while let Some(event) = next_event {
                if event.timing() > sample_idx as u32 {
                    break;
                }

                match event {
                    NoteEvent::NoteOn { note, velocity, .. } => {
                        if midi_assign == MidiAssign::Cc || note == override_note {
                            self.engine.note_on(note, velocity);
                        }
                    }
                    NoteEvent::NoteOff { note, .. } => {
                        if midi_assign == MidiAssign::Cc || note == override_note {
                            self.engine.note_off(note);
                        }
                    }
                    NoteEvent::MidiCC { cc, value, .. } => {
                        if midi_assign == MidiAssign::Cc {
                            let cc_val = (value * 127.0).round() as u8;
                            self.engine.handle_midi_cc(cc, cc_val, override_cc);
                        }
                    }
                    _ => {}
                }

                next_event = context.next_event();
            }

            // Read live input sample frame
            let in_l = buffer.as_slice()[0][sample_idx];
            let in_r = if num_channels > 1 {
                buffer.as_slice()[1][sample_idx]
            } else {
                in_l
            };

            max_in_peak = max_in_peak.max(in_l.abs()).max(in_r.abs());

            // Process through tape stop DSP engine
            let (out_l, out_r) = self.engine.process_sample(
                in_l,
                in_r,
                drop_time,
                xfade_ms,
                drop_curve,
                stereo_div,
                auto_restart,
                auto_restart_thresh,
                power,
            );

            max_out_peak = max_out_peak.max(out_l.abs()).max(out_r.abs());

            // Write back to buffer
            buffer.as_slice()[0][sample_idx] = out_l;
            if num_channels > 1 {
                buffer.as_slice()[1][sample_idx] = out_r;
            }
        }

        // 4. Update UI telemetry
        self.telemetry.update(
            self.engine.speed_left(),
            self.engine.speed_right(),
            self.engine.is_braking(),
            self.engine.is_crossfading(),
            self.engine.brake_progress(),
            self.engine.brake_progress_l(),
            self.engine.brake_progress_r(),
            max_in_peak,
            max_out_peak,
            self.engine.live_envelope(),
            self.engine.transient_flash(),
        );

        ProcessStatus::Normal
    }
}

impl ClapPlugin for TapeStop {
    const CLAP_ID: &'static str = "com.fergler.tape-stop";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Tape Stop on MIDI – 70s Analog Tape Brake Effect");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Custom("tape-stop"),
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for TapeStop {
    const VST3_CLASS_ID: [u8; 16] = *b"FerglerTapeStopP";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::PitchShift,
        Vst3SubCategory::Stereo,
    ];
}

nih_export_clap!(TapeStop);
nih_export_vst3!(TapeStop);
