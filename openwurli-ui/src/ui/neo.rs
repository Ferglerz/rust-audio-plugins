//! The primary stops below the string. The lighter hammer completes the strike in free flight.
use super::{set, toggle, EventContext, Instrument, Knob, PianoView, Rect, Target, WIDTH};
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, FELT, RUBBER, STEEL, WOOD},
    point_in_rect,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Master(u8),
    Swell,
    Felt,
    Sustain,
}
impl Part {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Master(i) => (32. + f32::from(i - 1) * 28., 300., 26., 24.),
            Self::Swell => (403., 278., 280., 49.),
            Self::Felt => (32., 347., 202., 34.),
            Self::Sustain => (250., 347., 202., 34.),
        }
    }
    pub(super) fn level(self) -> Option<Knob> {
        (self == Self::Swell).then_some(Knob::NeoSwell)
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (l, _, w, _) = self.rect();
        ((x - l - 7.) / (w - 14.)).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self {
            Self::Master(_) => "Select one of the Siemens amplifier’s twelve volume steps. Step 1 grounds the wiper.",
            Self::Swell => "Move the left pedal to vary the input coupling capacitance. MIDI CC11 also operates this pedal.",
            Self::Felt => "Bring the light felt rail against the strings. It shortens their decay while keys are down or the sustain pedal is held.",
            Self::Sustain => "Lift the key dampers so released strings can ring. MIDI CC64 also operates this pedal.",
        }
    }
}
impl PianoView {
    pub(super) fn neo_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (44., 103., 375., 120.)) {
            return Some(Target::HohnerKey);
        }
        (1..=12)
            .map(Part::Master)
            .chain([Part::Swell, Part::Felt, Part::Sustain])
            .find(|p| point_in_rect(x, y, p.rect()))
            .map(Target::Neo)
    }
    pub(super) fn neo_press(&mut self, cx: &mut EventContext, part: Part) {
        let p = &self.params.neo;
        match part {
            Part::Master(n) => set(cx, &p.master, i32::from(n)),
            Part::Felt => toggle(cx, &p.felt),
            Part::Sustain => toggle(cx, &p.sustain),
            Part::Swell => {}
        }
    }
    pub(super) fn paint_neo(&self, d: &mut Draw) {
        let p = &self.params.neo;
        let (swell, felt, sustain) = p.display();
        let strike = &self.strikes[Instrument::Neo.index()];
        let n = strike.note();
        let age = strike.elapsed();
        let down = self.keyboard.is_down(n);
        let main = action::hammer_lift(age);
        let secondary = if age < 0.064 {
            0.
        } else {
            action::hammer_lift((age - 0.064) * 1.7)
        };
        let vibration = if down || sustain {
            action::vibration(age, if felt { 0.13 } else { 0.85 })
        } else {
            0.
        };
        let strings = neo_bechstein::action::strings_at(n);
        let head = neo_bechstein::pickup::head(n);
        action::panel(d, WIDTH, "NEO-BECHSTEIN  ·  MICROHAMMER ACTION", n);
        action::hardware(d, |d| {
            // Hammer, stop and flying striker in side elevation below the horizontal course.
            for i in 0..strings {
                let y = 126. + i as f32 * 3.;
                d.poly(
                    &[
                        (53., y),
                        (130., y + 1.3 * vibration),
                        (228., y + 3.4 * vibration),
                        (321., y + 2. * vibration),
                        (411., y),
                    ],
                    STEEL,
                    1.8,
                );
            }
            let lever = 197. - main * 24.;
            d.line(70., 207., 201., lever, WOOD, 6.);
            d.circle(201., lever - 5., 9., FELT, true);
            action::screw(d, 70., 207., 5.);
            d.line(171., 170., 171., 178., CASTING, 5.);
            d.line(164., 178., 180., 178., RUBBER, 5.);
            // Nernst fig. 2: a tapered vertical pin slides in a stationary sleeve.
            // The large hammer stops while the pin and its felt crown continue alone.
            let heel = 183. - 12. * main - 17. * secondary;
            d.line(201., heel, 201., heel - 23., WOOD, 3.);
            d.square_rect(192., 158., 6., 9., CASTING);
            d.square_rect(204., 158., 6., 9., CASTING);
            d.circle(201., heel - 25., 4.7, FELT, true);
            let lifted = if down || sustain { 10. } else { 0. };
            d.square_rect(282., 111. - lifted, 19., 6., WOOD);
            d.square_rect(282., 117. - lifted, 19., 8., FELT);
            d.square_rect(342., if felt { 129. } else { 140. }, 23., 6., FELT);
            let gap_y = 134. + 3.5 * self.params.regulation.neo.value();
            d.poly(
                &[
                    (377., gap_y),
                    (377., gap_y + 21.),
                    (400., gap_y + 21.),
                    (400., gap_y),
                ],
                CASTING,
                5.,
            );
            for i in 0..7 {
                let x = 378. + i as f32 * 3.;
                d.line(x, gap_y + 16., x + 2., gap_y + 25., WOOD, 1.3);
            }
        });
        d.text(55., 231., "MAIN HAMMER & STOP", 10., MUTED);
        d.text(207., 231., "MICROHAMMER", 10., MUTED);
        d.text(334., 231., "FELT & PICKUP", 10., MUTED);
        d.text(
            453.,
            117.,
            &format!("HORSESHOE PICKUP   {} / 18", head + 1),
            12.,
            TEAL,
        );
        d.text(
            453.,
            143.,
            if strings == 2 {
                "PAIRED STRING COURSE"
            } else {
                "SINGLE STRING COURSE"
            },
            11.,
            TEXT,
        );
        d.text(453., 173., "SIEMENS ELV413", 12., TEXT);
        d.text(453., 197., "2 × RE604   ·   6 Ω SPEAKER", 11., MUTED);
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        d.text(33., 289., "AMPLIFIER STEP", 11., TEAL);
        for i in 1..=12 {
            d.button(
                Part::Master(i).rect(),
                &i.to_string(),
                p.master.value() == i32::from(i),
                GOLD,
            );
        }
        let (x, y, w, _) = Part::Swell.rect();
        d.text(x + 7., y + 11., "SWELL PEDAL", 11., TEAL);
        d.text_right(
            x + w - 7.,
            y + 11.,
            &format!("{:.0}%", 100. * swell),
            12.,
            TEXT,
        );
        d.line(x + 7., y + 32., x + w - 7., y + 32., LINE, 4.);
        d.panel(
            (x + 7. + (w - 14.) * swell - 4., y + 24., 8., 16.),
            TEAL,
            LINE,
        );
        d.button(Part::Felt.rect(), "Felt rail", felt, GOLD);
        d.button(Part::Sustain.rect(), "Damper pedal", sustain, GOLD);
    }
}
