//! Electric Pianos shares one host, keyboard and output stage across its physical instruments.

use nih_plug::{midi::control_change, prelude::*};
use openwurli_dsp::WurliEngine;
use std::sync::Arc;

pub mod baldwin;
pub mod claviset;
pub mod duo;
pub mod elepian;
mod hohner;
pub mod mk8;
pub mod models;
pub mod neo;
pub mod params;
pub mod regulation;
pub mod road;
pub mod stereovibe;
pub mod storytone;
pub mod sustain_mod;
pub mod valente;
pub mod vibanet;
pub mod vivi;
pub mod vv64;
use hohner::Action;
use models::Instrument;
use pianet_n::claviset as claviset_dsp;
pub mod ui;

use params::{GrandModel, OpenWurliUiParams, PianoType};
mod output;

// Use upstream v0.9's nominal physical volume-pot position. The wrapper's
// Volume remains a linear output control with its existing 50% default.
const ENGINE_VOLUME: f64 = 0.8;
const OUTPUT_VOLUME_REFERENCE: f32 = 0.5;
// Upstream's plugin-level alignment, applied after the physical signal chain.
const OUTPUT_ALIGNMENT_DB: f32 = 14.0;

pub struct OpenWurliUi {
    params: Arc<OpenWurliUiParams>,
    engine: WurliEngine,
    inactive_wurli_settled: bool,
    finish: FastFinish,
    stage: stage73::engine::Stage73Engine,
    grand: cp80::engine::Engine,
    other_grand: cp80::engine::Engine,
    active_grand: GrandModel,
    original_grand: cp80::engine::Engine,
    using_original: bool,
    active: Instrument,
    d6: clavinet_d6::live::Engine,
    reeds: [pianet_n::Engine; pianet_n::Model::ALL.len()],
    electra: electra_piano::live::Engine,
    duo: hohner_duo::Engine,
    storytone: ::storytone::engine::Engine,
    neo: neo_bechstein::Engine,
    elepian: pianet_n::elepian::Engine,
    valente: pianet_n::valente::Engine,
    vivi: pianet_n::vivi::Engine,
    vv64: vintage_vibe_64::live::Engine,
    stereovibe: vintage_vibe_64::live::StereoEngine,
    road: roadmaster::live::Engine,
    road_board_mix: f64,
    baldwin: baldwin_electropiano::Engine,
    mk8: rhodes_mk8::live::Engine,
    vibanet: clavinet_d6::vibanet::Engine,
    clavisets: [claviset_dsp::Engine; 2],
    expression: Option<f32>,
    bass_pedal: bool,
    soft_pedal: bool,
    pedal: bool,
    transition: output::Transition,
}

/// Optional post-engine sag for OpenWurli.
struct FastFinish {
    sag_envelope: f32,
    sag_attack: f32,
    sag_release: f32,
}

impl FastFinish {
    fn new(sample_rate: f32) -> Self {
        let mut finish = Self {
            sag_envelope: 0.0,
            sag_attack: 0.0,
            sag_release: 0.0,
        };
        finish.set_sample_rate(sample_rate);
        finish
    }

    fn set_sample_rate(&mut self, sample_rate: f32) {
        let sample_rate = sample_rate.max(1.0);
        self.sag_attack = 1.0 - (-1.0 / (0.008 * sample_rate)).exp();
        self.sag_release = 1.0 - (-1.0 / (0.150 * sample_rate)).exp();
    }

    fn reset(&mut self) {
        self.sag_envelope = 0.0;
    }

    fn process(&mut self, output: &mut [f32], sag_enabled: bool) {
        if !sag_enabled {
            self.sag_envelope = 0.0;
            return;
        }
        for sample in output {
            let magnitude = sample.abs();
            let coefficient = if magnitude > self.sag_envelope {
                self.sag_attack
            } else {
                self.sag_release
            };
            self.sag_envelope += coefficient * (magnitude - self.sag_envelope);
            *sample /= 1.0 + 12.0 * self.sag_envelope;
        }
    }
}

impl Default for OpenWurliUi {
    fn default() -> Self {
        Self {
            params: Arc::new(OpenWurliUiParams::default()),
            engine: WurliEngine::new(44_100.0),
            inactive_wurli_settled: false,
            finish: FastFinish::new(44_100.0),
            stage: stage73::engine::Stage73Engine::extended(44100.),
            grand: cp80::engine::Engine::extended(44100.),
            other_grand: cp80::engine::Engine::cp70b(44100., true),
            active_grand: GrandModel::Cp80,
            original_grand: cp80::engine::Engine::cp70(44100., true),
            using_original: false,
            active: Instrument::Piano(PianoType::Wurli),
            d6: clavinet_d6::live::Engine::extended(44100.),
            reeds: Self::reed_engines(44100.),
            electra: Self::electra_engine(44100.),
            duo: hohner_duo::Engine::extended(44100.),
            storytone: Self::storytone_engine(44100.),
            neo: Self::neo_engine(44100.),
            elepian: Self::elepian_engine(44100.),
            valente: Self::valente_engine(44100.),
            vivi: Self::vivi_engine(44100.),
            vv64: Self::vv64_engine(44100.),
            stereovibe: Self::stereovibe_engine(44100.),
            road: Self::road_engine(44100.),
            road_board_mix: 0.,
            baldwin: Self::baldwin_engine(44100.),
            mk8: Self::mk8_engine(44100.),
            vibanet: clavinet_d6::vibanet::Engine::vibanet(44100., true),
            clavisets: Self::claviset_engines(44100.),
            expression: None,
            bass_pedal: false,
            soft_pedal: false,
            pedal: false,
            transition: Default::default(),
        }
    }
}

impl OpenWurliUi {
    fn sync_duo_pedals(&mut self) {
        let (enabled, held) = self.params.sustain_mod.duo.state(self.pedal);
        self.duo.set_sustain_mod(enabled, held);
        self.duo
            .set_controls(self.params.duo.controls(self.pedal && !enabled));
    }
    fn sync_pianet_pedal(&mut self, model: pianet_n::Model) {
        if let Some(config) = self.params.sustain_mod.for_model(Instrument::Reed(model)) {
            let (enabled, held) = config.state(self.pedal);
            self.reeds[model as usize].set_sustain_mod(enabled);
            self.reeds[model as usize].set_sustain(held);
        }
    }
    fn sync_params(&mut self) {
        self.engine.set_volume(ENGINE_VOLUME);
        self.engine
            .set_tremolo_depth(self.params.tremolo_depth.value() as f64);
        self.engine
            .set_speaker_character(self.params.speaker_character.value() as f64);
        // The legacy mlp parameter is hidden and inert, including old saved false values.
        self.engine.set_mlp_enabled(true);
        // Hiss is removed, including native preamp noise in both circuit modes.
        self.engine.set_noise_enabled(false);
        self.engine
            .set_reed_decay(self.params.reed_decay.value() as f64);
        self.engine
            .set_hammer_hardness(self.params.hammer_hardness.value() as f64);
        self.engine
            .set_pickup_drive(self.params.pickup_drive.value() as f64);
        self.engine
            .set_pickup_height_mm(self.params.pickup_height.value() as f64);
        self.engine
            .set_pickup_distance_mm(self.params.regulation.wurli_gap.value().into());
        self.engine
            .set_tremolo_response(self.params.tremolo_response.value() as f64);
    }

    // The wrapper restricts new strikes to the selected compass. Keeping extended
    // geometry prepared lets held keys receive their normal releases after a toggle.
    fn sync_vibanet(&mut self) {
        self.vibanet
            .set_vibanet_controls(self.params.vibanet.controls());
    }
    fn vv64_engine(rate: f64) -> vintage_vibe_64::live::Engine {
        let mut engine = vintage_vibe_64::live::Engine::new(rate);
        engine.set_extended(true);
        engine
    }
    fn sync_vv64(&mut self) {
        self.vv64.set_sustain(self.params.vv64.sustain(self.pedal));
        self.vv64.set_controls(self.params.vv64.controls());
    }

    fn stereovibe_engine(rate: f64) -> vintage_vibe_64::live::StereoEngine {
        let mut engine = vintage_vibe_64::live::StereoEngine::active(rate);
        engine.set_extended(true);
        engine
    }
    fn sync_stereovibe(&mut self) {
        self.stereovibe
            .set_sustain(self.params.stereovibe.sustain(self.pedal));
        self.stereovibe
            .set_controls(self.params.stereovibe.controls());
    }

    fn mk8_engine(rate: f64) -> rhodes_mk8::live::Engine {
        let mut engine = rhodes_mk8::live::Engine::new(rate);
        engine.set_extended(true);
        engine
    }
    fn sync_mk8(&mut self) {
        self.mk8.set_sustain(self.params.mk8.sustain(self.pedal));
        self.mk8.set_controls(self.params.mk8.controls());
        self.mk8.set_fx_controls(self.params.mk8.fx.controls());
    }

    fn road_engine(rate: f64) -> roadmaster::live::Engine {
        let mut engine = roadmaster::live::Engine::new(rate);
        engine.set_extended(true);
        engine
    }
    fn sync_road(&mut self) {
        self.road.set_sustain(self.params.road.sustain(self.pedal));
    }
    fn baldwin_engine(rate: f64) -> baldwin_electropiano::Engine {
        let mut engine = baldwin_electropiano::Engine::new(rate);
        engine.set_extended(true);
        engine
    }
    fn sync_baldwin(&mut self) {
        let (soft, sustain) = self.params.baldwin.pedals(self.soft_pedal, self.pedal);
        self.baldwin
            .set_controls(self.params.baldwin.volume.value().into(), soft, sustain);
    }
    fn elepian_engine(rate: f64) -> pianet_n::elepian::Engine {
        let mut engine = pianet_n::elepian::Engine::new(rate);
        engine.set_extended(true);
        engine
    }

    fn sync_elepian(&mut self) {
        self.elepian
            .set_sustain(self.params.elepian.sustain(self.pedal));
    }

    fn valente_engine(rate: f64) -> pianet_n::valente::Engine {
        let mut engine = pianet_n::valente::Engine::new(rate);
        engine.set_extended(true);
        engine
    }

    fn sync_valente(&mut self) {
        self.valente.set_controls(self.params.valente.controls());
        self.valente
            .set_sustain(self.params.valente.sustain(self.pedal));
    }

    fn vivi_engine(rate: f64) -> pianet_n::vivi::Engine {
        let mut engine = pianet_n::vivi::Engine::new(rate);
        engine.set_extended(true);
        engine
    }

    fn sync_vivi(&mut self) {
        self.vivi
            .set_controls(self.params.vivi.controls(self.expression));
        self.vivi.set_sustain(self.params.vivi.sustain(self.pedal));
        self.vivi
            .set_bass_sustain(self.params.vivi.bass_sustain(self.bass_pedal));
    }

    fn neo_engine(rate: f64) -> neo_bechstein::Engine {
        let mut engine = neo_bechstein::Engine::new(rate);
        engine.keyboard.set_extended(true);
        engine
    }

    fn sync_neo(&mut self) {
        let (felt, sustain) = self.params.neo.pedals(self.pedal);
        self.neo.keyboard.set_felt(felt);
        self.neo.keyboard.set_sustain(sustain);
        self.neo
            .set_controls(self.params.neo.controls(self.expression.unwrap_or(0.)));
    }

    fn storytone_engine(rate: f64) -> ::storytone::engine::Engine {
        let mut engine = ::storytone::engine::Engine::new(rate);
        engine.set_extended(true);
        engine
    }

    fn electra_engine(rate: f64) -> electra_piano::live::Engine {
        let mut engine = electra_piano::live::Engine::new(rate);
        engine.set_extended(true);
        engine
    }

    fn claviset_engines(rate: f64) -> [claviset_dsp::Engine; 2] {
        use claviset_dsp::Instrument::{Claviset200, Pianotron};
        [Claviset200, Pianotron].map(|which| {
            let mut engine = claviset_dsp::Engine::new(which, rate);
            engine.set_extended(true);
            engine
        })
    }

    fn sync_claviset(&mut self, which: claviset_dsp::Instrument) {
        let engine = &mut self.clavisets[which as usize];
        engine.set_controls(self.params.claviset.controls(which, self.expression));
        if which == claviset_dsp::Instrument::Claviset200 {
            engine.set_sustain(self.params.claviset.sustain(self.pedal));
        }
    }

    fn sync_storytone(&mut self) {
        let (soft, sustain) = self.params.storytone.pedals(self.soft_pedal, self.pedal);
        self.storytone.set_soft(soft);
        self.storytone.set_sustain(sustain);
        self.storytone.set_controls(
            self.params
                .storytone
                .controls(self.expression.unwrap_or(0.)),
        );
    }

    fn handle_event(&mut self, event: &NoteEvent<()>) {
        if let NoteEvent::MidiCC { cc, value, .. } = *event {
            match cc {
                control_change::DAMPER_PEDAL => self.pedal = value >= 0.5,
                11 => self.expression = Some(value.clamp(0., 1.)),
                66 => self.bass_pedal = value >= 0.5,
                67 => self.soft_pedal = value >= 0.5,
                control_change::RESET_ALL_CONTROLLERS => {
                    self.pedal = false;
                    self.soft_pedal = false;
                    self.bass_pedal = false;
                    self.expression = None;
                }
                _ => {}
            }
            if matches!(cc, 64 | 121) {
                self.params.sustain_mod.update_midi(self.pedal);
            }
            if matches!(cc, 64 | 67 | 121) {
                self.params.electra_pedals.store(
                    u8::from(self.pedal) | (u8::from(self.soft_pedal) << 1),
                    std::sync::atomic::Ordering::Relaxed,
                );
            }
        }
        if let NoteEvent::NoteOn { note, velocity, .. } = *event {
            if velocity > 0. {
                if !self.params.key_range().contains(&note) {
                    return;
                }
                self.params.keyboard.attack(note);
            }
        }
        match self.active {
            Instrument::Piano(PianoType::Stage) => {
                return stage73::dispatch(&mut self.stage, event)
            }
            Instrument::Piano(PianoType::Grand) => {
                return cp80::plugin::dispatch(&mut self.grand, *event)
            }
            Instrument::Clavinet(_) => {
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.d6.note_on(n, v),
                    Action::Up => self.d6.note_off(n),
                    Action::Choke => self.d6.choke(n),
                })
            }
            Instrument::Vv64 => {
                if matches!(event, NoteEvent::MidiCC { cc: 64 | 121, .. }) {
                    self.vv64.set_sustain(self.params.vv64.sustain(self.pedal));
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.vv64.note_on(n, v),
                    Action::Up => self.vv64.note_off(n),
                    Action::Choke => self.vv64.choke(n),
                });
            }
            Instrument::StereoVibe => {
                if matches!(event, NoteEvent::MidiCC { cc: 64 | 121, .. }) {
                    self.stereovibe
                        .set_sustain(self.params.stereovibe.sustain(self.pedal));
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.stereovibe.note_on(n, v),
                    Action::Up => self.stereovibe.note_off(n),
                    Action::Choke => self.stereovibe.choke(n),
                });
            }
            Instrument::Road => {
                if matches!(event, NoteEvent::MidiCC { cc: 120, .. })
                    || matches!(event, NoteEvent::Choke { note, .. } if *note > 127)
                {
                    self.road.all_sound_off();
                    return;
                }
                if matches!(event, NoteEvent::MidiCC { cc: 64 | 121, .. }) {
                    self.road.set_sustain(self.params.road.sustain(self.pedal));
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.road.note_on(n, v),
                    Action::Up => self.road.note_off(n),
                    Action::Choke => self.road.choke(n),
                });
            }
            Instrument::Baldwin => {
                if matches!(
                    event,
                    NoteEvent::MidiCC {
                        cc: 64 | 67 | 121,
                        ..
                    }
                ) {
                    self.sync_baldwin();
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.baldwin.note_on(n, v),
                    Action::Up => self.baldwin.note_off(n),
                    Action::Choke => self.baldwin.choke(n),
                });
            }
            Instrument::Mk8 => {
                if matches!(event, NoteEvent::MidiCC { cc: 64 | 121, .. }) {
                    self.mk8.set_sustain(self.params.mk8.sustain(self.pedal));
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.mk8.note_on(n, v),
                    Action::Up => self.mk8.note_off(n),
                    Action::Choke => self.mk8.choke(n),
                });
            }
            Instrument::Vibanet => {
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.vibanet.note_on(n, v),
                    Action::Up => self.vibanet.note_off(n),
                    Action::Choke => self.vibanet.choke(n),
                });
            }
            Instrument::Elepian => {
                if matches!(event, NoteEvent::MidiCC { cc: 64 | 121, .. }) {
                    self.elepian
                        .set_sustain(self.params.elepian.sustain(self.pedal));
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.elepian.note_on(n, v),
                    Action::Up => self.elepian.note_off(n),
                    Action::Choke => self.elepian.choke(n),
                });
            }
            Instrument::Valente => {
                if matches!(event, NoteEvent::MidiCC { cc: 64 | 121, .. }) {
                    self.valente
                        .set_sustain(self.params.valente.sustain(self.pedal));
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.valente.note_on(n, v),
                    Action::Up => self.valente.note_off(n),
                    Action::Choke => self.valente.choke(n),
                });
            }
            Instrument::Vivi => {
                if matches!(
                    event,
                    NoteEvent::MidiCC {
                        cc: 11 | 64 | 66 | 121,
                        ..
                    }
                ) {
                    self.sync_vivi();
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.vivi.note_on(n, v),
                    Action::Up => self.vivi.note_off(n),
                    Action::Choke => self.vivi.choke(n),
                });
            }
            Instrument::Neo => {
                if matches!(
                    event,
                    NoteEvent::MidiCC {
                        cc: 11 | 64 | 121,
                        ..
                    }
                ) {
                    self.sync_neo();
                }
                let keyboard = &mut self.neo.keyboard;
                return hohner::dispatch(
                    event,
                    self.params.key_range(),
                    |n, action| match action {
                        Action::Down(v) => keyboard.note_on(n, v),
                        Action::Up => keyboard.note_off(n),
                        Action::Choke => keyboard.choke(n),
                    },
                );
            }
            Instrument::Claviset(which) => {
                if matches!(
                    event,
                    NoteEvent::MidiCC {
                        cc: 11 | 64 | 121,
                        ..
                    }
                ) {
                    self.sync_claviset(which);
                }
                let range = self.params.key_range();
                let engine = &mut self.clavisets[which as usize];
                return hohner::dispatch(event, range, |n, a| match a {
                    Action::Down(v) => engine.note_on(n, v),
                    Action::Up => engine.note_off(n),
                    Action::Choke => engine.choke(n),
                });
            }
            Instrument::Storytone => {
                if matches!(
                    event,
                    NoteEvent::MidiCC {
                        cc: 11 | 64 | 67 | 121,
                        ..
                    }
                ) {
                    self.sync_storytone();
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.storytone.note_on(n, v),
                    Action::Up => self.storytone.note_off(n),
                    Action::Choke => self.storytone.choke(n),
                });
            }
            Instrument::Duo => {
                if matches!(event, NoteEvent::MidiCC { cc: 64 | 121, .. }) {
                    self.sync_duo_pedals();
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.duo.note_on(n, v),
                    Action::Up => self.duo.note_off(n),
                    Action::Choke => self.duo.choke(n),
                });
            }
            Instrument::Electra => {
                if matches!(event, NoteEvent::MidiCC { .. }) {
                    self.electra
                        .set_sustain(self.pedal || self.params.electra_sustain.value());
                    self.electra
                        .set_controls(self.params.electra_controls(self.soft_pedal));
                }
                return hohner::dispatch(event, self.params.key_range(), |n, a| match a {
                    Action::Down(v) => self.electra.note_on(n, v),
                    Action::Up => self.electra.note_off(n),
                    Action::Choke => self.electra.choke(n),
                });
            }
            Instrument::Reed(m) => {
                if matches!(event, NoteEvent::MidiCC { cc: 64 | 121, .. }) {
                    self.sync_pianet_pedal(m);
                }

                let range = self.params.key_range();
                let engine = &mut self.reeds[m as usize];
                return hohner::dispatch(event, range, |n, a| match a {
                    Action::Down(v) => engine.note_on(n, v),
                    Action::Up => engine.note_off(n),
                    Action::Choke => engine.choke(n),
                });
            }
            Instrument::Piano(PianoType::Wurli) => {}
        }
        match event {
            // CLAP's wildcard key (-1) arrives as 255 through NIH-plug.
            NoteEvent::NoteOn { note, .. } if *note > 127 => {}
            NoteEvent::NoteOff { note, .. } if *note > 127 => {
                self.engine.release_all_keys();
            }
            NoteEvent::NoteOn { note, velocity, .. } if *velocity == 0.0 => {
                self.engine.note_off_extended(*note)
            }
            NoteEvent::NoteOn { note, velocity, .. } if self.params.key_range().contains(note) => {
                self.engine.note_on_extended(*note, *velocity)
            }
            // Always release the exact pitch, even if Extended Notes was
            // disabled while it was held. Ignored pitches cannot release an end key.
            NoteEvent::NoteOff { note, .. } => self.engine.note_off_extended(*note),
            NoteEvent::Choke { note, .. } if *note > 127 => self.engine.all_sound_off(),
            NoteEvent::Choke { note, .. } => self.engine.choke_extended(*note),
            NoteEvent::MidiCC { cc, .. } if *cc == control_change::ALL_SOUND_OFF => {
                self.engine.all_sound_off();
            }
            NoteEvent::MidiCC { cc, .. }
                if *cc == control_change::ALL_NOTES_OFF || (124..=127).contains(cc) =>
            {
                self.engine.release_all_keys();
            }
            NoteEvent::MidiCC { cc, .. } if *cc == control_change::RESET_ALL_CONTROLLERS => {
                self.engine.set_sustain(false);
            }
            NoteEvent::MidiCC { cc, value, .. } if *cc == control_change::DAMPER_PEDAL => {
                self.engine.set_sustain(*value >= 0.5)
            }
            _ => {}
        }
    }

    fn reed_engines(rate: f64) -> [pianet_n::Engine; pianet_n::Model::ALL.len()] {
        pianet_n::Model::ALL.map(|m| {
            let mut e = pianet_n::Engine::new(m, rate);
            e.set_extended(true);
            e.set_early_output_gains(1., output::early_local_gain(m));
            e
        })
    }

    fn select(&mut self, next: Instrument) {
        let grand_model = self.params.grand_model.value();
        let original = self.params.original_cp70();
        let swap_grands = next == Instrument::Piano(PianoType::Grand)
            && (grand_model != self.active_grand || original != self.using_original);
        if self.active == next && !swap_grands {
            return;
        }
        match self.active {
            Instrument::Piano(PianoType::Wurli) => {
                self.engine.clear_keys();
                self.inactive_wurli_settled = false;
                self.finish.reset();
            }
            Instrument::Piano(PianoType::Stage) => self.stage.reset(),
            Instrument::Piano(PianoType::Grand) => self.grand.reset(),
            Instrument::Electra => self.electra.reset(),
            Instrument::Duo => self.duo.reset(),
            Instrument::Storytone => self.storytone.reset(),
            Instrument::Neo => self.neo.reset(),
            Instrument::Elepian => self.elepian.reset(),
            Instrument::Valente => self.valente.reset(),
            Instrument::Vivi => self.vivi.reset(),
            Instrument::Vv64 => self.vv64.reset(),
            Instrument::StereoVibe => self.stereovibe.reset(),
            Instrument::Road => self.road.reset(),
            Instrument::Baldwin => self.baldwin.reset(),
            Instrument::Mk8 => self.mk8.reset(),
            Instrument::Vibanet => self.vibanet.reset(),
            Instrument::Claviset(which) => self.clavisets[which as usize].reset(),
            Instrument::Clavinet(_) => self.d6.reset(),
            Instrument::Reed(m) => self.reeds[m as usize].reset(),
        }
        if swap_grands {
            if self.using_original {
                std::mem::swap(&mut self.grand, &mut self.original_grand);
            }
            if grand_model != self.active_grand {
                std::mem::swap(&mut self.grand, &mut self.other_grand);
            }
            if original {
                std::mem::swap(&mut self.grand, &mut self.original_grand);
            }
            self.active_grand = grand_model;
            self.using_original = original;
        }
        self.active = next;
        match next {
            Instrument::Piano(PianoType::Wurli) => self.engine.set_sustain(self.pedal),
            Instrument::Piano(PianoType::Stage) => self.stage.sustain(self.pedal),
            Instrument::Piano(PianoType::Grand) => self.grand.sustain(self.pedal),
            Instrument::Storytone => self.sync_storytone(),
            Instrument::Neo => self.sync_neo(),
            Instrument::Elepian => self.sync_elepian(),
            Instrument::Valente => self.sync_valente(),
            Instrument::Vivi => self.sync_vivi(),
            Instrument::Vv64 => self.sync_vv64(),
            Instrument::StereoVibe => self.sync_stereovibe(),
            Instrument::Road => self.sync_road(),
            Instrument::Baldwin => self.sync_baldwin(),
            Instrument::Mk8 => self.sync_mk8(),
            Instrument::Vibanet => self.sync_vibanet(),
            Instrument::Claviset(which) => self.sync_claviset(which),
            Instrument::Electra => self
                .electra
                .set_sustain(self.pedal || self.params.electra_sustain.value()),
            Instrument::Clavinet(_) | Instrument::Reed(_) | Instrument::Duo => {}
        }
        self.transition.start();
    }

    fn render_stereo(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        mut next: impl FnMut() -> Option<NoteEvent<()>>,
    ) {
        let model = self.params.instrument();
        let changed = self.active != model;
        self.select(model);
        let params = self.params.clone();
        let pickups = regulation::PickupControls::new(&params, model);
        if changed {
            pickups.snap();
        }
        self.set_pickups(pickups.frame(false));
        let range = self.params.key_range();
        match self.active {
            Instrument::Piano(PianoType::Wurli) => {
                self.sync_params();
                self.params
                    .keyboard
                    .drain_auditions(range, |n, v| self.engine.audition(n, v));
            }
            Instrument::Piano(PianoType::Stage) => {
                self.stage.set_controls(self.params.stage_controls());
                self.params
                    .keyboard
                    .drain_auditions(range, |n, v| self.stage.audition(n, v));
            }
            Instrument::Electra => {
                self.electra
                    .set_controls(self.params.electra_controls(self.soft_pedal));
                self.electra
                    .set_sustain(self.pedal || self.params.electra_sustain.value());
                self.params.keyboard.drain_auditions(range, |n, v| match n {
                    Some(n) => self.electra.audition(n, v),
                    None => self.electra.audition_off(),
                });
            }
            Instrument::Claviset(which) => {
                self.sync_claviset(which);
                let engine = &mut self.clavisets[which as usize];
                self.params
                    .keyboard
                    .drain_auditions(range, |n, v| engine.audition(n, v));
            }
            Instrument::Vv64 => {
                self.sync_vv64();
                self.params.keyboard.drain_auditions(range, |n, v| match n {
                    Some(note) => self.vv64.audition(note, v),
                    None => self.vv64.audition_off(),
                });
            }
            Instrument::StereoVibe => {
                self.sync_stereovibe();
                self.params.keyboard.drain_auditions(range, |n, v| match n {
                    Some(note) => self.stereovibe.audition(note, v),
                    None => self.stereovibe.audition_off(),
                });
            }
            Instrument::Road => {
                self.sync_road();
                self.params
                    .keyboard
                    .drain_auditions(range, |n, v| self.road.audition(n, v));
            }
            Instrument::Baldwin => {
                self.sync_baldwin();
                self.params.keyboard.drain_auditions(range, |n, v| match n {
                    Some(note) => self.baldwin.audition(note, v),
                    None => self.baldwin.audition_off(),
                });
            }
            Instrument::Mk8 => {
                self.sync_mk8();
                self.params.keyboard.drain_auditions(range, |n, v| match n {
                    Some(note) => self.mk8.audition(note, v),
                    None => self.mk8.audition_off(),
                });
            }
            Instrument::Vibanet => {
                self.sync_vibanet();
                self.params.keyboard.drain_auditions(range, |n, v| match n {
                    Some(note) => self.vibanet.audition(note, v),
                    None => self.vibanet.audition_off(),
                });
            }
            Instrument::Elepian => {
                self.sync_elepian();
                self.params
                    .keyboard
                    .drain_auditions(range, |n, v| self.elepian.audition(n, v));
            }
            Instrument::Valente => {
                self.sync_valente();
                self.params
                    .keyboard
                    .drain_auditions(range, |n, v| self.valente.audition(n, v));
            }
            Instrument::Vivi => {
                self.sync_vivi();
                self.params
                    .keyboard
                    .drain_auditions(range, |n, v| self.vivi.audition(n, v));
            }
            Instrument::Neo => {
                self.sync_neo();
                let keyboard = &mut self.neo.keyboard;
                self.params.keyboard.drain_auditions(range, |n, v| match n {
                    Some(n) => keyboard.audition(n, v),
                    None => keyboard.audition_off(),
                });
            }
            Instrument::Storytone => {
                self.sync_storytone();
                self.params.keyboard.drain_auditions(range, |n, v| match n {
                    Some(n) => self.storytone.audition(n, v),
                    None => self.storytone.audition_off(),
                });
            }
            Instrument::Duo => {
                self.sync_duo_pedals();
                self.params
                    .keyboard
                    .drain_auditions(range, |n, v| self.duo.audition(n, v));
            }
            Instrument::Clavinet(m) => {
                use clavinet_d6::panel::Board;
                self.d6.set_board(match m {
                    models::ClavinetModel::D6 => Board::D6,
                    models::ClavinetModel::E7 => Board::E7,
                });
                self.d6.set_power(self.params.clavinet_e7_power.value());
                let p = &self.params.d6;
                let wiring = if p.invert_da.value() {
                    clavinet_d6::Wiring::DaSubtracts
                } else {
                    clavinet_d6::Wiring::DaAdds
                };
                self.d6.set_controls(
                    p.pickup.value().selector(),
                    wiring,
                    p.mask(),
                    self.params.clavinet_volume().value().into(),
                    if p.mute.value() { 1. } else { 0. },
                );
                self.params
                    .keyboard
                    .drain_auditions(range, |note, vel| match note {
                        Some(n) => self.d6.audition(n, vel),
                        None => self.d6.audition_off(),
                    });
            }
            Instrument::Reed(m) => {
                self.sync_pianet_pedal(m);
                let engine = &mut self.reeds[m as usize];
                use pianet_n::{early::Monitor, Model};
                if matches!(m, Model::C | Model::L) {
                    let monitor = match (m, self.params.early_local_output.value()) {
                        (Model::C, true) => Monitor::Phones,
                        (Model::L, true) => Monitor::Speakers,
                        _ => Monitor::External,
                    };
                    engine.set_early_monitor(monitor);
                }
                if m != pianet_n::Model::T {
                    let (swell, vibrato) = self.params.reed_controls();
                    engine.set_n_controls(swell.value(), vibrato.value());
                }
                self.params
                    .keyboard
                    .drain_auditions(range, |n, v| engine.audition(n, v));
            }
            Instrument::Piano(PianoType::Grand) => {
                self.grand.set(self.params.grand_controls());
                self.params
                    .keyboard
                    .drain_auditions(range, |n, v| self.grand.audition(n, v));
            }
        }
        let mut pos = 0;
        loop {
            let event = next();
            let end = event
                .as_ref()
                .map_or(left.len(), |e| (e.timing() as usize).clamp(pos, left.len()));
            let (l, r) = (&mut left[pos..end], &mut right[pos..end]);
            let mut moving = 0;
            while moving < l.len() && pickups.is_moving() {
                self.set_pickups(pickups.frame(true));
                self.render_audio(&mut l[moving..=moving], &mut r[moving..=moving]);
                moving += 1;
            }
            if moving < l.len() {
                self.render_audio(&mut l[moving..], &mut r[moving..]);
            }
            if let Some(event) = event {
                self.handle_event(&event);
            } else {
                break;
            }
            pos = end;
        }
        let gain = output::trim(self.active, self.active_grand, self.using_original)
            / OUTPUT_VOLUME_REFERENCE;
        for (l, r) in left.iter_mut().zip(right) {
            let frame = self.transition.apply([*l * gain, *r * gain]);
            let volume = self.params.volume.smoothed.next();
            *l = frame[0] * volume;
            *r = frame[1] * volume;
        }
        self.params.keyboard.publish(|| {
            let pads = match self.active {
                Instrument::Reed(m) => self.reeds[m as usize].caught_pads(),
                Instrument::Duo => self.duo.caught_pads(),
                _ => 0,
            };
            self.params.sustain_mod.publish_catches(pads);
            self.held_notes()
        });
    }

    fn set_pickups(&mut self, [gap, bridge, reed, pad, alignment]: [f32; 5]) {
        match self.active {
            Instrument::Piano(_) => {}
            Instrument::Clavinet(_) => self.d6.set_pickup_distance_mm(gap.into(), bridge.into()),
            Instrument::Claviset(which) => {
                self.clavisets[which as usize].set_pickup_distance_mm(gap)
            }
            Instrument::Electra => self.electra.set_pickup_gap_mm(gap.into()),
            Instrument::Duo => {
                self.duo
                    .set_pickup_distance_mm(gap.into(), bridge.into(), reed.into());
                self.duo.set_pad_release_noise(pad.into());
            }
            Instrument::Reed(m) => {
                self.reeds[m as usize].set_pickup_distance_mm(gap);
                if !m.is_cembalet() {
                    self.reeds[m as usize].set_pad_release(pad);
                }
            }
            Instrument::Storytone => self.storytone.set_pickup_gap_mm(gap.into()),
            Instrument::Neo => self.neo.keyboard.set_pickup_gap_mm(gap.into()),
            Instrument::Elepian => {
                self.elepian.set_pickup_distance_mm(gap);
                self.elepian.set_controls(pianet_n::elepian::Controls {
                    alignment_mm: alignment.into(),
                    ..self.params.elepian.controls()
                });
            }
            Instrument::Vv64 => self.vv64.set_pickup_gap_mm(gap.into()),
            Instrument::StereoVibe => self.stereovibe.set_pickup_gap_mm(gap.into()),
            Instrument::Road => {
                self.road
                    .set_pickup_distance_mm([gap.into(), bridge.into(), reed.into()])
            }
            Instrument::Baldwin => self.baldwin.set_pickup_position_mm(gap.into()),
            Instrument::Mk8 => self.mk8.set_pickup_gap_mm(gap.into()),
            Instrument::Vibanet => self
                .vibanet
                .set_pickup_distance_mm(gap.into(), bridge.into()),
            Instrument::Valente => self.valente.set_pickup_distance_mm(gap),
            Instrument::Vivi => self.vivi.set_pickup_distance_mm(gap),
        }
    }

    fn render_audio(&mut self, l: &mut [f32], r: &mut [f32]) {
        match self.active {
            Instrument::Piano(PianoType::Wurli) => {
                self.engine.render(l);
                self.finish_output(l);
                r.copy_from_slice(l);
            }
            Instrument::Piano(PianoType::Stage) => {
                self.stage.render_stereo(l, r);
            }
            Instrument::Duo => {
                if self.params.duo.separate_outputs() {
                    for (left, right) in l.iter_mut().zip(r) {
                        let [clavinet, pianet, _] = self.duo.jacks();
                        *left = clavinet as f32;
                        *right = pianet as f32;
                    }
                } else {
                    self.duo.render(l);
                    r.copy_from_slice(l);
                }
            }
            Instrument::Claviset(which) => {
                self.clavisets[which as usize].render(l);
                r.copy_from_slice(l);
            }
            Instrument::Vv64 => {
                self.vv64.render(l);
                r.copy_from_slice(l);
            }
            Instrument::StereoVibe => {
                self.stereovibe.render_stereo(l, r);
            }
            Instrument::Road => {
                let target = f64::from(self.params.road.board.value());
                for y in l.iter_mut() {
                    let o = self.road.next_outputs();
                    self.road_board_mix +=
                        (target - self.road_board_mix).clamp(-1. / 128., 1. / 128.);
                    let board = o.board_m_s * 0.5127;
                    *y = (o.jack_v + self.road_board_mix * (board - o.jack_v)) as f32;
                }
                r.copy_from_slice(l);
            }
            Instrument::Baldwin => {
                self.baldwin.render(l);
                r.copy_from_slice(l);
            }
            Instrument::Mk8 => {
                self.mk8.render_stereo(l, r);
            }
            Instrument::Vibanet => {
                self.vibanet.render(l);
                r.copy_from_slice(l);
            }
            Instrument::Elepian => {
                self.elepian.render(l);
                r.copy_from_slice(l);
            }
            Instrument::Valente => {
                self.valente.render(l);
                r.copy_from_slice(l);
            }
            Instrument::Vivi => {
                self.vivi.render(l);
                r.copy_from_slice(l);
            }
            Instrument::Neo => {
                self.neo.render(l);
                r.copy_from_slice(l);
            }
            Instrument::Storytone => {
                self.storytone.render(l);
                r.copy_from_slice(l);
            }
            Instrument::Electra => {
                self.electra.render(l);
                r.copy_from_slice(l);
            }
            Instrument::Clavinet(_) => {
                self.d6.render(l);
                r.copy_from_slice(l);
            }
            Instrument::Reed(m) => {
                self.reeds[m as usize].render(l);
                r.copy_from_slice(l);
            }
            Instrument::Piano(PianoType::Grand) => {
                for (l, r) in l.iter_mut().zip(r) {
                    [*l, *r] = self.grand.sample();
                }
            }
        }
    }

    fn held_notes(&self) -> u128 {
        match self.active {
            Instrument::Piano(PianoType::Wurli) => self.engine.playing_notes(),
            Instrument::Piano(PianoType::Stage) => self.stage.playing_notes(),
            Instrument::Piano(PianoType::Grand) => self.grand.pressed(),
            Instrument::Clavinet(_) => self.d6.held_notes(),
            Instrument::Reed(m) => self.reeds[m as usize].pressed(),
            Instrument::Electra => self.electra.held_notes(),
            Instrument::Duo => self.duo.pressed(),
            Instrument::Storytone => self.storytone.held_notes(),
            Instrument::Neo => self.neo.keyboard.held_notes(),
            Instrument::Elepian => self.elepian.pressed(),
            Instrument::Valente => self.valente.pressed(),
            Instrument::Vivi => self.vivi.pressed(),
            Instrument::Vv64 => self.vv64.held_notes(),
            Instrument::StereoVibe => self.stereovibe.held_notes(),
            Instrument::Road => self.road.pressed(),
            Instrument::Baldwin => self.baldwin.held_notes(),
            Instrument::Mk8 => self.mk8.held_notes(),
            Instrument::Vibanet => self.vibanet.held_notes(),
            Instrument::Claviset(which) => self.clavisets[which as usize].pressed(),
        }
    }

    fn finish_output(&mut self, output: &mut [f32]) {
        let alignment = util::db_to_gain(OUTPUT_ALIGNMENT_DB);
        for s in output.iter_mut() {
            *s *= alignment;
        }
        self.finish.process(output, self.params.rail_sag.value());
    }

    fn latency(&self) -> u32 {
        match self.params.instrument() {
            Instrument::Piano(PianoType::Wurli) | Instrument::Electra | Instrument::Storytone => 0,
            Instrument::Clavinet(_) => clavinet_d6::live::LATENCY,
            Instrument::Reed(_) => pianet_n::LATENCY,
            Instrument::Duo => hohner_duo::LATENCY,
            Instrument::Neo => neo_bechstein::amp::LATENCY,
            Instrument::Elepian => pianet_n::elepian::LATENCY,
            Instrument::Valente => pianet_n::valente::LATENCY,
            Instrument::Vivi => pianet_n::vivi::LATENCY,
            Instrument::Vv64 => vintage_vibe_64::live::LATENCY as u32,
            Instrument::StereoVibe => vintage_vibe_64::live::LATENCY as u32,
            Instrument::Road => roadmaster::LATENCY as u32,
            Instrument::Baldwin => baldwin_electropiano::LATENCY,
            Instrument::Mk8 => rhodes_mk8::live::LATENCY as u32,
            Instrument::Vibanet => clavinet_d6::live::LATENCY,
            Instrument::Claviset(_) => claviset_dsp::LATENCY,
            Instrument::Piano(PianoType::Stage) => stage73::engine::LATENCY,
            Instrument::Piano(PianoType::Grand) => cp80::engine::LATENCY,
        }
    }
}

impl Plugin for OpenWurliUi {
    const NAME: &'static str = "Electric Pianos";
    const VENDOR: &'static str = "Fergler";
    const URL: &'static str = "https://github.com/Ferglerz/rust-audio-plugins";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn filter_state(state: &mut PluginState) {
        use nih_plug::wrapper::state::ParamValue;
        state.params.remove("cpu_mode");
        regulation::restore_missing(state);
        sustain_mod::Settings::restore_missing(state);
        // A preset saved before the new output paths can be loaded over an
        // instance with either path enabled.
        for (id, value) in [
            ("stage_peterson", ParamValue::Bool(false)),
            ("stage_peterson_hz", ParamValue::F32(5.)),
            ("stage_peterson_depth", ParamValue::F32(0.7)),
            ("model_bank", ParamValue::String("original".into())),
            ("electra_enable", ParamValue::Bool(false)),
            ("claviset_model", ParamValue::String("off".into())),
            ("vv64_enable", ParamValue::Bool(false)),
            ("stereovibe_enable", ParamValue::Bool(false)),
            ("road_enable", ParamValue::Bool(false)),
            ("baldwin_enable", ParamValue::Bool(false)),
            ("mk8_enable", ParamValue::Bool(false)),
            ("mk8_fx_fitted", ParamValue::Bool(false)),
            ("vibanet_enable", ParamValue::Bool(false)),
            ("elepian_enable", ParamValue::Bool(false)),
            ("valente_enable", ParamValue::Bool(false)),
            ("vivi_enable", ParamValue::Bool(false)),
            ("neo_enable", ParamValue::Bool(false)),
            ("storytone_enable", ParamValue::Bool(false)),
            ("duo_enable", ParamValue::Bool(false)),
            ("clavinet_model", ParamValue::String("d6".into())),
            ("early_reed_model", ParamValue::String("pianet_n".into())),
            ("pianet_revision", ParamValue::String("n".into())),
            ("clavinet_e7_power", ParamValue::Bool(true)),
            ("grand_model", ParamValue::String("cp80".into())),
            ("grand_70_original", ParamValue::Bool(false)),
            ("grand_frame_ring", ParamValue::Bool(false)),
            ("stage_panel_vol", ParamValue::F32(1.)),
            (
                "stage_cable_pf",
                ParamValue::F32(stage73::circuit::DEFAULT_CABLE_PF as f32),
            ),
            ("grand_jack_vol", ParamValue::F32(1.)),
        ] {
            state.params.entry(id.into()).or_insert(value);
        }
        state
            .params
            .entry("piano_type".into())
            .or_insert_with(|| ParamValue::String("wurli".into()));
        // NIH-plug restores plain floats without clamping them to the range.
        if let Some(nih_plug::wrapper::state::ParamValue::F32(drive)) =
            state.params.get_mut("pickup_drive")
        {
            *drive = drive.clamp(0.5, params::PICKUP_DRIVE_MAX);
        }
        // A previous preset can be restored over an already voiced instance.
        let height = state
            .params
            .entry("pickup_height".into())
            .or_insert(ParamValue::F32(0.0));
        let limit = openwurli_dsp::pickup::MAX_HEIGHT_TRIM_MM as f32;
        *height = match height {
            ParamValue::F32(mm) if mm.is_finite() => ParamValue::F32(mm.clamp(-limit, limit)),
            _ => ParamValue::F32(0.0),
        };
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let rate = buffer_config.sample_rate;
        if !(22050.0..=384000.0).contains(&rate) {
            return false;
        }
        self.stage = stage73::engine::Stage73Engine::extended(f64::from(rate));
        self.grand = cp80::engine::Engine::extended(f64::from(rate));
        self.other_grand = cp80::engine::Engine::cp70b(f64::from(rate), true);
        self.active_grand = GrandModel::Cp80;
        self.original_grand = cp80::engine::Engine::cp70(f64::from(rate), true);
        self.using_original = false;
        self.d6 = clavinet_d6::live::Engine::extended(f64::from(rate));
        self.reeds = Self::reed_engines(rate.into());
        self.electra = Self::electra_engine(rate.into());
        self.duo = hohner_duo::Engine::extended(rate.into());
        self.storytone = Self::storytone_engine(rate.into());
        self.neo = Self::neo_engine(rate.into());
        self.elepian = Self::elepian_engine(rate.into());
        self.valente = Self::valente_engine(rate.into());
        self.vivi = Self::vivi_engine(rate.into());
        self.vv64 = Self::vv64_engine(rate.into());
        self.stereovibe = Self::stereovibe_engine(rate.into());
        self.road = Self::road_engine(rate.into());
        self.baldwin = Self::baldwin_engine(rate.into());
        self.mk8 = Self::mk8_engine(rate.into());
        self.vibanet = clavinet_d6::vibanet::Engine::vibanet(rate.into(), true);
        self.clavisets = Self::claviset_engines(rate.into());
        self.active = self.params.instrument();
        self.select(self.active);
        self.stage.set_controls(self.params.stage_controls());
        self.grand.set(self.params.grand_controls());
        _context.set_latency_samples(self.latency());
        self.inactive_wurli_settled = false;
        self.engine
            .set_sample_rate(buffer_config.sample_rate as f64);
        self.finish.set_sample_rate(buffer_config.sample_rate);
        self.engine
            .ensure_buffer_capacity(buffer_config.max_buffer_size as usize);
        self.sync_params();
        true
    }

    fn reset(&mut self) {
        let wurli = Instrument::Piano(PianoType::Wurli);
        let inactive = self.active != wurli && self.params.instrument() != wurli;
        // An inactive Wurli receives neither audio nor controls. Its first reset
        // clears the previous performance and leaves the analog circuits settled.
        if !inactive || !self.inactive_wurli_settled {
            self.engine.reset();
            self.inactive_wurli_settled = inactive;
        }
        self.stage.reset();
        self.grand.reset();
        self.other_grand.reset();
        self.original_grand.reset();
        self.d6.reset();
        self.electra.reset();
        self.duo.reset();
        self.storytone.reset();
        self.neo.reset();
        self.elepian.reset();
        self.valente.reset();
        self.vivi.reset();
        self.vv64.reset();
        self.stereovibe.reset();
        self.road.reset();
        self.road_board_mix = f64::from(self.params.road.board.value());
        self.baldwin.reset();
        self.mk8.reset();
        self.vibanet.reset();
        for engine in &mut self.clavisets {
            engine.reset();
        }
        self.expression = None;
        self.soft_pedal = false;
        self.bass_pedal = false;
        self.params
            .electra_pedals
            .store(0, std::sync::atomic::Ordering::Relaxed);
        for engine in &mut self.reeds {
            engine.reset();
        }
        self.transition = Default::default();
        self.pedal = false;
        self.params.sustain_mod.update_midi(false);
        self.params.sustain_mod.publish_catches(0);
        self.finish.reset();
        self.params.keyboard.publish(|| 0);
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.clone())
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if self.active != self.params.instrument() {
            context.set_latency_samples(self.latency());
        }
        let (left, right) = buffer.as_slice().split_at_mut(1);
        self.render_stereo(left[0], right[0], || context.next_event());
        match self.active {
            Instrument::Piano(PianoType::Grand) if self.grand.sounding() => {
                ProcessStatus::KeepAlive
            }
            Instrument::Piano(PianoType::Grand) => ProcessStatus::Tail(16384),
            Instrument::Piano(PianoType::Stage) if self.stage.is_ringing() => {
                ProcessStatus::KeepAlive
            }
            Instrument::Claviset(which) if self.clavisets[which as usize].sounding() => {
                ProcessStatus::KeepAlive
            }
            Instrument::Vv64 if self.vv64.sounding() => ProcessStatus::KeepAlive,
            Instrument::StereoVibe if self.stereovibe.sounding() => ProcessStatus::KeepAlive,
            Instrument::Road if self.road.sounding() => ProcessStatus::KeepAlive,
            Instrument::Baldwin if self.baldwin.sounding() => ProcessStatus::KeepAlive,
            Instrument::Mk8 if self.mk8.sounding() => ProcessStatus::KeepAlive,
            Instrument::Vibanet if self.vibanet.sounding() => ProcessStatus::KeepAlive,
            Instrument::Elepian if self.elepian.sounding() => ProcessStatus::KeepAlive,
            Instrument::Valente if self.valente.sounding() => ProcessStatus::KeepAlive,
            Instrument::Vivi if self.vivi.sounding() => ProcessStatus::KeepAlive,
            Instrument::Neo if self.neo.sounding() => ProcessStatus::KeepAlive,
            Instrument::Storytone if self.storytone.sounding() => ProcessStatus::KeepAlive,
            Instrument::Duo if self.duo.sounding() => ProcessStatus::KeepAlive,
            Instrument::Electra if self.electra.sounding() => ProcessStatus::KeepAlive,
            Instrument::Clavinet(_) if self.d6.sounding() => ProcessStatus::KeepAlive,
            Instrument::Reed(m) if self.reeds[m as usize].sounding() => ProcessStatus::KeepAlive,
            _ => ProcessStatus::Normal,
        }
    }
}

impl ClapPlugin for OpenWurliUi {
    const CLAP_ID: &'static str = "com.fergler.openwurli-ui";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("OpenWurli, Stage 73, Electric Grand, Clavinet, Pianet and Cembalet");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Custom("electric-piano"),
    ];
}

impl Vst3Plugin for OpenWurliUi {
    const VST3_CLASS_ID: [u8; 16] = *b"FergOpenWurliUI_";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(OpenWurliUi);
// Upstream's Rust 1.99 compatibility allowance for the framework's VST3
// export macro; keep it scoped to the export rather than the whole crate.
#[allow(unknown_lints, semicolon_in_expressions_from_non_local_macros)]
mod vst3_export {
    use super::*;
    nih_export_vst3!(OpenWurliUi);
}

#[cfg(test)]
mod tests;
