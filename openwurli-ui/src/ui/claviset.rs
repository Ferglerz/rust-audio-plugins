//! Key, one-way cam, lifted damper and native console controls.
use super::{set, toggle, EventContext, Instrument, Knob, PianoView, Rect, Target, WIDTH};
use crate::claviset::{RateTab, ToneTab};
use pianet_n::claviset::Instrument as CamPiano;
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, FELT, RUBBER, STEEL, WOOD},
    point_in_rect,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    FootSwell,
    KneeSwell,
    Tone,
    Depth,
    Rate,
    Register(usize),
    Modulation(usize),
    Sustain,
    Power,
}
const NAMES: [&str; 9] = [
    "Bass",
    "Guitar",
    "Tutti",
    "Piano / Vibes",
    "Harp",
    "Celesta",
    "Music box",
    "Metallophone",
    "Xylophone",
];
const RATES: [&str; 4] = ["Off", "Slow", "Medium", "Fast"];
const KEY: Rect = (40., 100., 379., 138.);
impl Part {
    pub(super) fn level(self) -> Option<Knob> {
        Some(match self {
            Self::FootSwell => Knob::ClavisetSwell,
            Self::KneeSwell => Knob::PianotronSwell,
            Self::Tone => Knob::PianotronTone,
            Self::Depth => Knob::PianotronDepth,
            Self::Rate => Knob::PianotronRate,
            _ => return None,
        })
    }
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::FootSwell => (451., 113., 229., 40.),
            Self::KneeSwell => (38., 285., 146., 79.),
            Self::Tone => (204., 285., 146., 79.),
            Self::Depth => (370., 285., 146., 79.),
            Self::Rate => (536., 285., 146., 79.),
            Self::Register(i) => (
                30. + (i % 3) as f32 * 104.,
                271. + (i / 3) as f32 * 39.,
                100.,
                34.,
            ),
            Self::Modulation(i) => (
                365. + (i % 2) as f32 * 101.,
                296. + (i / 2) as f32 * 36.,
                96.,
                31.,
            ),
            Self::Sustain => (451., 171., 229., 29.),
            Self::Power => (586., 313., 94., 31.),
        }
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (left, _, w, _) = self.rect();
        ((x - left - 6.) / (w - 12.)).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self{
            Self::FootSwell=>"Move the left volume pedal. MIDI CC11 takes control of its travel.",
            Self::KneeSwell=>"Move the knee lever under the keyboard. MIDI CC11 takes control of its travel.",
            Self::Tone=>"Turn the Selmer tone control from its low to high voicing.",
            Self::Depth=>"Set the depth of the Selmer’s amplitude modulation.",
            Self::Rate=>"Set the frequency of the Selmer’s amplitude modulation.",
            Self::Register(_)=>"Select one register from the interlocking tone tabs.",
            Self::Modulation(_)=>"Select the modulation speed or switch the oscillator out of the audio path.",
            Self::Sustain=>"Raise the damper rail with the Bowden pedal. MIDI CC64 can also hold the dampers up.",
            Self::Power=>"Switch the Model 200’s battery preamp on or off.",
        }
    }
}
fn slider(d: &mut Draw, p: Part, label: &str, value: f64) {
    let (x, y, w, h) = p.rect();
    let v = value as f32;
    d.text(x + 6., y + 13., label, 11., TEAL);
    let ly = y + h * 0.66;
    d.line(x + 6., ly, x + w - 6., ly, LINE, 4.);
    let cx = x + 6. + (w - 12.) * v;
    d.panel((cx - 3., ly - 7., 6., 14.), TEAL, LINE);
    d.text_right(x + w - 6., y + 13., &format!("{:.0}%", v * 100.), 10., TEXT);
}

impl PianoView {
    pub(super) fn claviset_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, KEY) {
            return Some(Target::HohnerKey);
        }
        let Instrument::Claviset(which) = self.params.instrument() else {
            return None;
        };
        let hit = |p: Part| point_in_rect(x, y, p.rect());
        let p = match which {
            CamPiano::Claviset200 => (0..9)
                .map(Part::Register)
                .chain((0..4).map(Part::Modulation))
                .chain([Part::Sustain, Part::Power, Part::FootSwell])
                .find(|p| hit(*p)),
            CamPiano::Pianotron => [Part::KneeSwell, Part::Tone, Part::Depth, Part::Rate]
                .into_iter()
                .find(|p| hit(*p)),
        };
        p.map(Target::Claviset)
    }
    pub(super) fn claviset_press(&mut self, cx: &mut EventContext, part: Part) {
        let p = &self.params.claviset;
        match part {
            Part::Register(i) => set(cx, &p.register, ToneTab::ALL[i]),
            Part::Modulation(i) => set(cx, &p.modulation, RateTab::ALL[i]),
            Part::Sustain => toggle(cx, &p.sustain_pedal),
            Part::Power => toggle(cx, &p.power),
            _ => {}
        }
    }
    pub(super) fn paint_claviset(&self, d: &mut Draw) {
        let Instrument::Claviset(which) = self.params.instrument() else {
            return;
        };
        let p = &self.params.claviset;
        let (c, pedal) = p.display_state(which);
        let k = &self.strikes[Instrument::Claviset(which).index()];
        let n = k.note();
        let t = k.elapsed();
        let held = self.keyboard.is_down(n);
        let key = if held { (t / 0.08).min(1.) } else { 0. };
        let gap = match which {
            CamPiano::Claviset200 => self.params.regulation.cam.value(),
            CamPiano::Pianotron => self.params.regulation.selmer.value(),
        } * 7.;
        let damper = key.max(f32::from(pedal));
        let tip = if held || pedal {
            action::vibration(t, 0.61) * 5.
        } else {
            0.
        };
        let title = match which {
            CamPiano::Pianotron => "SELMER PIANOTRON  ·  NO. 8701",
            CamPiano::Claviset200 => "WELTMEISTER  ·  CLAVISET 200",
        };
        action::panel(d, WIDTH, title, n);
        action::hardware(d, |d| {
            let front = 190. + 8. * key;
            let rear = 179. - 9. * key;
            action::solid(
                d,
                [
                    (50., front),
                    (239., rear),
                    (239., rear + 7.),
                    (50., front + 8.),
                ],
                (5., -3.),
                WOOD,
            );
            action::screw(d, 151., 190., 5.);
            d.line(235., rear, 246., 157., STEEL, 4.);
            d.circle(246., 157., 15., CASTING, true);
            let a = (-24. + 100. * key).to_radians();
            let (s, co) = a.sin_cos();
            let end = (246. + 21. * co, 157. - 21. * s);
            d.line(246., 157., end.0, end.1, RUBBER, 7.);
            action::screw(d, 246., 157., 4.);
            d.poly(
                &[
                    (365., 147.),
                    (326., 147. + tip * 0.16),
                    (290., 147. + tip * 0.62),
                    (263., 147. + tip),
                ],
                STEEL,
                3.,
            );
            action::solid(
                d,
                [(361., 141.), (383., 141.), (383., 153.), (361., 153.)],
                (4., -2.),
                CASTING,
            );
            d.line(225., rear, 212., 125. - 11. * damper, CASTING, 2.);
            d.line(
                212.,
                125. - 11. * damper,
                294.,
                138. - 11. * damper,
                STEEL,
                2.,
            );
            d.panel((286., 137. - 11. * damper, 17., 9.), FELT, CASTING);
            action::solid(
                d,
                [
                    (272., 161. + gap),
                    (288., 161. + gap),
                    (288., 182. + gap),
                    (272., 182. + gap),
                ],
                (3., -2.),
                CASTING,
            );
            for i in 0..7 {
                let y = 167. + gap + i as f32 * 1.7;
                d.line(270., y, 290., y - 1., WOOD, 1.3);
            }
            d.poly(
                &[(283., 181. + gap), (298., 192.), (328., 192.)],
                STEEL,
                1.3,
            );
        });
        d.text(54., 222., "KEY", 10., MUTED);
        d.text(194., 222., "ROTATING CAM", 10., MUTED);
        d.text(317., 222., "REED / COIL", 10., MUTED);
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        match which {
            CamPiano::Claviset200 => {
                slider(d, Part::FootSwell, "FOOT SWELL", c.swell);
                d.button(Part::Sustain.rect(), "Damper pedal", pedal, GOLD);
                for (i, name) in NAMES.into_iter().enumerate() {
                    d.button(
                        Part::Register(i).rect(),
                        name,
                        p.register.value() == ToneTab::ALL[i],
                        GOLD,
                    );
                }
                d.text(370., 285., "VIBRATO", 11., TEAL);
                for (i, name) in RATES.into_iter().enumerate() {
                    d.button(
                        Part::Modulation(i).rect(),
                        name,
                        p.modulation.value() == RateTab::ALL[i],
                        TEAL,
                    );
                }
                d.toggle_switch(Part::Power.rect(), "POWER", c.power, TEAL);
            }
            CamPiano::Pianotron => {
                d.text(451., 129., "MAGNETIC PICKUP", 11., TEAL);
                d.text(451., 156., "TRANSFORMER", 11., TEXT);
                d.text(451., 183., "9 V PREAMP", 11., TEXT);
                for (part, label, v) in [
                    (Part::KneeSwell, "KNEE SWELL", c.swell),
                    (Part::Tone, "TONE", c.tone),
                    (Part::Depth, "VIBRATO DEPTH", c.depth),
                    (Part::Rate, "VIBRATO SPEED", c.rate),
                ] {
                    slider(d, part, label, v);
                }
            }
        }
    }
}
