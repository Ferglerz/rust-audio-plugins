//! Oblique cutaway of a reed. Lengths and motion are exaggerated
//! for visibility. The two fingers are on opposite sides of the free reed.
use super::Knob;
use nih_plug_vizia::vizia::vg::Color;
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, Callout, Point},
    point_in_rect,
    theme::rgb,
};
use std::f32::consts::TAU;

pub(super) const PANEL_RECT: (f32, f32, f32, f32) = (18.0, 62.0, super::WIDTH - 36.0, 194.0);
pub(super) const HEIGHT_PX_PER_MM: f32 = 72.;
pub(super) const HEIGHT_HELP: &str = "Hold a note and drag to raise or lower the pickup fingers. Double-click to restore the stock height.";

pub(super) struct Part {
    pub(super) knob: Knob,
    caption: Callout,
}

const HAMMER: Part = Part {
    knob: Knob::HammerHardness,
    caption: Callout {
        label: "HAMMER HARDNESS",
        origin: (104.0, 236.0),
        width: 78.0,
        elbow: (222.0, 240.0),
    },
};
const REED: Part = Part {
    knob: Knob::ReedDecay,
    caption: Callout {
        label: "REED DECAY",
        origin: (318.0, 108.0),
        width: 72.0,
        elbow: (332.0, 119.0),
    },
};
const PICKUP: Part = Part {
    knob: Knob::PickupHeight,
    caption: Callout {
        label: "PICKUP HEIGHT",
        origin: (521.0, 208.0),
        width: 119.0,
        elbow: (510.0, 219.0),
    },
};

// Height is positive up, depth is positive towards the player.
fn project(x: f32, height: f32, depth: f32) -> Point {
    (
        x + 0.46 * depth,
        166.0 - 0.055 * (x - 140.0) - height + 0.51 * depth,
    )
}

#[derive(Clone, Copy)]
pub(super) struct Motion {
    note: u8,
    comb_offset: f32,
    hammer_lift: f32,
    reed_swing: f32,
}

impl Motion {
    pub(super) fn new(elapsed: f32, note: u8, decay: f32, height_mm: f32, drive: f32) -> Self {
        // The bass pickup sits above the reed. Higher reeds sit above the pickup.
        Self {
            note,
            comb_offset: if openwurli_dsp::pickup::pickup_band_sign(note) > 0.0 {
                19.0
            } else {
                0.0
            } + height_mm * HEIGHT_PX_PER_MM,
            hammer_lift: action::hammer_lift(elapsed),
            reed_swing: 18.0 * drive * action::vibration(elapsed, decay.sqrt() / 3.0),
        }
    }

    fn reed_height(self, x: f32) -> f32 {
        // Cantilever deflection is zero in both position and slope at the clamp.
        let u = ((x - 164.0) / (463.0 - 164.0)).clamp(0.0, 1.0);
        self.reed_swing * u * u * (3.0 - u) * 0.5
    }
}

pub(super) fn hit(x: f32, y: f32, motion: Motion) -> Option<&'static Part> {
    if !point_in_rect(x, y, PANEL_RECT) {
        return None;
    }
    for part in [&HAMMER, &REED, &PICKUP] {
        if part.caption.hit(x, y) {
            return Some(part);
        }
    }
    let height = project(x, 0.0, 0.0).1 - y;
    let comb_height = height - motion.comb_offset;
    // The front finger can obscure the moving reed, especially in the bass.
    if (391.0..=499.0).contains(&x) && (-26.0..=-11.8).contains(&comb_height) {
        return Some(&PICKUP);
    }
    let reed_height = height - motion.reed_height(x);
    let on_blade = if x < 374.0 {
        (-8.0..=7.0).contains(&reed_height)
    } else {
        (-6.5..=4.0).contains(&reed_height)
    };
    if (95.0..=171.0).contains(&x) && (-20.0..=26.0).contains(&height)
        || (161.0..=467.0).contains(&x) && on_blade
    {
        return Some(&REED);
    }
    if (374.0..=590.0).contains(&x) && (-37.0..=12.0).contains(&comb_height) {
        return Some(&PICKUP);
    }
    let arm = (x - 111.0) / (235.0 - 111.0);
    let lift = 17.0 * motion.hammer_lift;
    let shank_height = -48.0 + arm * (4.0 + lift);
    let on_shank = (-0.10..=1.03).contains(&arm) && (height - shank_height).abs() < 9.0;
    let on_head = (211.0..=253.0).contains(&x) && (-51.0..=-11.0).contains(&(height - lift));
    (on_shank || on_head).then_some(&HAMMER)
}

fn block(d: &mut Draw, x: (f32, f32), h: (f32, f32), z: (f32, f32), color: Color) {
    let face = [(x.0, h.1), (x.1, h.1), (x.1, h.0), (x.0, h.0)].map(|(x, h)| project(x, h, z.1));
    let depth = z.0 - z.1;
    action::solid(d, face, (0.46 * depth, 0.51 * depth), color);
}

fn screw(d: &mut Draw, x: f32, h: f32, radius: f32) {
    let rim: [_; 24] = std::array::from_fn(|i| {
        let (s, c) = (TAU * i as f32 / 24.0).sin_cos();
        project(x + c * radius, h, s * radius)
    });
    d.fill_poly(&rim, rgb(160, 169, 174));
    let a = project(x - radius * 0.69, h, -radius * 0.25);
    let b = project(x + radius * 0.69, h, radius * 0.25);
    d.line(a.0, a.1, b.0, b.1, rgb(65, 71, 76), 1.4);
}

fn reed(d: &mut Draw, motion: Motion) {
    let steel = rgb(179, 195, 202);
    let edge = |i: usize, z: f32, thickness: f32| {
        let x = 129.0 + (463.0 - 129.0) * i as f32 / 12.0;
        project(x, motion.reed_height(x) + thickness, z)
    };
    let surface: [_; 26] = std::array::from_fn(|i| {
        if i < 13 {
            edge(i, -7.0, 0.0)
        } else {
            edge(25 - i, 7.0, 0.0)
        }
    });
    let lip: [_; 26] = std::array::from_fn(|i| {
        if i < 13 {
            edge(i, 7.0, 0.0)
        } else {
            edge(25 - i, 7.0, -2.5)
        }
    });
    d.fill_poly(&surface, steel);
    d.fill_poly(&lip, rgb(113, 123, 127));
    d.poly(&surface[..13], rgb(212, 220, 224), 0.8);
}

pub(super) fn draw(d: &mut Draw, motion: Motion, active: Option<Knob>) {
    action::panel(d, super::WIDTH, "REED ACTION", motion.note);
    let offset = motion.comb_offset;
    action::hardware(d, |m| {
        let comb = rgb(191, 171, 124);
        block(
            m,
            (101.0, 163.0),
            (-17.0, -2.5),
            (-18.0, 18.0),
            action::CASTING,
        );
        block(
            m,
            (494.0, 576.0),
            (-26.0 + offset, -13.0 + offset),
            (-22.0, 22.0),
            action::RUBBER,
        );
        block(
            m,
            (386.0, 495.0),
            (-12.0 + offset, -6.0 + offset),
            (-26.0, -11.0),
            comb,
        );
        action::hammer(
            m,
            (111.0, 214.0),
            (234.0, 183.0),
            motion.hammer_lift,
            action::FELT,
        );
        reed(m, motion);
        block(
            m,
            (111.0, 164.0),
            (0.0, 8.0),
            (-13.0, 13.0),
            action::CASTING,
        );
        screw(m, 135.0, 8.5, 8.0);
        block(
            m,
            (487.0, 577.0),
            (-13.0 + offset, -6.0 + offset),
            (-26.0, 26.0),
            comb,
        );
        block(
            m,
            (386.0, 487.0),
            (-12.0 + offset, -6.0 + offset),
            (11.0, 26.0),
            comb,
        );
        screw(m, 550.0, -5.8 + offset, 6.8);
    });
    for (part, anchor) in [
        (&HAMMER, (230.0, 188.0 - 17.0 * motion.hammer_lift)),
        (&REED, project(350.0, motion.reed_height(350.0), 0.0)),
        (&PICKUP, project(505.0, -12.0 + offset, 26.0)),
    ] {
        part.caption
            .draw(d, part.caption.label, anchor, active == Some(part.knob));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pleasant_ui::{
        instrument_action::StrikePreview,
        playable_keyboard::{KeyboardLink, PlayableKeyboard},
    };
    use std::sync::Arc;

    #[test]
    fn a_strike_selects_the_register_and_the_front_finger_stays_draggable() {
        let link = Arc::new(KeyboardLink::default());
        let keyboard = PlayableKeyboard::new(link.clone(), || {});
        let strike = StrikePreview::default();
        for (note, bass) in [(52, true), (53, false), (53, false)] {
            link.attack(note);
            assert!(strike.poll(&keyboard));
            assert_eq!(strike.note(), note);
            for time in [0.0, 0.08, 0.15, 0.28, 2.0] {
                let motion = Motion::new(time, strike.note(), 1.0, 0.0, 1.0);
                assert_eq!(motion.comb_offset > 0.0, bass);
                let (x, y) = project(425.0, -6.0 + motion.comb_offset, 18.0);
                assert_eq!(
                    hit(x, y, motion).map(|part| part.knob),
                    Some(Knob::PickupHeight)
                );
                let (x, y) = project(348.0, motion.reed_height(348.0), 0.0);
                assert_eq!(
                    hit(x, y, motion).map(|part| part.knob),
                    Some(Knob::ReedDecay)
                );
            }
        }
    }
}
