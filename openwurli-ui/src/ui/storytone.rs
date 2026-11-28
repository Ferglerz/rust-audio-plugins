//! Storytone upright action and controls from the 1940 service sheet.
use super::{toggle, EventContext, Instrument, Knob, PianoView, Rect, Target, WIDTH};
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, FELT, RUBBER, STEEL, WOOD},
    point_in_rect,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Optimum,
    Manual,
    Swell,
    Soft,
    Sustain,
}

const CONTROLS: [(Part, Rect); 5] = [
    (Part::Optimum, (32., 282., 201., 46.)),
    (Part::Manual, (260., 282., 201., 46.)),
    (Part::Swell, (487., 282., 201., 46.)),
    (Part::Soft, (32., 352., 201., 30.)),
    (Part::Sustain, (260., 352., 201., 30.)),
];
impl Part {
    pub(super) fn rect(self) -> Rect {
        CONTROLS.iter().find(|(part, _)| *part == self).unwrap().1
    }
    pub(super) fn level(self) -> Option<Knob> {
        match self {
            Self::Optimum => Some(Knob::StoryOptimum),
            Self::Manual => Some(Knob::StoryManual),
            Self::Swell => Some(Knob::StorySwell),
            _ => None,
        }
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (left, _, width, _) = self.rect();
        ((x - left - 6.) / (width - 12.)).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self {
            Self::Optimum => "Set the service volume at the 6J7 input. This sets the ceiling available to the swell pedal.",
            Self::Manual => "Set the playing volume with the swell pedal at rest. The pedal can raise the level from this setting.",
            Self::Swell => "Move the electrical swell pedal toward full volume. MIDI CC11 also operates this control.",
            Self::Soft => "Shorten the hammer travel for the next strike. MIDI CC67 also operates the soft pedal.",
            Self::Sustain => "Lift the string dampers so released keys continue sounding. MIDI CC64 also operates this pedal.",
        }
    }
}

impl PianoView {
    pub(super) fn storytone_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (43., 101., 390., 131.)) {
            return Some(Target::HohnerKey);
        }
        CONTROLS
            .iter()
            .find(|(_, r)| point_in_rect(x, y, *r))
            .map(|(p, _)| Target::Storytone(*p))
    }
    pub(super) fn storytone_press(&mut self, cx: &mut EventContext, part: Part) {
        match part {
            Part::Soft => toggle(cx, &self.params.storytone.soft),
            Part::Sustain => toggle(cx, &self.params.storytone.sustain),
            _ => {}
        }
    }
    pub(super) fn paint_storytone(&self, d: &mut Draw) {
        let p = &self.params.storytone;
        let (swell, soft, sustain) = p.display();
        let strike = &self.strikes[Instrument::Storytone.index()];
        let n = strike.note();
        let age = strike.elapsed();
        let down = self.keyboard.is_down(n);
        let lift = action::hammer_lift(age);
        let ring = if down || sustain || n > 88 {
            action::vibration(age, 0.68)
        } else {
            0.
        };
        action::panel(d, WIDTH, "STORYTONE  ·  MAGNETIC STRING PICKUP", n);
        action::hardware(d, |d| {
            // Side section of one upright string, hammer, damper and pole-piece.
            let string_x = 268.;
            d.poly(
                &[
                    (string_x, 100.),
                    (string_x + 2.8 * ring, 137.),
                    (string_x + 3.4 * ring, 171.),
                    (string_x, 217.),
                ],
                STEEL,
                2.2,
            );
            for y in [101., 216.] {
                action::solid(
                    d,
                    [
                        (260., y - 4.),
                        (276., y - 4.),
                        (276., y + 1.),
                        (260., y + 1.),
                    ],
                    (5., -2.),
                    CASTING,
                );
            }
            let hx = 231. + lift * 24.;
            d.line(180., 203., hx - 6., 149., WOOD, 5.);
            d.circle(hx, 145., 12., FELT, true);
            d.circle(hx - 4., 145., 7., WOOD, true);
            action::screw(d, 180., 203., 5.);
            let key = if down { 4. } else { 0. };
            action::solid(
                d,
                [
                    (49., 201. + key),
                    (192., 205. - key),
                    (192., 213. - key),
                    (49., 211. + key),
                ],
                (5., -3.),
                WOOD,
            );
            if n <= 88 {
                let dx = if down || sustain { 13. } else { 0. };
                d.line(183., 114., 245. - dx, 129., CASTING, 3.);
                action::solid(
                    d,
                    [
                        (245. - dx, 123.),
                        (265. - dx, 123.),
                        (265. - dx, 134.),
                        (245. - dx, 134.),
                    ],
                    (3., -2.),
                    RUBBER,
                );
            }
            action::solid(
                d,
                [(279., 154.), (349., 154.), (349., 165.), (279., 165.)],
                (6., -4.),
                CASTING,
            );
            for i in 0..15 {
                let x = 299. + i as f32 * 2.;
                d.line(x, 150., x + 1.6, 168., WOOD, 1.7);
            }
            d.poly(&[(329., 165.), (348., 179.), (386., 179.)], STEEL, 1.3);
        });
        d.text(56., 237., "KEY & FELT HAMMER", 10., MUTED);
        d.text(274., 237., "STRING & COIL", 10., MUTED);
        let plate = ::storytone::engine::plate(n).unwrap_or(0);
        d.text(
            454.,
            119.,
            &format!("PICKUP PLATE  {} / 8", plate + 1),
            11.,
            TEAL,
        );
        d.text(
            454.,
            144.,
            if plate < 4 {
                "T1  ·  PRIMARY A"
            } else {
                "T1  ·  PRIMARY B"
            },
            11.,
            TEXT,
        );
        d.text(454., 170., "6J7   →   6C5   →   4 × 2A3", 12., TEXT);
        d.text(454., 197., "15 Ω VOICE COIL", 11., MUTED);
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        for ((_, (x, y, w, _)), (label, value)) in CONTROLS.iter().take(3).zip([
            ("OPTIMUM", p.optimum.value()),
            ("MANUAL", p.manual.value()),
            ("SWELL", swell),
        ]) {
            d.text(*x + 6., *y + 11., label, 11., TEAL);
            d.text_right(
                *x + *w - 6.,
                *y + 11.,
                &format!("{:.0}%", value * 100.),
                12.,
                TEXT,
            );
            let pos = *x + 6. + (*w - 12.) * value;
            d.line(*x + 6., *y + 29., *x + *w - 6., *y + 29., LINE, 4.);
            d.panel((pos - 4., *y + 21., 8., 16.), TEAL, LINE);
        }
        d.button(CONTROLS[3].1, "Soft pedal", soft, GOLD);
        d.button(CONTROLS[4].1, "Damper pedal", sustain, GOLD);
    }
}
