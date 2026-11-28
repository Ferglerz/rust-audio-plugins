//! Tape Stop on MIDI – 70s Analog Tape Brake Effect
//!
//! Cross-platform CLAP/VST3 plugin built with nih-plug.

use nih_plug::prelude::*;
use nih_plug::wrapper::state::ParamValue;
use std::sync::Arc;

pub mod dsp;
mod midi;
pub mod params;
pub mod telemetry;
pub mod ui;

use dsp::TapeStopEngine;
use params::{MidiAssign, TapeStopParams};
use telemetry::{TapeStopTelemetry, TransportFrame, TransportStage};

pub struct TapeStop {
    params: Arc<TapeStopParams>,
    engine: TapeStopEngine,
    midi: midi::MidiTape,
    pub telemetry: Arc<TapeStopTelemetry>,
    last_midi_assign: MidiAssign,
    position_cc: Option<u8>,
    position_msb: [Option<u8>; 16],
    position_lsb: [Option<u8>; 16],
}

impl Default for TapeStop {
    fn default() -> Self {
        Self {
            params: Arc::new(TapeStopParams::default()),
            engine: TapeStopEngine::new(),
            midi: midi::MidiTape::new(),
            telemetry: TapeStopTelemetry::new(),
            last_midi_assign: MidiAssign::Cc,
            position_cc: None,
            position_msb: [None; 16],
            position_lsb: [None; 16],
        }
    }
}

impl TapeStop {
    fn tape_settings(
        &self,
        host_bpm: Option<f64>,
        read: impl Fn(&FloatParam) -> f32,
    ) -> (f64, f32, f32, dsp::PreparedTapeSettings) {
        let bpm = pleasant_dsp::tempo::resolve_bpm(
            self.params.tempo_sync.value(),
            host_bpm,
            read(&self.params.tempo) as f64,
        );
        let (display_drop, return_sec) = self.params.timing_with(bpm, &read);
        let drop_time = if self.params.time_sync.value() {
            display_drop * dsp::DROP_RATE_SCALE
        } else {
            display_drop
        };
        let prepared = self
            .engine
            .prepare_block(
                drop_time,
                read(&self.params.stereo_div),
                read(&self.params.auto_restart_thresh),
            )
            .with_processing_settings(
                read(&self.params.xfade_ms),
                return_sec,
                read(&self.params.drop_curve),
                self.params.auto_restart.value(),
                self.params.power.value(),
            );
        (bpm, display_drop, return_sec, prepared)
    }

    fn handle_midi_event(
        &mut self,
        event: NoteEvent<()>,
        assign: MidiAssign,
        note_assignment: u8,
        cc_assignment: u8,
    ) -> bool {
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
            NoteEvent::MidiCC {
                channel, cc, value, ..
            } if self
                .position_cc
                .is_some_and(|n| n == cc || (n < 32 && n + 32 == cc)) =>
            {
                let ch = usize::from(channel);
                if ch < 16 {
                    let bytes = (value.clamp(0.0, 1.0) * 127.0).round() as u8;
                    if Some(cc) == self.position_cc {
                        self.position_msb[ch] = Some(bytes);
                    } else {
                        self.position_lsb[ch] = Some(bytes);
                    }
                    if let Some(msb) = self.position_msb[ch] {
                        let position = self.position_lsb[ch].map_or(msb as f32 / 127.0, |lsb| {
                            (u16::from(msb) * 128 + u16::from(lsb)) as f32 / 16383.0
                        });
                        self.engine.set_curve_position(position);
                    }
                }
                return true;
            }
            NoteEvent::MidiCC { cc, value, .. } if assign == MidiAssign::Cc => {
                let cc_val = (value * 127.0).round() as u8;
                self.engine.handle_midi_cc(cc, cc_val, cc_assignment);
            }
            _ => {}
        }
        false
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
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type BackgroundTask = ();
    type SysExMessage = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn filter_state(state: &mut PluginState) {
        use pleasant_dsp::tempo::QUARTER_NOTE;

        // An older preset may be loaded into an instance with these controls set.
        for (id, default) in [
            ("position_cc", ParamValue::I32(-1)),
            ("affect_midi", ParamValue::Bool(true)),
            ("time_sync", ParamValue::Bool(false)),
            ("tempo_sync", ParamValue::Bool(true)),
            ("tempo", ParamValue::F32(120.0)),
            ("drop_division", ParamValue::I32(QUARTER_NOTE)),
            ("return_division", ParamValue::I32(QUARTER_NOTE)),
            ("xy_x_axis", ParamValue::I32(0)),
            ("xy_mode", ParamValue::I32(0)),
        ] {
            if !state.params.contains_key(id) {
                state.params.insert(id.to_owned(), default);
            }
        }
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.engine.set_sample_rate(buffer_config.sample_rate);
        self.midi.set_sample_rate(buffer_config.sample_rate);
        true
    }

    fn reset(&mut self) {
        self.engine.reset();
        self.position_msb.fill(None);
        self.position_lsb.fill(None);
        self.midi.reset();
        self.telemetry.publish_transport(TransportFrame::default());
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
        let host_bpm = context.transport().tempo;
        let (bpm, display_drop, return_sec, settled_settings) =
            self.tape_settings(host_bpm, FloatParam::value);
        let smoothing_samples = self.params.smoothing_samples();
        self.telemetry
            .tempo
            .store(bpm as f32, std::sync::atomic::Ordering::Relaxed);
        self.telemetry
            .effective_drop
            .store(display_drop, std::sync::atomic::Ordering::Relaxed);
        self.telemetry
            .effective_return
            .store(return_sec, std::sync::atomic::Ordering::Relaxed);
        let midi_assign = self.params.midi_assign.value();
        let override_cc = self.params.override_cc.value() as u8;
        let override_note = self.params.override_note.value() as u8;
        let power = self.params.power.value();
        let affect_midi = self.params.affect_midi.value() && power && !self.params.bypass.value();
        self.midi
            .begin_block(affect_midi, &mut |event| context.send_event(event));
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

        let position_cc = u8::try_from(self.params.position_cc.value()).ok();
        if position_cc != self.position_cc {
            self.engine.set_curve_position(0.0);
            self.position_msb.fill(None);
            self.position_lsb.fill(None);
            self.position_cc = position_cc;
        }

        let mut next_event = context.next_event();

        for sample_idx in 0..num_samples {
            // Handle any MIDI events scheduled at this sample index
            while let Some(event) = next_event {
                if event.timing() > sample_idx as u32 {
                    break;
                }

                let position_event =
                    self.handle_midi_event(event, midi_assign, override_note, override_cc);
                if !position_event || !affect_midi {
                    self.midi
                        .push(event, &mut |event| context.send_event(event));
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
            let prepared_tape_settings = if sample_idx < smoothing_samples {
                self.tape_settings(host_bpm, |p| p.smoothed.next()).3
            } else {
                settled_settings
            };
            let (out_l, out_r) = self
                .engine
                .process_sample_prepared([in_l, in_r], prepared_tape_settings);

            // MIDI has one tape head: use the mean of the stereo tape speeds.
            self.midi.sample(
                sample_idx as u32,
                (self.engine.speed_left() + self.engine.speed_right()) * 0.5,
                self.engine.take_live_jump(),
                !self.engine.is_braking() && !self.engine.is_returning(),
                &mut |event| context.send_event(event),
            );

            max_out_peak = max_out_peak.max(out_l.abs()).max(out_r.abs());

            // Write back to buffer
            buffer.as_slice()[0][sample_idx] = out_l;
            if num_channels > 1 {
                buffer.as_slice()[1][sample_idx] = out_r;
            }
        }

        // 4. Update UI telemetry
        if num_samples > 0 {
            self.telemetry.publish_transport(TransportFrame {
                stage: if self.engine.position_active() {
                    TransportStage::Position
                } else if self.engine.is_braking() {
                    TransportStage::Braking
                } else if self.engine.is_returning() {
                    TransportStage::Returning
                } else if self.engine.is_crossfading() {
                    TransportStage::Crossfading
                } else {
                    TransportStage::Live
                },
                speed: [self.engine.speed_left(), self.engine.speed_right()],
                brake_progress: [
                    self.engine.brake_progress_l(),
                    self.engine.brake_progress_r(),
                ],
            });
        }
        self.telemetry.update_levels(
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
        ClapFeature::NoteEffect,
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
    fn state_roundtrip_preserves_controls() {
        let state: PluginState = serde_json::from_value(serde_json::json!({
            "params": {
                "drop_time": { "f32": 0.1374 },
                "return_sec": { "f32": 0.9231 },
                "affect_midi": { "bool": false },
                "xy_x_axis": { "i32": 1 },
                "xy_mode": { "i32": 1 },
                "time_sync": { "bool": true }
            },
            "fields": {}
        }))
        .unwrap();
        let json = serde_json::to_vec(&state).unwrap();
        let restored: PluginState = serde_json::from_slice(&json).unwrap();
        assert!(matches!(
            restored.params["drop_time"],
            ParamValue::F32(v) if (v - 0.1374).abs() < 1e-6
        ));
        assert!(matches!(
            restored.params["affect_midi"],
            ParamValue::Bool(false)
        ));
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
    fn expression_takes_over_a_stopped_head_and_holds_the_selected_pitch() {
        let mut plugin = TapeStop {
            position_cc: Some(11),
            ..Default::default()
        };
        plugin.engine.set_sample_rate(1000.0);
        let settings = plugin
            .engine
            .prepare_block(0.05, 0.0, -18.0)
            .with_processing_settings(10.0, 0.1, 2.0, false, true);
        let cc = |plugin: &mut TapeStop, channel, cc, byte: u8| {
            plugin.handle_midi_event(
                NoteEvent::MidiCC {
                    timing: 0,
                    channel,
                    cc,
                    value: byte as f32 / 127.0,
                },
                MidiAssign::Note,
                60,
                3,
            )
        };
        let tick = |plugin: &mut TapeStop| {
            plugin.engine.process_sample_prepared([0.3; 2], settings);
            plugin.engine.speed_left()
        };
        plugin.engine.note_on(60, 1.0);
        for _ in 0..1000 {
            tick(&mut plugin);
        }
        assert_eq!(plugin.engine.speed_left(), 0.0);
        assert!(!cc(&mut plugin, 0, 10, 80));
        assert!(cc(&mut plugin, 1, 43, 100)); // fine byte from another channel
        assert_eq!(tick(&mut plugin), 0.0);
        assert!(cc(&mut plugin, 0, 11, 32));
        let speed = 1.0 - dsp::s_curve(32.0 / 127.0, 2.0);
        let first = tick(&mut plugin);
        assert!(first > 0.0 && first < speed);
        plugin.engine.note_off(60);
        for _ in 0..10 {
            tick(&mut plugin);
        }
        assert_eq!(tick(&mut plugin), speed);
        assert!(cc(&mut plugin, 0, 43, 64));
        for _ in 0..5 {
            tick(&mut plugin);
        }
        assert_eq!(
            tick(&mut plugin),
            1.0 - dsp::s_curve((32.0 * 128.0 + 64.0) / 16383.0, 2.0)
        );
        assert!(plugin.engine.position_active());
        plugin.engine.set_curve_position(0.0);
        for _ in 0..20 {
            tick(&mut plugin);
        }
        assert!(!plugin.engine.is_braking());
        assert_eq!(plugin.engine.speed_left(), 1.0);
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
