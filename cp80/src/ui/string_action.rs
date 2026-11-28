//! Side view of one key. String spacing, vibration and hammer travel are enlarged.
use crate::KeySpec;
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, Callout, Point, StrikePreview, CASTING, RUBBER, STEEL},
    point_in_rect,
    theme::rgb,
};
use std::f32::consts::PI;

const LEFT: f32 = 151.;
const BEARING: f32 = 547.;
const HAMMER: Callout = Callout {
    label: "HAMMER",
    origin: (212., 238.),
    width: 50.,
    elbow: (193., 242.),
};
const STRINGS: Callout = Callout {
    label: "STRINGS",
    origin: (270., 113.),
    width: 125.,
    elbow: (395., 117.),
};
const PIEZO: Callout = Callout {
    label: "PIEZO PICKUP",
    origin: (585., 216.),
    width: 91.,
    elbow: (571., 220.),
};

const WIRED_PIEZO: Callout = Callout {
    label: "WIRED PIEZO",
    origin: (628., 228.),
    width: 65.,
    elbow: (612., 232.),
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Part {
    Hammer,
    Strings,
    Piezo,
}

impl Part {
    pub fn help(self) -> &'static str {
        match self {
            Self::Hammer => "Click and hold the hammer to play the displayed note. The hammer rebounds after contact.",
            Self::Strings => "Bass notes use one wound string. Higher notes use pairs, with plain wire from E♭4 up.",
            Self::Piezo => "String force at the bearing loads one piezo per key, feeding the preamp.",
        }
    }
}

pub struct StringAction {
    note: u8,
    strings: usize,
    wound: bool,
    wired: bool,
    hammer_x: f32,
    lift: f32,
    swing: f32,
}

impl StringAction {
    pub fn new(strike: &StrikePreview) -> Self {
        let note = strike.note();
        Self::with_spec(strike, KeySpec::extended(note), note < 63)
    }

    pub fn cp70b(strike: &StrikePreview) -> Self {
        let note = strike.note();
        Self::with_spec(strike, crate::seventy::extended(note), note <= 64)
    }

    pub fn cp70(strike: &StrikePreview) -> Self {
        let note = strike.note();
        let spec = crate::seventy::extended_for(crate::seventy::Revision::Original, note);
        Self {
            wired: true,
            ..Self::with_spec(strike, spec, note <= 64)
        }
    }

    fn with_spec(strike: &StrikePreview, spec: KeySpec, wound: bool) -> Self {
        let note = strike.note();
        let t = strike.elapsed();
        Self {
            note,
            strings: spec.strings,
            wound,
            wired: false,
            hammer_x: LEFT + (BEARING - LEFT) * spec.strike_fraction as f32,
            lift: action::hammer_lift(t),
            swing: 15. * action::vibration(t, 0.68),
        }
    }

    fn wire(&self, u: f32, course: usize) -> Point {
        let x = LEFT + u * (BEARING - LEFT);
        let y = 182. - 32. * u - self.swing * (PI * u).sin();
        (x + course as f32 * 7., y - course as f32 * 10.)
    }

    fn tip(&self) -> Point {
        let u = (self.hammer_x - LEFT) / (BEARING - LEFT);
        (self.hammer_x, 182. - 32. * u + 22.)
    }

    pub fn hit(&self, x: f32, y: f32) -> Option<Part> {
        let (hx, hy) = self.tip();
        let along_shank = (x - (hx - 111.)) / 111.;
        let shank_y = hy + 22. - along_shank * 16. * self.lift;
        if HAMMER.hit(x, y)
            || point_in_rect(x, y, (hx - 18., hy - 22., 45., 54.))
            || ((0. ..=1.).contains(&along_shank) && (y - shank_y).abs() <= 6.)
        {
            return Some(Part::Hammer);
        }
        let callout = if self.wired { WIRED_PIEZO } else { PIEZO };
        if callout.hit(x, y)
            || point_in_rect(x, y, (518., 140., 86., 45.))
            || (self.wired && point_in_rect(x, y, (593., 162., 48., 40.)))
        {
            return Some(Part::Piezo);
        }
        let u = (x - LEFT) / (BEARING - LEFT);
        let on_wire = (0. ..=1.).contains(&u)
            && (0..self.strings).any(|i| (y - self.wire(u, i).1).abs() < 7.);
        (STRINGS.hit(x, y) || on_wire).then_some(Part::Strings)
    }

    pub fn draw(&self, d: &mut Draw, active: Option<Part>) {
        action::panel(d, super::SIZE.0, "STRING ACTION", self.note);
        let wound = self.wound;
        let tip = self.tip();
        action::hardware(d, |m| {
            let depth = (12., -12.);
            // Fixed supports, with the piezo sandwiched beneath the right bearing.
            action::solid(
                m,
                [
                    (LEFT - 25., 185.),
                    (LEFT + 8., 181.),
                    (LEFT + 8., 196.),
                    (LEFT - 25., 200.),
                ],
                depth,
                CASTING,
            );
            action::solid(
                m,
                [(511., 172.), (586., 166.), (586., 183.), (511., 189.)],
                depth,
                CASTING,
            );
            if self.wired {
                // Cantilever top arm, exposed piezo and individual return lead.
                m.poly(
                    &[
                        (524., 169.),
                        (524., 165.),
                        (541., 151.),
                        (549., 149.),
                        (576., 158.),
                    ],
                    STEEL,
                    3.,
                );
                action::solid(
                    m,
                    [(530., 166.), (560., 161.), (560., 167.), (530., 172.)],
                    depth,
                    rgb(202, 157, 87),
                );
                m.poly(
                    &[(558., 163.), (581., 169.), (594., 189.), (612., 190.)],
                    rgb(154, 125, 171),
                    2.,
                );
                m.poly(
                    &[(525., 170.), (576., 174.), (593., 196.), (613., 196.)],
                    rgb(115, 137, 114),
                    2.,
                );
                action::solid(
                    m,
                    [(611., 186.), (636., 183.), (636., 197.), (611., 200.)],
                    (3., -3.),
                    rgb(73, 112, 89),
                );
                for y in [190., 196.] {
                    m.line(613., y, 621., y - 1., rgb(207, 182, 117), 2.);
                }
            } else {
                action::solid(
                    m,
                    [(525., 166.), (574., 162.), (574., 169.), (525., 173.)],
                    depth,
                    rgb(202, 157, 87),
                );
                action::solid(
                    m,
                    [
                        (524., 165.),
                        (540., 150.),
                        (549., 149.),
                        (574., 161.),
                        (574., 163.),
                        (524., 167.),
                    ],
                    depth,
                    STEEL,
                );
                // Signal lead from the ceramic element.
                m.poly(
                    &[(574., 165.), (589., 163.), (604., 173.), (619., 172.)],
                    rgb(166, 105, 76),
                    2.,
                );
            }
            action::hammer(m, (tip.0 - 111., tip.1 + 22.), tip, self.lift, RUBBER);
            for course in (0..self.strings).rev() {
                let wire: [_; 33] = std::array::from_fn(|i| self.wire(i as f32 / 32., course));
                let color = if wound { rgb(201, 148, 98) } else { STEEL };
                m.poly(&wire, color, if wound { 4.0 } else { 2.2 });
                if wound {
                    for i in 1..88 {
                        let (x, y) = self.wire(i as f32 / 88., course);
                        m.line(x - 1., y - 1.4, x + 1., y + 1.4, rgb(123, 85, 61), 0.85);
                    }
                }
                let (x, y) = wire[32];
                m.line(x, y, x + 37., y + 7., color, 1.8);
            }
        });
        HAMMER.draw(
            d,
            HAMMER.label,
            (tip.0 + 2., tip.1 + 7. - 15. * self.lift),
            active == Some(Part::Hammer),
        );
        let description = match (self.strings, wound) {
            (1, _) => "ONE WOUND STRING",
            (_, true) => "TWO WOUND STRINGS",
            _ => "TWO PLAIN STRINGS",
        };
        STRINGS.draw(
            d,
            description,
            self.wire(0.62, 0),
            active == Some(Part::Strings),
        );
        let (callout, point) = if self.wired {
            (WIRED_PIEZO, (613., 196.))
        } else {
            (PIEZO, (554., 166.))
        };
        callout.draw(d, callout.label, point, active == Some(Part::Piezo));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pleasant_ui::playable_keyboard::{KeyboardLink, PlayableKeyboard};
    use std::sync::Arc;

    #[test]
    fn displayed_course_and_audition_follow_the_played_register() {
        let link = Arc::new(KeyboardLink::default());
        let keyboard = PlayableKeyboard::new(link.clone(), || {});
        let strike = StrikePreview::default();
        for (note, strings) in [(21, 1), (42, 1), (43, 2), (62, 2), (63, 2), (108, 2)] {
            link.attack(note);
            assert!(strike.poll(&keyboard));
            let action = StringAction::new(&strike);
            assert_eq!((action.note, action.strings), (note, strings));
            let smaller = StringAction::cp70b(&strike);
            assert_eq!(smaller.wound, note <= 64);
            assert_eq!(smaller.strings, if note <= 42 { 1 } else { 2 });
            let original = StringAction::cp70(&strike);
            assert_eq!(original.hit(620., 195.), Some(Part::Piezo));
            assert_eq!(original.wound, smaller.wound);
            let (x, y) = action.tip();
            assert!(action.hit(x, y + 5.) == Some(Part::Hammer));
            let (x, y) = action.wire(0.5, 0);
            assert!(action.hit(x, y) == Some(Part::Strings));
            assert!(action.hit(554., 166.) == Some(Part::Piezo));
        }
    }
}
