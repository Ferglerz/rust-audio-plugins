mod baldwin;
mod claviset;
mod controls;
mod duo;
mod elepian;
mod hohner;
mod mk8;
mod road;
mod stereovibe;
mod storytone;
mod valente;
mod vibanet;
mod vivi;
mod vv64;
use crate::models::{ClavinetModel, Instrument, ModelBank, Pickup};
mod neo;
mod reed_action;
mod regulation;
mod slide;

use crate::params::{GrandModel, OpenWurliUiParams, PianoType};
use controls::{visible, Knob};
use cp80::{
    plugin::Brilliance,
    ui::string_action::{Part as StringPart, StringAction},
};
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{prelude::*, vg::FontId},
    ViziaTheming,
};
use pleasant_ui::{
    draw::{Draw, EditorViewport, TOOLTIP_HEIGHT},
    instrument_action::StrikePreview,
    load_font_once, param,
    piano::Piano,
    playable_keyboard::{self, PlayableKeyboard},
    point_in_rect, start_frame_timer,
    theme::{BG, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    FONT_JETBRAINS_MONO,
};
use slide::Slide;
use stage73::{
    era::Era,
    ui::voicing::{Part as TinePart, Voicing},
};
use std::{cell::Cell, rc::Rc, sync::Arc, time::Instant};

pleasant_ui::appearance_store!("Electric Pianos");
type Rect = (f32, f32, f32, f32);
pub(crate) const WIDTH: f32 = 728.;
pub(crate) const HEIGHT: f32 = 588.;
const SIZE: (f32, f32) = (WIDTH, HEIGHT);
const KEYS: Rect = (30., 470., 668., 65.);
const EXTENDED: Rect = (385., 425., 178., 32.);
const THEME: Rect = (608., 425., 88., 32.);
const SAG: Rect = (512., 324., 165., 32.);
const ORIGINAL_CP70: Rect = (186., 272., 74., 28.);
const GRAND_FRAME: Rect = (274., 272., 158., 28.);
const MODEL_SWITCH: Rect = (197., 272., 193., 28.);
const TREMOLO: Rect = (504., 272., 176., 28.);
const ERA: Rect = (32., 325., 147., 32.);
const MODEL_LABELS: [&str; 3] = ["OpenWurli", "Stage 73", "Electric Grand"];
const BRILLIANCE: [Brilliance; 3] = [Brilliance::Low, Brilliance::Medium, Brilliance::High];

fn tab(i: usize) -> Rect {
    (224. + i as f32 * 116., 10., 112., 32.)
}
fn era_row(i: usize) -> Rect {
    (ERA.0, ERA.1 + ERA.3 + 4. + i as f32 * 26., ERA.2, 26.)
}
fn grand_model_button(model: GrandModel) -> Rect {
    (30. + 78. * model.to_index() as f32, 272., 74., 28.)
}
fn brilliance(i: usize) -> Rect {
    (310. + i as f32 * 46., 336., 44., 28.)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Dial(Knob),
    Reed(Knob),
    Tine(TinePart),
    String(StringPart),
    Piano(usize),
    HohnerMenu,
    HohnerChoice(Instrument),
    HohnerKey,
    Pickup(Pickup),
    ToneTab(usize),
    D6Mute,
    E7Power,
    E7Volume,
    CembaletDamper,
    CembaletPlectrum,
    CembaletElectrode,
    ElectraFork,
    ElectraPickup,
    ElectraSoft,
    ElectraSustain,
    D6Phase,
    PianetVibrato,
    SustainMod,
    ModPedal,
    EarlyLocalOut,
    Duo(duo::Part),
    Storytone(storytone::Part),
    Neo(neo::Part),
    Elepian(elepian::Part),
    Vv64(vv64::Part),
    StereoVibe(stereovibe::Part),
    Road(road::Part),
    Baldwin(baldwin::Part),
    Mk8(mk8::Part),
    Vibanet(vibanet::Part),
    Valente(valente::Part),
    Vivi(vivi::Part),
    Claviset(claviset::Part),
    Era,
    EraChoice(usize),
    GrandModel(GrandModel),
    OriginalCp70,
    Brilliance(usize),
    PickupMenu,
    PickupSetting(Instrument, crate::regulation::Adjustment),
    Extended,
    Sag,
    Tremolo,
    Peterson,
    FrameRing,
    Theme,
    Keyboard,
}
impl Target {
    fn knob(self) -> Option<Knob> {
        match self {
            Self::Dial(k) | Self::Reed(k) => Some(k),
            Self::E7Volume => Some(Knob::D6Volume),
            Self::PickupSetting(m, k) => Some(Knob::PickupSetting(m, k)),
            Self::Duo(part) => part.level(),
            Self::Storytone(part) => part.level(),
            Self::Neo(part) => part.level(),
            Self::Elepian(part) => part.level(),
            Self::Vv64(part) => part.level(),
            Self::StereoVibe(part) => part.level(),
            Self::Baldwin(part) => part.level(),
            Self::Mk8(part) => part.level(),
            Self::Vibanet(part) => part.level(),
            Self::Valente(part) => part.level(),
            Self::Vivi(part) => part.level(),
            Self::Claviset(part) => part.level(),
            Self::Tine(TinePart::Tine) => Some(Knob::TineHeight),
            Self::Tine(TinePart::Pickup) => Some(Knob::PickupGap),
            _ => None,
        }
    }
}

struct Drag {
    model: Instrument,
    target: Target,
    knob: Knob,
    start: (f32, f32),
    initial: f32,
}

struct Frame;

struct PianoView {
    slide: Slide,
    params: Arc<OpenWurliUiParams>,
    keyboard: Rc<PlayableKeyboard>,
    strikes: Rc<[StrikePreview; Instrument::VIEW_COUNT]>,
    font: Cell<Option<FontId>>,
    labels: Cell<Option<FontId>>,
    hover: Option<(f32, f32)>,
    drag: Option<Drag>,
    era_open: bool,
    hohner_open: bool,
    regulation_open: bool,
}

impl PianoView {
    fn strike(&self, model: PianoType) -> &StrikePreview {
        &self.strikes[model.to_index()]
    }

    fn piano(&self) -> Piano {
        let r = self.params.key_range();
        Piano::new(KEYS, *r.start(), *r.end())
    }
    fn reed(&self) -> reed_action::Motion {
        reed_action::Motion::new(
            self.strike(PianoType::Wurli).elapsed(),
            self.strike(PianoType::Wurli).note(),
            self.params.reed_decay.value(),
            self.params.pickup_height.value(),
            self.params.pickup_drive.value(),
        )
    }
    fn tine(&self) -> Voicing {
        let p = &self.params.stage;
        Voicing::for_key(
            p.era.value(),
            self.strike(PianoType::Stage).note(),
            p.height.value(),
            p.gap.value(),
            Some(self.strike(PianoType::Stage).elapsed()),
        )
    }
    fn string_action(&self) -> StringAction {
        let strike = self.strike(PianoType::Grand);
        match self.params.grand_model.value() {
            GrandModel::Cp80 => StringAction::new(strike),
            GrandModel::Cp70B if self.params.original_cp70() => StringAction::cp70(strike),
            GrandModel::Cp70B => StringAction::cp70b(strike),
        }
    }
    fn hit(&self, x: f32, y: f32) -> Option<Target> {
        let on = |r| point_in_rect(x, y, r);
        let p = &self.params;
        let model = p.piano.value();
        let selected = p.instrument();
        if self.hohner_open {
            if (0..Instrument::GROUPS.len()).any(|group| on(hohner::menu_heading(group))) {
                return None;
            }
            if let Some((_, model)) = Instrument::extra_models()
                .enumerate()
                .find(|(i, _)| on(hohner::menu_row(*i)))
            {
                return Some(Target::HohnerChoice(model));
            }
        }
        if self.regulation_open && on(regulation::panel(selected)) {
            return self.regulation_hit(selected, x, y);
        }
        if on(regulation::BUTTON) {
            return Some(Target::PickupMenu);
        }
        if on(hohner::MENU) {
            return Some(Target::HohnerMenu);
        }
        if selected == Instrument::Piano(PianoType::Stage) && self.era_open && !self.slide.moving()
        {
            if let Some(i) = (0..Era::ALL.len()).find(|&i| on(era_row(i))) {
                return Some(Target::EraChoice(i));
            }
        }
        if let Some(i) = (0..3).find(|&i| on(tab(i))) {
            return Some(Target::Piano(i));
        }
        for (r, t) in [
            (KEYS, Target::Keyboard),
            (Knob::Volume.rect(), Target::Dial(Knob::Volume)),
            (EXTENDED, Target::Extended),
            (THEME, Target::Theme),
        ] {
            if on(r) {
                return Some(t);
            }
        }
        if self.slide.moving() {
            return None;
        }
        if let Some((enabled, _)) = self.params.sustain_mod.display(selected) {
            let (switch, pedal) = sustain_rects(selected);
            if on(switch) {
                return Some(Target::SustainMod);
            }
            if enabled && on(pedal) {
                return Some(Target::ModPedal);
            }
        }
        if !matches!(selected, Instrument::Piano(_)) {
            return match selected {
                Instrument::Storytone => self.storytone_hit(x, y),
                Instrument::Neo => self.neo_hit(x, y),
                Instrument::Elepian => self.elepian_hit(x, y),
                Instrument::Vv64 => self.vv64_hit(x, y),
                Instrument::StereoVibe => self.stereovibe_hit(x, y),
                Instrument::Road => self.road_hit(x, y),
                Instrument::Baldwin => self.baldwin_hit(x, y),
                Instrument::Mk8 => self.mk8_hit(x, y),
                Instrument::Vibanet => self.vibanet_hit(x, y),
                Instrument::Valente => self.valente_hit(x, y),
                Instrument::Vivi => self.vivi_hit(x, y),
                Instrument::Claviset(_) => self.claviset_hit(x, y),
                _ => self.hohner_hit(selected, x, y),
            };
        }
        if let Some(&k) = visible(model, p.stage.era.value(), p.stage.peterson.value())
            .iter()
            .find(|k| on(k.rect()))
        {
            return Some(Target::Dial(k));
        }
        if model == PianoType::Grand {
            if on(ORIGINAL_CP70) {
                return Some(Target::OriginalCp70);
            }
            for grand in [GrandModel::Cp80, GrandModel::Cp70B] {
                if on(grand_model_button(grand)) {
                    return Some(Target::GrandModel(grand));
                }
            }
        }
        match model {
            PianoType::Wurli if on(SAG) => Some(Target::Sag),
            PianoType::Stage if p.stage.era.value() != Era::Sparkletop && on(MODEL_SWITCH) => {
                Some(Target::Peterson)
            }
            PianoType::Stage if on(ERA) => Some(Target::Era),
            PianoType::Grand if p.grand_model.value() == GrandModel::Cp80 && on(GRAND_FRAME) => {
                Some(Target::FrameRing)
            }
            PianoType::Grand if on(TREMOLO) => Some(Target::Tremolo),
            PianoType::Stage if p.stage.era.value() == Era::Sparkletop && on(TREMOLO) => {
                Some(Target::Tremolo)
            }
            PianoType::Wurli => {
                reed_action::hit(x, y, self.reed()).map(|part| Target::Reed(part.knob))
            }
            PianoType::Stage => self.tine().hit(x, y).map(Target::Tine),
            PianoType::Grand => (0..3)
                .find(|&i| p.grand_model.value() == GrandModel::Cp80 && on(brilliance(i)))
                .map(Target::Brilliance)
                .or_else(|| self.string_action().hit(x, y).map(Target::String)),
        }
    }
    fn help(&self, target: Option<Target>) -> &'static str {
        match target {
            Some(Target::Dial(Knob::PianetSwell)) if self.params.instrument() == Instrument::Reed(pianet_n::Model::Cembalet) =>
                "Move the early Cembalet knee lever to change the output level.",

            Some(Target::EarlyLocalOut) if self.params.instrument() == Instrument::Reed(pianet_n::Model::C) =>
                "Switch between the external amplifier DIN and the headphone output after the AC125/AC126 driver.",
            Some(Target::EarlyLocalOut) => "Switch between the external amplifier DIN and the voltage across the pair of 5 Ω speakers.",
            Some(Target::Dial(Knob::PianetSwell)) if self.params.instrument() == Instrument::Reed(pianet_n::Model::C) =>
                "Move the 250 kΩ knee control to set Pianet C swell.",
            Some(Target::Dial(Knob::PianetSwell)) if self.params.instrument() == Instrument::Reed(pianet_n::Model::L) =>
                "Move the 1 MΩ knee control to set Pianet L swell. At zero it opens the power switch.",
            None if self.params.sustain_mod.display(self.params.instrument()).is_some_and(|(enabled,_)| enabled) =>
                "With Sustain Mod on, the pedal holds released Pianet pads above the reeds until pedal-up.",
            None if matches!(self.params.instrument(), Instrument::Reed(pianet_n::Model::C | pianet_n::Model::L)) =>
                "Click the adhesive pad to pluck the reed. Its motion changes capacitance at the stationary electrode.",
            Some(Target::Reed(k) | Target::Dial(k)) => k.help(),
            Some(Target::PickupSetting(m, kind)) => kind.help(m),
            Some(Target::PickupMenu) if self.params.instrument() == Instrument::Piano(PianoType::Grand) =>
                "The piezo pickups are bonded to the string bearings. Their fixed contact senses string force.",
            Some(Target::PickupMenu) => "Adjust the pickup distance and, where fitted, the pad return sound. Double-click a slider for stock regulation.",
            Some(Target::Vv64(part)) => part.help(),
            Some(Target::StereoVibe(part)) => part.help(),
            None if self.params.instrument() == Instrument::StereoVibe => "Strike the tine and adjust the active StereoVibe preamp and optical stereo tremolo.",
            Some(Target::Road(part)) => part.help(),
            None if self.params.instrument() == Instrument::Road => "Strike the upright action. The soundboard loads each string and the three magnetic strips feed the mono jack.",
            Some(Target::Baldwin(part)) => part.help(),
            None if self.params.instrument() == Instrument::Baldwin => "Strike the upright hammer to drive the two sections of the piezo pickup rail.",
            Some(Target::Mk8(part)) => part.help(),
            None if self.params.instrument() == Instrument::Mk8 => "Strike a tine and adjust the active preamp or the stereo Vari-Pan.",
            Some(Target::Vibanet(part)) => part.help(),
            None if self.params.instrument() == Instrument::Vibanet => "Strike a string or adjust its pickups, stepped equalizer and envelope-controlled wah.",
            Some(Target::Valente(part)) => part.help(),
            Some(Target::Vivi(part)) => part.help(),
            None if self.params.instrument() == Instrument::Vivi => "Strike a tone bar or move a pedal to change its electrical level or lift the dampers.",
            None if self.params.instrument() == Instrument::Valente => "Strike a steel reed to hear its magnetic pickup through the passive volume and brilliance controls.",
            None if self.params.instrument() == Instrument::Vv64 => "Strike the tine to hear the pickup through the passive volume, bass and treble controls.",
            Some(Target::Elepian(part)) => part.help(),
            None if self.params.instrument() == Instrument::Elepian => "Click the action to strike a reed. The pickup feeds the passive tone and volume circuit.",
            Some(Target::Neo(part)) => part.help(),
            None if self.params.instrument() == Instrument::Neo => "Click the microhammer action to play the string. The pickup heads feed the valve amplifier.",
            Some(Target::Storytone(part)) => part.help(),
            Some(Target::Claviset(part)) => part.help(),
            None if matches!(self.params.instrument(), Instrument::Claviset(_)) => "Click a key to turn the cam and pluck the reed. Release it to apply the felt damper.",
            None if self.params.instrument() == Instrument::Storytone => "Strike a key to see the hammer, strings and bridge plate move. The three pedals control soft, swell and sustain.",
            Some(Target::Duo(part)) => part.help(),
            None if self.params.instrument() == Instrument::Duo => "Play either action from the keyboard. The selector and footswitch route the two instruments.",
            Some(Target::E7Volume) => "Slide horizontally to set the E7 output volume.",
            Some(Target::E7Power) => "Switch the E7 electronics off or on. Strings continue vibrating while power is off.",
            Some(Target::ElectraFork) => "The unequal steel arms exchange vibration after the hammer strikes the lighter arm.",
            Some(Target::ElectraPickup) => "The inductive coil senses the lighter arm. The coils feed a seven-group balancing network and an output transformer.",
            Some(Target::ElectraSoft) => "Insert the left-pedal resistance. MIDI CC67 controls the same circuit.",
            Some(Target::ElectraSustain) => "Raise the dampers so released forks keep ringing. MIDI CC64 controls the same mechanism.",
            None if self.params.instrument() == Instrument::Electra => "Click the hammer or keyboard to play. The right pedal raises the dampers and the left pedal reduces the output.",
            Some(Target::CembaletDamper) => "Press to lift the separate damper. Releasing the key seats it on the reed.",
            Some(Target::CembaletPlectrum) => "Press to raise the plectrum under the reed until it slips past the tip.",
            Some(Target::CembaletElectrode) if self.params.instrument() == Instrument::Reed(pianet_n::Model::Cembalet) =>
                "Reed motion changes the RF tank capacitance. The tube detector recovers the audio.",
            Some(Target::CembaletElectrode) => "Reed motion changes capacitance to the DC-biased electrode and drives the transformer.",
            Some(Target::Tine(TinePart::Pickup)) => Knob::PickupGap.help(),
            Some(Target::Tine(TinePart::Tine)) => Knob::TineHeight.help(),
            Some(Target::GrandModel(_)) => "Choose the grand piano scale and matching pickup circuit.",
            Some(Target::OriginalCp70) => "Select the original CP-70 with its individually wired piezos. Input loading and TR response are estimated.",
            Some(Target::String(StringPart::Piezo)) if self.params.original_cp70() =>
                "The string bears on a piezo in a metal channel. Each key has its own lead to the input board.",
            Some(Target::String(StringPart::Strings)) if self.params.original_cp70() =>
                "Original CP-70 wound courses use the NB500880 string set. Pairs begin at G2 and plain wire at F4.",
            Some(Target::String(StringPart::Strings)) if self.params.grand_model.value() == GrandModel::Cp70B =>
                "CP-70B uses one wound string through F♯2, two wound strings through E4, then pairs of plain wires.",
            Some(Target::String(part)) => part.help(),
            Some(Target::HohnerMenu | Target::HohnerChoice(_)) => "Choose an instrument from the available piano families.",
            Some(Target::HohnerKey) => "Click and hold to play the displayed key. Release it to hear its mechanical damping.",
            Some(Target::Pickup(_)) => "Select the centre or bridge pickup, or combine them in phase or in opposition.",
            Some(Target::ToneTab(_)) if self.params.instrument() == Instrument::Clavinet(ClavinetModel::E7) =>
                "Engage this E7 filter branch. With all four tabs off, the jack has faint residual feedthrough.",
            Some(Target::ToneTab(_)) => "Engage this D6 tone circuit. With all four tabs off the output jack is silent.",
            Some(Target::D6Mute) => "Bring the felt mute against the strings to shorten their vibration.",
            Some(Target::SustainMod) if self.params.instrument()==Instrument::Duo => "Enable the Pianet pad latch. CC64 then holds the Pianet reeds and the registration pedal stays under panel control.",
            Some(Target::SustainMod) => "Enable a pedal-operated latch that keeps released pads clear of their reeds. Switch off to restore the original action.",
            Some(Target::ModPedal) => "Latch released Pianet pads above their reeds. Click again or release MIDI CC64 to return the pads.",
            Some(Target::PianetVibrato) => "Switch the selected instrument’s modulation on or off.",
            Some(Target::D6Phase) => "Exchange the two paired pickup positions to follow the alternate pickup wiring.",
            None if matches!(self.params.instrument(),Instrument::Clavinet(_)) => "Click the tangent to play. Click either pickup to hear it on its own.",
            None if matches!(self.params.instrument(), Instrument::Reed(m) if m.is_cembalet()) => "Click the plectrum or damper to play. The key releases the reed and returns its separate damper.",
            None if self.params.instrument()==Instrument::Reed(pianet_n::Model::N) => "The adhesive pad lifts the reed until it detaches. Key release brings the pad back onto the reed.",
            None if self.params.instrument()==Instrument::Reed(pianet_n::Model::T) => "The silicone pad pulls the reed by suction. A passive magnetic pickup senses each reed.",
            Some(Target::Piano(_)) => "Select the piano to play. Each piano retains its own voicing and tone settings.",
            Some(Target::EraChoice(i)) => Era::ALL[i].help(),
            Some(Target::FrameRing) => "Let undamped strings respond through the electric grand’s frame. Coupling strength is estimated.",
            Some(Target::Peterson) => "Route the harp through the Peterson 80-watt Suitcase preamp. Switch off for the passive Stage panel.",
            Some(Target::Era) => self.params.stage.era.value().help(),
            Some(Target::Brilliance(_)) => "Switch the electric grand’s high-frequency circuit.",
            Some(Target::Extended) => "Extend the selected piano from its original keyboard to all 128 MIDI pitches.",
            Some(Target::Sag) => "Add power-supply sag to soften strong Wurli attacks.",
            Some(Target::Theme) => "Cycle Dark, Light and Auto appearance.",
            Some(Target::Tremolo) => "Switch tremolo on or off. Rate and depth retain their settings.",
            Some(Target::Keyboard) => playable_keyboard::HELP,
            None => match self.params.piano.value() {
                PianoType::Wurli => "OpenWurli DSP by HAL0ZER0 · Drag the hammer, reed or pickup to adjust its voicing.",
                PianoType::Stage if self.params.stage.peterson.value() && self.params.stage.era.value() != Era::Sparkletop =>
                    "Peterson 80-watt Suitcase · The two lamps move the harp signal between the stereo outputs.",
                PianoType::Stage => self.params.stage.era.value().help(),
                PianoType::Grand => "Click the hammer or the keyboard to hear the displayed string action.",
            }
        }
    }
    fn draw_dial(&self, d: &mut Draw, k: Knob) {
        let p = k.param(&self.params);
        let n = p.unmodulated_normalized_value();
        let value = if k == Knob::Speed {
            format!("{:.2} Hz", cp80::engine::tremolo_hz(p.value().into()))
        } else if k == Knob::TremoloResponse {
            format!(
                "{:.1} ms",
                openwurli_dsp::tremolo::release_ms(p.value().into())
            )
        } else {
            p.normalized_value_to_string(n, true)
        };
        let label = k.label(self.params.stage.era.value());
        if let Some((x, y, w)) = k.inline() {
            let (cx, cy) = (x - 19., y - 4.);
            let angle = (135. + 270. * n).to_radians();
            let color = if k == Knob::Volume { GOLD } else { TEAL };
            d.circle(cx, cy, 10., LINE, true);
            d.circle(cx, cy, 8., PANEL, true);
            d.line(
                cx,
                cy,
                cx + 8. * angle.cos(),
                cy + 8. * angle.sin(),
                color,
                2.2,
            );
            // Action captions and their leaders are painted by the physical diagrams.
            if matches!(k, Knob::Volume | Knob::PickupDrive) {
                d.text(x, y, label, 11., TEXT);
            }
            d.text(x + w, y, &value, 11., color);
        } else {
            let inactive = match k {
                Knob::TremoloResponse => self.params.tremolo_depth.value() == 0.,
                Knob::Rate | Knob::Depth => !self.params.stage.tremolo.value(),
                Knob::Speed | Knob::GrandDepth => !self.params.grand.tremolo.value(),
                _ => false,
            };
            let trem = matches!(
                k,
                Knob::Tremolo
                    | Knob::TremoloResponse
                    | Knob::Rate
                    | Knob::Speed
                    | Knob::GrandDepth
                    | Knob::Depth
            );
            d.knob(
                k.rect(),
                label,
                &value,
                n,
                if trem { TEAL } else { GOLD },
                inactive,
            );
        }
    }
    fn tremolo(&self) -> &BoolParam {
        if self.params.piano.value() == PianoType::Stage {
            &self.params.stage.tremolo
        } else {
            &self.params.grand.tremolo
        }
    }
    fn paint(&self, d: &mut Draw) {
        let p = &self.params;
        let model = p.piano.value();
        let target = self
            .drag
            .as_ref()
            .filter(|drag| drag.model == p.instrument())
            .map(|drag| drag.target)
            .or_else(|| self.hover.and_then(|(x, y)| self.hit(x, y)));
        d.hover = self.hover;
        d.square_rect(0., 0., WIDTH, 52., PANEL);
        d.line(0., 51., WIDTH, 51., LINE, 1.);
        d.text(24., 34., "ELECTRIC PIANOS", 21., TEXT);
        for (i, label) in MODEL_LABELS.iter().enumerate() {
            d.tab_button(
                tab(i),
                label,
                p.instrument() == Instrument::Piano(PianoType::ALL[i]),
                GOLD,
            );
        }
        let extra = !matches!(p.instrument(), Instrument::Piano(_));
        let label = match p.instrument() {
            Instrument::Piano(_) => "More pianos",
            Instrument::Duo => "Hohner Duo",
            Instrument::Valente => "Valente",
            Instrument::StereoVibe => "StereoVibe",
            Instrument::Baldwin => "Baldwin",
            Instrument::Vivi => "Vivi-Tone",
            model => model.label(),
        };
        d.tab_button(hohner::MENU, label, extra || self.hohner_open, GOLD);
        let (mx, my, mw, mh) = hohner::MENU;
        d.poly(
            &[
                (mx + mw - 15., my + mh / 2. - 2.),
                (mx + mw - 11., my + mh / 2. + 2.),
                (mx + mw - 7., my + mh / 2. - 2.),
            ],
            MUTED,
            1.4,
        );
        d.c.save();
        d.scissor(0., 52., WIDTH, 352.);
        let offset = d.offset_x;
        for (page, x) in self.slide.panels() {
            if x.abs() >= 1. {
                continue;
            }
            d.offset_x = offset + x * WIDTH;
            d.square_rect(0., 52., WIDTH, 352., BG);
            d.hover = if self.slide.moving() {
                None
            } else {
                self.hover
            };
            match page {
                Instrument::Piano(piano) => {
                    self.paint_model(d, piano, if self.slide.moving() { None } else { target })
                }
                Instrument::Storytone => self.paint_storytone(d),
                Instrument::Neo => self.paint_neo(d),
                Instrument::Elepian => self.paint_elepian(d),
                Instrument::Vv64 => self.paint_vv64(d),
                Instrument::StereoVibe => self.paint_stereovibe(d),
                Instrument::Road => self.paint_road(d),
                Instrument::Baldwin => self.paint_baldwin(d),
                Instrument::Mk8 => self.paint_mk8(d),
                Instrument::Vibanet => self.paint_vibanet(d),
                Instrument::Valente => self.paint_valente(d),
                Instrument::Vivi => self.paint_vivi(d),
                Instrument::Claviset(_) => self.paint_claviset(d),
                other => self.paint_hohner(d, other),
            }
            if let Some((enabled, held)) = p.sustain_mod.display(page) {
                let (switch, pedal) = sustain_rects(page);
                d.toggle_switch(switch, "SUSTAIN MOD", enabled, TEAL);
                if enabled {
                    d.button(pedal, "Sustain pedal", held, GOLD);
                }
            }
        }
        d.offset_x = offset;
        d.hover = self.hover;
        d.c.restore();

        d.panel((18., 409., 692., 139.), PANEL, LINE);
        self.draw_dial(d, Knob::Volume);
        d.toggle_switch(EXTENDED, "EXTENDED NOTES", p.extended_notes.value(), TEAL);
        self.paint_pickup_button(d, p.instrument());
        d.appearance_button(THEME, prefs().label());
        self.keyboard.draw(d, self.piano());
        if !extra && model == PianoType::Stage && self.era_open && !self.slide.moving() {
            let top = era_row(0);
            d.panel(
                (top.0 - 2., top.1 - 2., top.2 + 4., 26. * 6. + 4.),
                PANEL,
                LINE,
            );
            for (i, &era) in Era::ALL.iter().enumerate() {
                d.button(
                    era_row(i),
                    Era::variants()[i],
                    p.stage.era.value() == era,
                    GOLD,
                );
            }
        }
        if self.regulation_open && !crate::regulation::adjustments(p.instrument()).is_empty() {
            self.paint_regulation(d, p.instrument());
        }
        if self.hohner_open {
            for (group, (_, models)) in Instrument::GROUPS.iter().enumerate() {
                let (x, y, w, h) = hohner::menu_heading(group);
                d.panel(
                    (
                        x - 2.,
                        y - 2.,
                        w + 4.,
                        h + models.len() as f32 * hohner::ROW_HEIGHT + 4.,
                    ),
                    PANEL,
                    LINE,
                );
            }
            for (group, (label, _)) in Instrument::GROUPS.iter().enumerate() {
                let (x, y, w, h) = hohner::menu_heading(group);
                d.square_rect(x, y, w, h, PANEL);
                d.text(x + 9., y + 15., label, 11., TEXT);
            }
            for (i, model) in Instrument::extra_models().enumerate() {
                d.button(
                    hohner::menu_row(i),
                    model.label(),
                    p.instrument() == model,
                    GOLD,
                );
            }
        }
        d.bottom_tooltip(
            (26., HEIGHT - TOOLTIP_HEIGHT, 676., TOOLTIP_HEIGHT),
            self.help(target),
            MUTED,
        );
    }
    fn paint_model(&self, d: &mut Draw, model: PianoType, target: Option<Target>) {
        let p = &self.params;
        match model {
            PianoType::Wurli => reed_action::draw(d, self.reed(), target.and_then(Target::knob)),
            PianoType::Stage => self.tine().draw(
                d,
                match target.and_then(Target::knob) {
                    Some(Knob::TineHeight) => Some(TinePart::Tine),
                    Some(Knob::PickupGap) => Some(TinePart::Pickup),
                    _ => None,
                },
            ),
            PianoType::Grand => self.string_action().draw(
                d,
                match target {
                    Some(Target::String(part)) => Some(part),
                    _ => None,
                },
            ),
        }
        d.panel((18., 267., 692., 131.), PANEL, LINE);
        let title = match model {
            PianoType::Wurli => "TONE & RESPONSE",
            PianoType::Stage => "RHODES ERA",
            PianoType::Grand => "TONE",
        };
        if model != PianoType::Grand {
            d.text(32., 290., title, 12., TEXT);
        }
        match model {
            PianoType::Wurli => d.toggle_switch(SAG, "EXTRA SAG", p.rail_sag.value(), TEAL),
            PianoType::Stage => {
                d.button(
                    ERA,
                    Era::variants()[p.stage.era.value().to_index()],
                    self.era_open,
                    GOLD,
                );
                let (x, y) = (ERA.0 + ERA.2 - 15., ERA.1 + 15.);
                d.poly(
                    &[(x - 4., y - 2.), (x, y + 2.), (x + 4., y - 2.)],
                    TEXT,
                    1.4,
                );
                if p.stage.era.value() == Era::Sparkletop {
                    d.toggle_switch(TREMOLO, "TREMOLO", p.stage.tremolo.value(), TEAL);
                } else {
                    d.toggle_switch(
                        MODEL_SWITCH,
                        "PETERSON SUITCASE",
                        p.stage.peterson.value(),
                        TEAL,
                    );
                }
            }
            PianoType::Grand => {
                for grand in [GrandModel::Cp80, GrandModel::Cp70B] {
                    d.tab_button(
                        grand_model_button(grand),
                        GrandModel::variants()[grand.to_index()],
                        p.grand_model.value() == grand && !p.original_cp70(),
                        GOLD,
                    );
                }
                d.button(ORIGINAL_CP70, "CP-70", p.original_cp70(), GOLD);
                if p.grand_model.value() == GrandModel::Cp80 {
                    d.toggle_switch(GRAND_FRAME, "FRAME RING", p.grand.frame_ring.value(), TEAL);
                    d.text(312., 321., "BRILLIANCE", 11., TEXT);
                    for (i, label) in ["Low", "Med", "High"].iter().enumerate() {
                        d.button(
                            brilliance(i),
                            label,
                            p.grand.brilliance.value() == BRILLIANCE[i],
                            GOLD,
                        );
                    }
                }
                d.toggle_switch(TREMOLO, "TREMOLO", p.grand.tremolo.value(), TEAL);
            }
        }
        for &k in visible(model, p.stage.era.value(), p.stage.peterson.value()) {
            self.draw_dial(d, k);
        }
    }

    fn end(&mut self, cx: &mut EventContext) {
        if let Some(drag) = self.drag.take() {
            param::end(cx, drag.knob.param(&self.params).as_ptr());
            cx.release();
        }
    }
    fn press(&mut self, cx: &mut EventContext, target: Option<Target>, x: f32, y: f32) {
        self.end(cx);
        if !matches!(target, Some(Target::PickupSetting(..) | Target::PickupMenu)) {
            self.regulation_open = false;
        }
        if self.hohner_open {
            self.hohner_open = false;
            match target {
                Some(Target::HohnerChoice(model)) => {
                    set(cx, &self.params.claviset_model, model.into());
                    set(cx, &self.params.neo_enabled, model == Instrument::Neo);
                    set(cx, &self.params.vv64_enabled, model == Instrument::Vv64);
                    set(
                        cx,
                        &self.params.stereovibe_enabled,
                        model == Instrument::StereoVibe,
                    );
                    set(cx, &self.params.road_enabled, model == Instrument::Road);
                    set(
                        cx,
                        &self.params.baldwin_enabled,
                        model == Instrument::Baldwin,
                    );
                    set(cx, &self.params.mk8_enabled, model == Instrument::Mk8);
                    set(
                        cx,
                        &self.params.vibanet_enabled,
                        model == Instrument::Vibanet,
                    );
                    set(
                        cx,
                        &self.params.valente_enabled,
                        model == Instrument::Valente,
                    );
                    set(cx, &self.params.vivi_enabled, model == Instrument::Vivi);
                    set(
                        cx,
                        &self.params.elepian_enabled,
                        model == Instrument::Elepian,
                    );
                    set(
                        cx,
                        &self.params.storytone_enabled,
                        model == Instrument::Storytone,
                    );
                    set(cx, &self.params.duo_enabled, model == Instrument::Duo);
                    set(
                        cx,
                        &self.params.electra_enabled,
                        model == Instrument::Electra,
                    );
                    if let Instrument::Clavinet(revision) = model {
                        set(cx, &self.params.clavinet_model, revision);
                    }
                    if let Some(revision) = model.pianet_revision() {
                        set(cx, &self.params.pianet_revision, revision);
                    }
                    if let Some(revision) = model.reed_selection() {
                        set(cx, &self.params.reed_model, revision);
                    }
                    set(cx, &self.params.model_bank, model.bank());
                    return;
                }
                Some(Target::Piano(_)) => {}
                _ => return,
            }
        }
        if self.era_open {
            self.era_open = false;
            if let Some(Target::EraChoice(i)) = target {
                set(cx, &self.params.stage.era, Era::ALL[i]);
                return;
            }
            if !matches!(target, Some(Target::Piano(_))) {
                return;
            }
        }
        match target {
            Some(Target::GrandModel(model)) => {
                set(cx, &self.params.grand_model, model);
                if self.params.grand_70_original.value() {
                    set(cx, &self.params.grand_70_original, false);
                }
            }
            Some(Target::OriginalCp70) => {
                set(cx, &self.params.grand_70_original, true);
                set(cx, &self.params.grand_model, GrandModel::Cp70B);
            }
            Some(Target::Piano(i)) => {
                set(cx, &self.params.electra_enabled, false);
                set(cx, &self.params.duo_enabled, false);
                set(cx, &self.params.storytone_enabled, false);
                set(cx, &self.params.neo_enabled, false);
                set(cx, &self.params.elepian_enabled, false);
                set(cx, &self.params.vv64_enabled, false);
                set(cx, &self.params.stereovibe_enabled, false);
                set(cx, &self.params.road_enabled, false);
                set(cx, &self.params.baldwin_enabled, false);
                set(cx, &self.params.mk8_enabled, false);
                set(cx, &self.params.vibanet_enabled, false);
                set(cx, &self.params.valente_enabled, false);
                set(cx, &self.params.vivi_enabled, false);
                set(
                    cx,
                    &self.params.claviset_model,
                    crate::models::ClavisetModel::Off,
                );
                set(cx, &self.params.piano, PianoType::ALL[i]);
                set(cx, &self.params.model_bank, ModelBank::Piano);
            }
            Some(Target::HohnerMenu) => self.hohner_open = true,
            Some(Target::Pickup(choice)) => set(cx, &self.params.d6.pickup, choice),
            Some(Target::ToneTab(i)) => toggle(cx, self.params.d6.tabs()[i]),
            Some(Target::D6Mute) => toggle(cx, &self.params.d6.mute),
            Some(Target::ElectraSoft) => toggle(cx, &self.params.electra_soft),
            Some(Target::ElectraSustain) => toggle(cx, &self.params.electra_sustain),
            Some(Target::Claviset(part)) if part.level().is_none() => self.claviset_press(cx, part),
            Some(Target::Vv64(part)) if part.level().is_none() => self.vv64_press(cx, part),
            Some(Target::StereoVibe(part)) if part.level().is_none() => {
                self.stereovibe_press(cx, part)
            }
            Some(Target::Road(part)) => self.road_press(cx, part),
            Some(Target::Baldwin(part)) if part.level().is_none() => self.baldwin_press(cx, part),
            Some(Target::Mk8(part)) if part.level().is_none() => self.mk8_press(cx, part),
            Some(Target::Vibanet(part)) if part.level().is_none() => self.vibanet_press(cx, part),
            Some(Target::Valente(part)) if part.level().is_none() => self.valente_press(cx, part),
            Some(Target::Vivi(part)) if part.level().is_none() => self.vivi_press(cx, part),
            Some(Target::Elepian(part)) if part.level().is_none() => self.elepian_press(cx, part),
            Some(Target::Neo(part)) if part.level().is_none() => self.neo_press(cx, part),
            Some(Target::Storytone(part)) if part.level().is_none() => {
                self.storytone_press(cx, part)
            }
            Some(Target::Duo(part)) if part.level().is_none() => self.duo_press(cx, part),
            Some(Target::E7Power) => toggle(cx, &self.params.clavinet_e7_power),
            Some(Target::D6Phase) => toggle(cx, &self.params.d6.invert_da),
            Some(Target::SustainMod | Target::ModPedal) => {
                if let Some(m) = self.params.sustain_mod.for_model(self.params.instrument()) {
                    if target == Some(Target::SustainMod) {
                        toggle(cx, &m.enabled);
                    } else if m.enabled.value() {
                        toggle(cx, &m.pedal);
                    }
                }
            }
            Some(Target::PianetVibrato) => toggle(cx, self.params.reed_controls().1),
            Some(Target::EarlyLocalOut) => toggle(cx, &self.params.early_local_output),
            Some(Target::FrameRing) => toggle(cx, &self.params.grand.frame_ring),
            Some(Target::Peterson) => toggle(cx, &self.params.stage.peterson),
            Some(Target::Era) => self.era_open = true,
            Some(Target::Brilliance(i)) => set(cx, &self.params.grand.brilliance, BRILLIANCE[i]),
            Some(Target::PickupMenu) => {
                self.regulation_open = !self.regulation_open
                    && !crate::regulation::adjustments(self.params.instrument()).is_empty()
            }
            Some(Target::Extended) => toggle(cx, &self.params.extended_notes),
            Some(Target::Sag) => toggle(cx, &self.params.rail_sag),
            Some(Target::Tremolo) => toggle(cx, self.tremolo()),
            Some(Target::Theme) => {
                prefs().toggle();
            }
            Some(t) if t.knob().is_some() => {
                let knob = t.knob().unwrap();
                let p = knob.param(&self.params);
                param::begin(cx, p.as_ptr());
                if let Target::PickupSetting(m, kind) = t {
                    param::set_normalized(cx, p.as_ptr(), regulation::slider_value(m, kind, x));
                }
                if let Target::Claviset(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::Vv64(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::StereoVibe(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::Baldwin(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::Mk8(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::Vibanet(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::Valente(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::Vivi(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::Elepian(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::Neo(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::Storytone(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if let Target::Duo(part) = t {
                    param::set_normalized(cx, p.as_ptr(), part.slider_value(x));
                }
                if t == Target::E7Volume {
                    param::set_normalized(cx, p.as_ptr(), hohner::slider_value(x));
                }
                self.drag = Some(Drag {
                    model: self.params.instrument(),
                    target: t,
                    knob,
                    start: (x, y),
                    initial: p.unmodulated_normalized_value(),
                });
                cx.capture();
            }
            _ => {}
        }
    }
}

fn sustain_rects(model: Instrument) -> (Rect, Rect) {
    if model == Instrument::Duo {
        ((182., 217., 135., 31.), (324., 217., 123., 31.))
    } else {
        ((317., 344., 139., 31.), (461., 344., 122., 31.))
    }
}
fn set<P: Param>(cx: &mut EventContext, p: &P, value: P::Plain) {
    param::set_normalized_once(cx, p.as_ptr(), p.preview_normalized(value));
}
fn toggle(cx: &mut EventContext, p: &BoolParam) {
    set(cx, p, !p.value());
}

impl View for PianoView {
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        if let Some(drag) = &self.drag {
            let p = &self.params;
            let model = p.piano.value();
            if drag.model != p.instrument()
                || (matches!(drag.model, Instrument::Piano(_))
                    && !matches!(drag.knob, Knob::Volume | Knob::PickupSetting(..))
                    && !visible(model, p.stage.era.value(), p.stage.peterson.value())
                        .contains(&drag.knob))
            {
                self.end(cx);
            }
        }
        event.map(|_: &Frame, _| {
            let now = Instant::now();
            if self.slide.select(self.params.instrument(), now) {
                self.end(cx);
                self.era_open = false;
                self.hohner_open = false;
            }
            if self.slide.tick(now) {
                cx.needs_redraw();
            }
        });
        event.map(|e, _| {
            let b = cx.bounds();
            let Some(v) = EditorViewport::fit((b.x, b.y, b.w, b.h), SIZE) else {
                return;
            };
            let (x, y) = v.to_local(cx.mouse().cursorx, cx.mouse().cursory);
            let target = self.hit(x, y);
            self.keyboard.event_for_note(cx, e, || {
                if self.era_open || self.hohner_open {
                    None
                } else {
                    match target {
                        Some(Target::Keyboard) => self.piano().note_at(x, y),
                        Some(
                            Target::HohnerKey | Target::CembaletDamper | Target::CembaletPlectrum,
                        ) => Some(self.strikes[self.params.instrument().index()].note()),
                        Some(Target::String(StringPart::Hammer)) => {
                            Some(self.strike(PianoType::Grand).note())
                        }
                        _ => None,
                    }
                }
            });
            if self.params.piano.value() != PianoType::Stage {
                self.era_open = false;
            }
            match e {
                WindowEvent::MouseDown(MouseButton::Left) => self.press(cx, target, x, y),
                WindowEvent::MouseDoubleClick(MouseButton::Left) if !self.era_open => {
                    if let Some(k) = target.and_then(Target::knob) {
                        self.end(cx);
                        let p = k.param(&self.params);
                        param::set_normalized_once(cx, p.as_ptr(), p.default_normalized_value());
                    }
                }
                WindowEvent::MouseMove(..) => {
                    self.hover = Some((x, y));
                    if let Some(drag) = &self.drag {
                        let p = drag.knob.param(&self.params);
                        let value = match drag.target {
                            Target::PickupSetting(m, kind) => regulation::slider_value(m, kind, x),
                            Target::E7Volume => hohner::slider_value(x),
                            Target::Duo(part) => part.slider_value(x),
                            Target::Storytone(part) => part.slider_value(x),
                            Target::Neo(part) => part.slider_value(x),
                            Target::Elepian(part) => part.slider_value(x),
                            Target::Vv64(part) => part.slider_value(x),
                            Target::StereoVibe(part) => part.slider_value(x),
                            Target::Baldwin(part) => part.slider_value(x),
                            Target::Mk8(part) => part.slider_value(x),
                            Target::Vibanet(part) => part.slider_value(x),
                            Target::Valente(part) => part.slider_value(x),
                            Target::Vivi(part) => part.slider_value(x),
                            Target::Claviset(part) => part.slider_value(x),
                            Target::Tine(part) => p.preview_normalized(
                                p.preview_plain(drag.initial) + part.drag_mm(drag.start, (x, y)),
                            ),
                            Target::Reed(Knob::PickupHeight) => p.preview_normalized(
                                p.preview_plain(drag.initial)
                                    + (drag.start.1 - y) / reed_action::HEIGHT_PX_PER_MM,
                            ),
                            _ => drag.initial + (drag.start.1 - y) / 140.,
                        };
                        param::set_normalized(cx, p.as_ptr(), value.clamp(0., 1.));
                    }
                }
                WindowEvent::MouseUp(MouseButton::Left) => self.end(cx),
                WindowEvent::FocusOut => {
                    self.end(cx);
                    self.era_open = false;
                    self.hohner_open = false;
                }
                WindowEvent::KeyDown(Code::Escape, _) => {
                    self.era_open = false;
                    self.hohner_open = false;
                }
                WindowEvent::MouseLeave => self.hover = None,
                _ => return,
            }
            cx.needs_redraw();
        });
    }
    fn draw(&self, cx: &mut DrawContext, c: &mut Canvas) {
        let b = cx.bounds();
        load_font_once(c, &self.font, FONT_JETBRAINS_MONO);
        let labels = load_font_once(
            c,
            &self.labels,
            nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR,
        );
        if let Some(mut d) = Draw::new_fitted(
            c,
            prefs().light(),
            (b.x, b.y, b.w, b.h),
            SIZE,
            self.font.get(),
        ) {
            d.with_readable_style(labels, |d| self.paint(d));
        }
    }
}

pub fn create(params: Arc<OpenWurliUiParams>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, gui| {
            let keyboard = Rc::new(PlayableKeyboard::new(params.keyboard.clone(), move || {
                gui.request_process()
            }));
            let strikes: Rc<[StrikePreview; Instrument::VIEW_COUNT]> = Rc::default();
            let poll = params.clone();
            let last = Cell::new(None);
            PianoView {
                slide: Slide::new(params.instrument()),
                params: params.clone(),
                keyboard: keyboard.clone(),
                strikes: strikes.clone(),
                font: Cell::default(),
                labels: Cell::default(),
                hover: None,
                drag: None,
                era_open: false,
                hohner_open: false,
                regulation_open: false,
            }
            .build(cx, |cx| {
                start_frame_timer(cx, pleasant_ui::FRAME_INTERVAL, move |cx| {
                    cx.emit(Frame);
                    let mut knobs = [0; 10];
                    for (slot, k) in knobs.iter_mut().zip(visible(
                        poll.piano.value(),
                        poll.stage.era.value(),
                        poll.stage.peterson.value(),
                    )) {
                        *slot = k.param(&poll).value().to_bits();
                    }
                    let mode = prefs().mode();
                    let key = (
                        (
                            (
                                (
                                    poll.instrument(),
                                    match poll.instrument() {
                                        Instrument::Claviset(which) => {
                                            Some(poll.claviset.display_state(which))
                                        }
                                        _ => None,
                                    },
                                ),
                                (
                                    (
                                        poll.duo.display_state(),
                                        poll.sustain_mod.display(poll.instrument()),
                                        poll.sustain_mod
                                            .pad_is_caught(strikes[poll.instrument().index()].note()),
                                    ),
                                    {
                                        let model = poll.instrument();
                                        let mut values = [0; 4];
                                        for (slot, &kind) in values
                                            .iter_mut()
                                            .zip(crate::regulation::adjustments(model))
                                        {
                                            *slot = crate::regulation::param(&poll, model, kind)
                                                .value()
                                                .to_bits();
                                        }
                                        values
                                    },
                                ),
                                (
                                    (
                                        poll.storytone.display(),
                                        poll.neo.master.value(),
                                        (
                                            poll.neo.display(),
                                            poll.elepian.display_state(),
                                            poll.vv64.display_state(),
                                            poll.stereovibe.display_state(),
                                            poll.road.display_state(),
                                            (poll.baldwin.volume.value(), poll.baldwin.display()),
                                            poll.mk8.display_state(),
                                            poll.vibanet.controls(),
                                            poll.valente.display_state(),
                                            poll.vivi.display_state(),
                                        ),
                                    ),
                                    poll.storytone.optimum.value(),
                                    poll.storytone.manual.value(),
                                ),
                            ),
                            (poll.grand_model.value(), poll.grand_70_original.value()),
                            (poll.d6.pickup.value(), poll.clavinet_e7_power.value()),
                            poll.d6.mask(),
                            poll.d6.mute.value(),
                            poll.d6.invert_da.value(),
                            (
                                poll.clavinet_volume().value().to_bits(),
                                poll.electra_volume.value().to_bits(),
                                poll.electra_soft_depth.value().to_bits(),
                                poll.electra_pedal_state(),
                            ),
                            poll.reed_controls().0.value().to_bits(),
                            (
                                poll.reed_controls().1.value(),
                                poll.early_local_output.value(),
                            ),
                        ),
                        (poll.stage.era.value(), poll.stage.peterson.value()),
                        poll.stage.tremolo.value(),
                        (poll.grand.tremolo.value(), poll.grand.frame_ring.value()),
                        poll.grand.brilliance.value(),
                        poll.rail_sag.value(),
                        poll.extended_notes.value(),
                        poll.volume.value().to_bits(),
                        mode,
                        mode.resolved(),
                        knobs,
                    );
                    let changed = last.replace(Some(key)) != Some(key);
                    if strikes[poll.instrument().index()].poll(&keyboard) || changed {
                        cx.needs_redraw();
                    }
                })
            })
            .width(Stretch(1.))
            .height(Stretch(1.));
        },
    )
}

#[cfg(test)]
mod tests;
