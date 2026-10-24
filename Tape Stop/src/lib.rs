//! Tape Stop on MIDI – 70s Analog Tape Brake Effect
//!
//! Cross-platform CLAP/VST3 plugin built with nih-plug.

use nih_plug::prelude::*;
use nih_plug::wrapper::state::ParamValue;
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

impl TapeStop {
    fn handle_midi_event(
        &mut self,
        event: NoteEvent<()>,
        assign: MidiAssign,
        note_assignment: u8,
        cc_assignment: u8,
    ) {
        match event {
            NoteEvent::NoteOn { note, velocity, .. }
                if assign == MidiAssign::Cc || note == note_assignment =>
            {
                self.engine.note_on(note, velocity);
            }
            NoteEvent::NoteOff { note, .. } => {
                // Release the accepted note even if its assignment changed while held.
                self.engine.note_off(note);
            }
            NoteEvent::MidiCC { cc, value, .. } if assign == MidiAssign::Cc => {
                let cc_val = (value * 127.0).round() as u8;
                self.engine.handle_midi_cc(cc, cc_val, cc_assignment);
            }
            _ => {}
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

    fn filter_state(state: &mut PluginState) {
        // NIH-plug leaves absent parameters unchanged when loading into an
        // existing instance. Explicit defaults make older presets recall Free.
        for (id, value) in [
            ("time_sync", ParamValue::Bool(false)),
            ("tempo_sync", ParamValue::Bool(true)),
            ("tempo", ParamValue::F32(120.0)),
            (
                "drop_division",
                ParamValue::I32(pleasant_dsp::tempo::QUARTER_NOTE),
            ),
            (
                "return_division",
                ParamValue::I32(pleasant_dsp::tempo::QUARTER_NOTE),
            ),
            ("xy_x_axis", ParamValue::I32(0)),
            ("xy_mode", ParamValue::I32(0)),
        ] {
            state.params.entry(id.to_owned()).or_insert(value);
        }
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
        let bpm = pleasant_dsp::tempo::resolve_bpm(
            self.params.tempo_sync.value(),
            context.transport().tempo,
            self.params.tempo.value() as f64,
        );
        let (display_drop, return_sec) = self.params.timing(bpm);
        let drop_time = if self.params.time_sync.value() {
            display_drop * dsp::DROP_RATE_SCALE
        } else {
            display_drop
        };
        self.telemetry
            .tempo
            .store(bpm as f32, std::sync::atomic::Ordering::Relaxed);
        self.telemetry
            .effective_drop
            .store(display_drop, std::sync::atomic::Ordering::Relaxed);
        self.telemetry
            .effective_return
            .store(return_sec, std::sync::atomic::Ordering::Relaxed);
        let xfade_ms = self.params.xfade_ms.value();
        let drop_curve = self.params.drop_curve.value();
        let stereo_div = self.params.stereo_div.value();
        let midi_assign = self.params.midi_assign.value();
        let override_cc = self.params.override_cc.value() as u8;
        let override_note = self.params.override_note.value() as u8;
        let auto_restart = self.params.auto_restart.value();
        let auto_restart_thresh = self.params.auto_restart_thresh.value();
        let power = self.params.power.value();
        let prepared_tape_settings = self
            .engine
            .prepare_block(drop_time, stereo_div, auto_restart_thresh)
            .with_processing_settings(xfade_ms, return_sec, drop_curve, auto_restart, power);

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

                self.handle_midi_event(event, midi_assign, override_note, override_cc);

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
            let (out_l, out_r) = self
                .engine
                .process_sample_prepared([in_l, in_r], prepared_tape_settings);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_roundtrip_preserves_new_controls_and_legacy_state_gets_defaults() {
        let mut state: PluginState = serde_json::from_value(serde_json::json!({
            "params": { "drop_time": { "f32": 0.1374 }, "return_sec": { "f32": 0.9231 } }, "fields": {}
        })).unwrap();
        TapeStop::filter_state(&mut state);
        assert!(matches!(state.params["time_sync"], ParamValue::Bool(false)));
        assert!(matches!(state.params["tempo_sync"], ParamValue::Bool(true)));
        assert!(
            matches!(state.params["drop_time"], ParamValue::F32(v) if (v - 0.1374).abs() < 1e-6)
        );
        state.params.insert("xy_x_axis".into(), ParamValue::I32(1));
        state.params.insert("xy_mode".into(), ParamValue::I32(1));
        state
            .params
            .insert("time_sync".into(), ParamValue::Bool(true));
        let json = serde_json::to_vec(&state).unwrap();
        let mut restored: PluginState = serde_json::from_slice(&json).unwrap();
        TapeStop::filter_state(&mut restored);
        assert!(matches!(restored.params["xy_x_axis"], ParamValue::I32(1)));
        assert!(matches!(restored.params["xy_mode"], ParamValue::I32(1)));
        assert!(matches!(
            restored.params["time_sync"],
            ParamValue::Bool(true)
        ));
    }

    #[test]
    fn synced_drop_reaches_zero_at_the_selected_note_and_preserves_stereo_offsets() {
        let params = TapeStopParams {
            time_sync: BoolParam::new("Sync", true),
            ..Default::default()
        };
        let (drop, ret) = params.timing(120.0);
        assert_eq!((drop, ret), (0.5, 0.5));
        let mut engine = TapeStopEngine::new();
        engine.set_sample_rate(1000.0);
        for divergence in [0.0, 40.0] {
            engine.reset();
            let prepared = engine
                .prepare_block(drop * dsp::DROP_RATE_SCALE, divergence, -18.0)
                .with_processing_settings(50.0, ret, 1.0, false, true);
            engine.note_on(60, 1.0);
            for _ in 0..500 {
                engine.process_sample_prepared([0.3, -0.3], prepared);
            }
            assert!(engine.speed_left() < 1e-6);
            if divergence == 0.0 {
                assert!(engine.speed_right() < 1e-6);
            } else {
                assert!(engine.speed_right() > 0.1);
                for _ in 0..101 {
                    engine.process_sample_prepared([0.3, -0.3], prepared);
                }
                assert!(engine.speed_right() < 1e-6);
            }
        }
    }

    #[test]
    fn held_note_releases_after_assignment_changes() {
        let mut plugin = TapeStop::default();
        plugin.handle_midi_event(
            NoteEvent::NoteOn {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 60,
                velocity: 1.0,
            },
            MidiAssign::Note,
            60,
            1,
        );
        assert!(plugin.engine.is_braking());
        plugin.handle_midi_event(
            NoteEvent::NoteOff {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 61,
                velocity: 0.0,
            },
            MidiAssign::Note,
            61,
            1,
        );
        assert!(
            plugin.engine.is_braking(),
            "Unheld note must not release the accepted note"
        );
        plugin.handle_midi_event(
            NoteEvent::NoteOff {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 60,
                velocity: 0.0,
            },
            MidiAssign::Note,
            61,
            1,
        );
        assert!(!plugin.engine.is_braking());
        assert!(plugin.engine.is_returning());
    }
}
