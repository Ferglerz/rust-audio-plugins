//! Tangent contact, gel damping, two magnetic rails and the Vibanet controls.
use super::{set, toggle, EventContext, Instrument, Knob, PianoView, Rect, Target, WIDTH};
use crate::vibanet::SELECTORS;
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, RUBBER, STEEL, WOOD},
    point_in_rect,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Pickup(u8),
    Eq(usize, i8),
    Wah,
    Volume,
    Sensitivity,
    Mute,
}
impl Part {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Pickup(i) => (
                449. + 116. * f32::from(i % 2),
                139. + 33. * f32::from(i / 2),
                109.,
                28.,
            ),
            Self::Eq(band, step) => (
                91. + f32::from(step + 2) * 52.,
                278. + band as f32 * 35.,
                48.,
                27.,
            ),
            Self::Wah => (390., 279., 107., 29.),
            Self::Sensitivity => (511., 273., 169., 43.),
            Self::Volume => (390., 333., 130., 47.),
            Self::Mute => (547., 333., 133., 47.),
        }
    }
    pub(super) fn level(self) -> Option<Knob> {
        match self {
            Self::Volume => Some(Knob::VibanetVolume),
            Self::Sensitivity => Some(Knob::VibanetSensitivity),
            Self::Mute => Some(Knob::VibanetMute),
            _ => None,
        }
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (l, _, w, _) = self.rect();
        ((x - l - 6.) / (w - 12.)).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self {
            Self::Pickup(0) => "Solo the pickup near the bridge.",
            Self::Pickup(1) => "Solo the pickup nearer the middle of the string.",
            Self::Pickup(2) => "Combine both pickups in phase.",
            Self::Pickup(_) => "Combine both pickups with opposite polarity.",
            Self::Eq(_, _) => "Select a fixed 7.5 dB step on the five-position equalizer switch.",
            Self::Wah => "Switch the envelope-controlled filter into the instrument output.",
            Self::Sensitivity => "Adjust how strongly the pickup signal opens the wah filter.",
            Self::Mute => "Move the mute bar onto the speaking strings to shorten their decay.",
            Self::Volume => "Set the Vibanet output level.",
        }
    }
}
impl PianoView {
    pub(super) fn vibanet_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (42., 100., 370., 131.)) {
            return Some(Target::HohnerKey);
        }
        (0..4)
            .map(Part::Pickup)
            .chain((0..3).flat_map(|band| (-2..=2).map(move |step| Part::Eq(band, step))))
            .chain([Part::Volume, Part::Mute, Part::Wah, Part::Sensitivity])
            .find(|part| point_in_rect(x, y, part.rect()))
            .map(Target::Vibanet)
    }
    pub(super) fn vibanet_press(&mut self, cx: &mut EventContext, part: Part) {
        let p = &self.params.vibanet;
        match part {
            Part::Pickup(i) => set(cx, &p.selector, i32::from(i)),
            Part::Eq(i, step) => set(cx, [&p.low, &p.mid, &p.high][i], i32::from(step)),
            Part::Wah => toggle(cx, &p.wah),
            _ => {}
        }
    }
    pub(super) fn paint_vibanet(&self, d: &mut Draw) {
        let c = self.params.vibanet.controls();
        let strike = &self.strikes[Instrument::Vibanet.index()];
        let note = strike.note();
        let down = self.keyboard.is_down(note);
        let age = strike.elapsed();
        let lift = if down { (age / 0.016).min(1.) } else { 0. };
        let ring = if down {
            action::vibration(age, 0.42) * 2.8
        } else {
            0.
        };
        action::panel(d, WIDTH, "VIBANET  ·  TANGENT & CLAVIGEL", note);
        action::hardware(d, |d| {
            // The tuning pins and gel lie beyond the tangent on the afterlength.
            action::screw(d, 59., 151., 4.);
            d.square_rect(166., 134., 14., 12., CASTING);
            for y in [145., 151.] {
                d.square_rect(74., y, 52., 4., TEAL);
            }
            let contact = 152. - lift * 4.;
            d.poly(
                &[
                    (59., 150.),
                    (106., 150.),
                    (173., contact),
                    (232., 150. + ring),
                    (289., 150. + ring * 0.6),
                    (408., 150.),
                ],
                STEEL,
                1.7,
            );
            d.line(64., 205., 173., 185. - lift * 17., WOOD, 7.);
            action::screw(d, 64., 205., 4.);
            d.square_rect(167., 165. - lift * 15., 12., 14., RUBBER);
            let centre = 165. + self.params.regulation.vibanet_centre.value() * 4.;
            let bridge = 128. - self.params.regulation.vibanet_bridge.value() * 4.;
            for (x, y) in [(273., centre), (376., bridge)] {
                d.panel((x, y, 24., 11.), CASTING, LINE);
                for i in 0..6 {
                    let xx = x + 3. + 3. * i as f32;
                    d.line(xx, y + 1., xx + 1., y + 10., WOOD, 1.);
                }
            }
            d.square_rect(341., 172. - c.mute as f32 * 20., 13., 8., RUBBER);
            action::screw(d, 408., 150., 4.);
        });
        d.text(70., 228., "GEL", 10., TEAL);
        d.text(139., 228., "TANGENT", 10., MUTED);
        d.text(254., 228., "CENTRE", 10., MUTED);
        d.text(360., 228., "BRIDGE", 10., MUTED);
        d.text(450., 123., "PICKUP SELECTOR", 12., TEAL);
        for (i, (selector, label)) in SELECTORS.iter().enumerate() {
            d.button(
                Part::Pickup(i as u8).rect(),
                label,
                *selector == c.pickup,
                GOLD,
            );
        }
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        for (i, label) in ["BASS", "MID", "TREBLE"].iter().enumerate() {
            d.text(32., 295. + i as f32 * 35., label, 10., TEAL);
            for (step, text) in (-2..=2).zip(["−15", "−7.5", "0 dB", "+7.5", "+15"]) {
                d.button(Part::Eq(i, step).rect(), text, c.eq[i] == step, GOLD);
            }
        }
        d.button(Part::Wah.rect(), "Auto-wah", c.wah, GOLD);
        for (part, value, label) in [
            (Part::Sensitivity, c.sensitivity, "SENSITIVITY"),
            (Part::Volume, c.volume, "VOLUME"),
            (Part::Mute, c.mute, "MUTE BAR"),
        ] {
            let (x, y, w, _) = part.rect();
            d.text(x + 6., y + 11., label, 10., TEAL);
            d.text_right(
                x + w - 6.,
                y + 11.,
                &format!("{:.0}%", value * 100.),
                10.,
                TEXT,
            );
            d.line(x + 6., y + 31., x + w - 6., y + 31., LINE, 3.);
            d.panel(
                (x + 3. + (w - 12.) * value as f32, y + 23., 6., 16.),
                TEAL,
                LINE,
            );
        }
    }
}
