mod calibration;
pub mod circuit;
mod circuit_network;
pub mod engine;
pub mod era;
mod field;
mod harp;
mod jordan_lfo;
pub mod params;
pub mod period_panel;
mod peterson;
mod tine;
pub mod ui;
use engine::{Stage73Engine, HIGH, LOW};
use nih_plug::{midi::control_change, prelude::*};
use params::StageParams;
use std::sync::Arc;

pub struct Stage73 {
    params: Arc<StageParams>,
    engine: Stage73Engine,
}

impl Default for Stage73 {
    fn default() -> Self {
        Self {
            params: Arc::default(),
            engine: Stage73Engine::new(44100.0),
        }
    }
}
pub fn dispatch(engine: &mut Stage73Engine, event: &NoteEvent<()>) {
    match *event {
        NoteEvent::NoteOn { note, velocity, .. } => engine.note_on(note, velocity),
        NoteEvent::NoteOff { note, .. } if note > 127 => engine.release_all(),
        NoteEvent::NoteOff { note, .. } => engine.note_off(note),
        NoteEvent::Choke { note, .. } if note > 127 => {
            engine.choke_all();
        }
        NoteEvent::Choke { note, .. } => engine.choke(note),
        NoteEvent::MidiCC {
            cc: control_change::DAMPER_PEDAL,
            value,
            ..
        } => engine.sustain(value >= 0.5),
        NoteEvent::MidiCC {
            cc: control_change::RESET_ALL_CONTROLLERS,
            ..
        } => engine.sustain(false),
        NoteEvent::MidiCC {
            cc: control_change::ALL_SOUND_OFF,
            ..
        } => {
            engine.choke_all();
        }
        NoteEvent::MidiCC { cc: 123..=127, .. } => engine.release_all(),
        _ => {}
    }
}
impl Stage73 {
    fn render_block(
        &mut self,
        out: &mut [f32],
        mut right: Option<&mut [f32]>,
        next: impl FnMut() -> Option<NoteEvent<()>>,
    ) {
        let keyboard = &self.params.keyboard;
        keyboard.drain_auditions(LOW..=HIGH, |note, velocity| {
            self.engine.audition(note, velocity)
        });
        let mut cursor = 0;
        pleasant_dsp::instrument::render_events(
            &mut self.engine,
            out,
            next,
            NoteEvent::timing,
            |engine, event| {
                if let NoteEvent::NoteOn { note, velocity, .. } = *event {
                    if velocity > 0.0 && (LOW..=HIGH).contains(&note) {
                        keyboard.attack(note);
                    }
                }
                dispatch(engine, event);
            },
            |engine, left| {
                if let Some(r) = right.as_deref_mut() {
                    let end = cursor + left.len();
                    engine.render_stereo(left, &mut r[cursor..end]);
                    cursor = end;
                } else {
                    engine.render(left);
                }
            },
        );
        keyboard.publish(|| self.engine.playing_notes());
    }
}

impl Plugin for Stage73 {
    const NAME: &'static str = "Stage 73";
    const VENDOR: &'static str = "Fergler";
    const URL: &'static str = "https://github.com/Ferglerz/rust-audio-plugins";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    type SysExMessage = ();
    type BackgroundTask = ();
    fn filter_state(state: &mut PluginState) {
        use nih_plug::wrapper::state::ParamValue;
        // Earlier sessions controlled tremolo entirely by depth, including host automation.
        state
            .params
            .entry("trem_enabled".into())
            .or_insert(ParamValue::Bool(true));
        if !state.params.contains_key("era") {
            state.params.insert(
                "era".into(),
                ParamValue::String(era::Era::LateI.slug().into()),
            );
            for (id, default) in [
                ("jordan_volume", 0.8),
                ("jordan_bass", 0.5),
                ("jordan_treble", 0.5),
                ("trem_rate", 5.0),
                ("trem_depth", 0.0),
            ] {
                state
                    .params
                    .entry(id.into())
                    .or_insert(ParamValue::F32(default));
            }
        }
    }
    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }
    fn editor(&mut self, _: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone())
    }
    fn initialize(
        &mut self,
        _: &AudioIOLayout,
        config: &BufferConfig,
        ctx: &mut impl InitContext<Self>,
    ) -> bool {
        if !config.sample_rate.is_finite() || config.sample_rate < 8000.0 {
            return false;
        }
        self.engine = Stage73Engine::new(config.sample_rate as f64);
        self.engine.set_controls(self.params.controls());
        self.engine.reset();
        self.params.keyboard.publish(|| 0);
        ctx.set_latency_samples(engine::LATENCY);
        true
    }
    fn reset(&mut self) {
        self.engine.set_controls(self.params.controls());
        self.engine.reset();
        self.params.keyboard.publish(|| 0);
    }
    fn process(
        &mut self,
        buffer: &mut Buffer,
        _: &mut AuxiliaryBuffers,
        ctx: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        self.engine.set_controls(self.params.controls());
        let (left, others) = buffer
            .as_slice()
            .split_first_mut()
            .expect("instrument requires output");
        self.render_block(left, others.first_mut().map(|ch| &mut **ch), || {
            ctx.next_event()
        });
        if self.engine.is_ringing() {
            ProcessStatus::KeepAlive
        } else {
            ProcessStatus::Normal
        }
    }
}
impl ClapPlugin for Stage73 {
    const CLAP_ID: &'static str = "com.fergler.stage73";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A physically modeled 73-key tine electric piano");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::Instrument, ClapFeature::Synthesizer];
}
impl Vst3Plugin for Stage73 {
    const VST3_CLASS_ID: [u8; 16] = *b"FergStage73Piano";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}
#[cfg(test)]
mod tests {
    use super::*;
    use pleasant_dsp::instrument::render_events;
    fn rms(a: &[f32]) -> f64 {
        (a.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / a.len() as f64).sqrt()
    }
    fn harmonic(samples: &[f32], hz: f64, rate: f64) -> f64 {
        let mut re = 0.0;
        let mut im = 0.0;
        for (i, &sample) in samples.iter().enumerate() {
            let phase = std::f64::consts::TAU * i as f64;
            let windowed = (0.5 - 0.5 * (phase / samples.len() as f64).cos()) * sample as f64;
            let (sin, cos) = (phase * hz / rate).sin_cos();
            re += windowed * cos;
            im += windowed * sin;
        }
        re.hypot(im)
    }
    fn render(e: &mut Stage73Engine, n: usize) -> Vec<f32> {
        let mut x = vec![0.0; n];
        e.render(&mut x);
        x
    }
    fn note(time: u32, n: u8, v: f32) -> NoteEvent<()> {
        NoteEvent::NoteOn {
            timing: time,
            voice_id: None,
            channel: 0,
            note: n,
            velocity: v,
        }
    }
    #[test]
    fn timed_events_and_restrike_keep_one_physical_key() {
        let mut a = Stage73Engine::new(48000.0);
        let mut b = Stage73Engine::new(48000.0);
        let mut stream = [note(113, 55, 0.7), note(1024, 55, 0.7)].into_iter();
        let mut block = [0.0; 1024];
        render_events(
            &mut a,
            &mut block,
            || stream.next(),
            NoteEvent::timing,
            dispatch,
            Stage73Engine::render,
        );
        assert!(block[..113].iter().all(|s| *s == 0.0));
        b.note_on(55, 0.7);
        assert_eq!(block[113..], render(&mut b, 911));
        b.note_on(55, 0.7);
        let restrike = render(&mut a, 1200);
        assert_eq!(restrike, render(&mut b, 1200));
        b.reset();
        b.note_on(55, 0.7);
        assert_ne!(restrike, render(&mut b, 1200));
    }
    #[test]
    fn keyboard_audition_renders_and_shares_the_tine_with_midi() {
        let mut plugin = Stage73::default();
        let mut audio = [0.0; 512];
        plugin.params.keyboard.audition(Some(61));
        plugin.render_block(&mut audio, None, || None);
        let mut midi = Stage73Engine::new(44100.0);
        midi.note_on(61, 0.8);
        assert_eq!(audio.as_slice(), render(&mut midi, 512));
        assert!(rms(&audio) > 1e-4);

        plugin.engine.note_on(61, 0.8);
        plugin.params.keyboard.audition(None);
        plugin.render_block(&mut audio, None, || None);
        assert_eq!(plugin.engine.playing_notes(), 1 << 61);
        plugin.params.keyboard.audition(Some(61));
        plugin.render_block(&mut audio, None, || None);
        plugin.engine.note_off(61);
        assert_eq!(plugin.engine.playing_notes(), 1 << 61);
        plugin.engine.sustain(true);
        plugin.params.keyboard.audition(Some(100));
        plugin.render_block(&mut audio, None, || None);
        assert_eq!(plugin.engine.playing_notes(), (1 << 61) | (1 << 100));
        plugin.engine.sustain(false);
        assert_eq!(plugin.engine.playing_notes(), 1 << 100);
        plugin.params.keyboard.audition(None);
        plugin.render_block(&mut audio, None, || None);
        assert_eq!(plugin.engine.playing_notes(), 0);
    }
    #[test]
    fn pedal_holds_released_notes_and_only_damps_unheld_keys() {
        for rate in [44100, 48000, 96000] {
            let mut held = Stage73Engine::new(rate as f64);
            held.note_on(55, 0.8);
            held.sustain(true);
            held.note_off(55);
            let before = render(&mut held, rate / 3);
            assert!(rms(&before[rate / 4..]) > 1e-3);
            held.sustain(false);
            let stopped = render(&mut held, rate / 2);
            assert!(rms(&stopped[rate / 3..]) < 1e-6);
            held.note_on(55, 0.8);
            held.sustain(true);
            held.sustain(false);
            let sound = render(&mut held, rate / 2);
            assert!(rms(&sound[rate / 3..]) > 1e-3);
        }
    }
    #[test]
    fn compass_choke_and_live_panel_automation() {
        let mut e = Stage73Engine::new(44100.0);
        for n in [0, 27, 101, 127, 255] {
            e.note_on(n, 1.0);
        }
        assert!(render(&mut e, 256).iter().all(|s| *s == 0.0));
        for n in LOW..=HIGH {
            e.note_on(n, 0.6);
        }
        e.sustain(true);
        e.release_all();
        assert!(rms(&render(&mut e, 4096)) > 0.005);
        let c = engine::Controls {
            volume: 0.0,
            bass: 0.2,
            height_mm: -2.0,
            gap_mm: -1.0,
            ..Default::default()
        };
        e.set_controls(c);
        let x = render(&mut e, 4096);
        assert!(x.iter().all(|s| s.is_finite()));
        assert_eq!(rms(&x[2000..]), 0.0);
        e.set_controls(engine::Controls::default());
        assert!(rms(&render(&mut e, 4096)) > 0.001);
        dispatch(
            &mut e,
            &NoteEvent::Choke {
                timing: 0,
                voice_id: None,
                channel: 0,
                note: 255,
            },
        );
        let x = render(&mut e, 4096);
        assert!(rms(&x[3000..]) < 1e-9);
    }
    #[test]
    fn hammer_tip_boundaries_play_on_both_sides_at_all_rates() {
        let boundaries = [28, 50, 51, 60, 61, 70, 71, 84, 85, 100];
        for rate in [44100, 48000, 96000] {
            let mut e = Stage73Engine::new(rate as f64);
            for n in boundaries {
                e.reset();
                e.note_on(n, 0.37);
                let p = rms(&render(&mut e, rate / 10));
                e.reset();
                e.note_on(n, 1.0);
                let f = rms(&render(&mut e, rate / 10));
                assert!(p > 1e-6 && f > p, "MIDI {n} at {rate}: {p} / {f}");
            }
        }
    }
    #[test]
    fn centered_pickup_suppresses_odd_harmonics_and_gap_changes_output() {
        let rate = 48000.0;
        let hz = 440.0 * 2f64.powf((55.0 - 69.0) / 12.0);
        let render_at = |height, gap| {
            let mut e = Stage73Engine::new(rate);
            e.set_controls(engine::Controls {
                height_mm: height,
                gap_mm: gap,
                ..Default::default()
            });
            e.reset();
            e.note_on(55, 0.6);
            render(&mut e, 14400)
        };
        let partial = |x: &[f32], h: f64| harmonic(&x[2400..], h * hz, rate);
        let center = render_at(-era::Era::LateI.geometry(27).0, 0.0);
        let normal = render_at(0.0, 0.0);
        assert!(partial(&center, 2.0) > 8.0 * partial(&center, 1.0));
        assert!(partial(&normal, 1.0) > partial(&normal, 2.0));
        assert!(rms(&render_at(0.0, -0.7)) > rms(&render_at(0.0, 0.7)));
    }

    #[test]
    fn late_mark_i_octave_balance_and_growth_follow_dry_b_flat_4() {
        // Matt's jack recording, 90–300 ms. B-flat 4 is outside the voicing fit.
        let hz = 440.0 * 2f64.powf(1.0 / 12.0);
        let measured = [-40.07, -19.65];
        let modeled = [0.37075, 1.0].map(|velocity| {
            let mut e = Stage73Engine::new(48000.0);
            e.note_on(70, velocity);
            let audio = render(&mut e, 14400);
            let window = &audio[4320..];
            20.0 * (harmonic(window, 2.0 * hz, 48000.0) / harmonic(window, hz, 48000.0)).log10()
        });
        for (level, reference) in modeled.into_iter().zip(measured) {
            assert!(
                (level - reference).abs() < 6.0,
                "octave {level} vs {reference} dB"
            );
        }
        let growth_error = (modeled[1] - modeled[0]) - (measured[1] - measured[0]);
        assert!(
            growth_error.abs() < 3.1,
            "octave growth error {growth_error} dB"
        );
    }
}

#[cfg(test)]
mod era_tests {
    use super::*;
    use crate::{engine::Controls, era::Era};
    use nih_plug::wrapper::state::ParamValue;

    #[test]
    fn tremolo_switch_preserves_depth_and_legacy_automation() {
        let mut params = StageParams::default();
        assert_eq!(params.controls().depth, 0.0);
        params.tone.tremolo = BoolParam::new("Tremolo", true);
        assert_eq!(params.controls().depth, 0.5);
        params.tone.tremolo = BoolParam::new("Tremolo", false);
        assert_eq!(params.controls().depth, 0.0);
        assert_eq!(params.depth.value(), 0.5);

        for depth in [0.0, 0.37] {
            let mut state = PluginState {
                version: "0.1.0".into(),
                params: [
                    ("era".into(), ParamValue::String("sparkletop".into())),
                    ("trem_depth".into(), ParamValue::F32(depth)),
                ]
                .into(),
                fields: Default::default(),
            };
            Stage73::filter_state(&mut state);
            assert!(matches!(
                state.params["trem_enabled"],
                ParamValue::Bool(true)
            ));
            assert!(matches!(state.params["trem_depth"], ParamValue::F32(v) if v == depth));
            state
                .params
                .insert("trem_enabled".into(), ParamValue::Bool(false));
            Stage73::filter_state(&mut state);
            assert!(matches!(
                state.params["trem_enabled"],
                ParamValue::Bool(false)
            ));
        }
    }

    #[test]
    fn restoring_legacy_state_selects_the_original_instrument() {
        let mut state = PluginState {
            version: "0.1.0".into(),
            params: Default::default(),
            fields: Default::default(),
        };
        state
            .params
            .insert("bass_boost".into(), ParamValue::F32(0.42));
        Stage73::filter_state(&mut state);
        assert!(matches!(&state.params["era"],ParamValue::String(s) if s=="late_i"));
        assert!(matches!(state.params["bass_boost"],ParamValue::F32(v) if v==0.42));
        state
            .params
            .insert("era".into(), ParamValue::String("mark_v".into()));
        Stage73::filter_state(&mut state);
        assert!(matches!(&state.params["era"],ParamValue::String(s) if s=="mark_v"));
    }

    #[test]
    fn era_automation_keeps_sustained_and_auditioned_keys() {
        for rate in [44100., 48000., 96000.] {
            let mut engine = Stage73Engine::new(rate);
            engine.sustain(true);
            engine.note_on(42, 0.6);
            engine.note_off(42);
            engine.audition(Some(55), 0.7);
            let mut audio = [0.; 137];
            for era in Era::ALL.into_iter().cycle().take(15) {
                engine.set_controls(Controls {
                    era,
                    ..Default::default()
                });
                engine.render(&mut audio);
                assert!(audio.iter().all(|v| v.is_finite()));
                assert_eq!(engine.playing_notes(), (1 << 42) | (1 << 55));
            }
            engine.audition(None, 0.);
            engine.sustain(false);
            engine.render(&mut vec![0.; rate as usize]);
            assert_eq!(engine.playing_notes(), 0);
            engine.render(&mut audio);
            assert!(audio.iter().all(|v| v.abs() < 1e-6));
        }
    }

    #[test]
    fn output_switch_starts_from_the_running_signal() {
        for old in [Era::Sparkletop, Era::EarlyI, Era::LateI] {
            let mut reference = Stage73Engine::new(48000.);
            let mut switched = Stage73Engine::new(48000.);
            for e in [&mut reference, &mut switched] {
                e.set_controls(Controls {
                    era: old,
                    ..Default::default()
                });
                e.reset();
                e.note_on(47, 0.7);
                e.render(&mut [0.; 1500]);
            }
            let next = if old == Era::Sparkletop {
                Era::EarlyI
            } else {
                Era::Sparkletop
            };
            switched.set_controls(Controls {
                era: next,
                ..Default::default()
            });
            let (mut steady, mut changed) = ([0.; 64], [0.; 64]);
            reference.render(&mut steady);
            switched.render(&mut changed);
            let level = steady.iter().map(|v| v.abs()).fold(0., f32::max);
            assert!((steady[16] - changed[16]).abs() < 0.035 * level + 1e-7);
        }
    }
}
