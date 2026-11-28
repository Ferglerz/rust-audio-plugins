//! Duo key linkage, keyboard selector, two branch sliders and recording routing.
use super::{set, toggle, EventContext, Instrument, Knob, PianoView, Pickup, Rect, Target, WIDTH};
use crate::duo::Registration;
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, RUBBER, STEEL, WOOD},
    point_in_rect,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Split(usize),
    Two,
    Three,
    Foot,
    Pickup(usize),
    Tone(usize),
    Mute,
    Power,
    Recording,
    ClavVolume,
    ReedVolume,
}
impl Part {
    pub(super) fn level(self) -> Option<Knob> {
        match self {
            Self::ClavVolume => Some(Knob::DuoClavVolume),
            Self::ReedVolume => Some(Knob::DuoPianetVolume),
            _ => None,
        }
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        ((x - SLIDER_X) / SLIDER_W).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self {
            Self::Split(0) => "Use both full pickup banks. The two registration tabs decide which branches the footswitch can silence.",
            Self::Split(1) => "Clavinet through C3. Pianet from C♯3 upward.",
            Self::Split(2) => "Pianet through C3. Clavinet from C♯3 upward.",
            Self::Split(_) => "Pianet across the keyboard, with Clavinet added from C♯3 upward.",
            Self::Two => "Left rocker. Position 1 lets pedal A silence the Pianet. Position 2 keeps its output connected.",
            Self::Three => "Right rocker. Position 1 lets pedal B silence the Clavinet. Position 3 keeps its output connected.",
            Self::Foot => "Change the footswitch contact. MIDI CC64 can hold contact B. Both forward tabs disconnect the pedal.",
            Self::Pickup(_) => "Choose either string pickup or a parallel pair. The reed pickup remains independent.",
            Self::Tone(_) => "Switch a section of the Clavinet tone circuit. This affects only the string branch.",
            Self::Mute => "Bring the felt mute against the strings. The Pianet reeds continue ringing.",
            Self::Power => "Switch the preamp power. The key, string and suction pad continue moving.",
            Self::Recording => "Select the combined mono jack or the separated recording sockets, Clavinet left and Pianet right.",
            Self::ClavVolume => "Drag to set the Clavinet's logarithmic volume slider from 0 to 5.",
            Self::ReedVolume => "Drag to set the Pianet's logarithmic volume slider from 0 to 5.",
        }
    }
}
const SLIDER_X: f32 = 510.;
const SLIDER_W: f32 = 168.;
const KEY: Rect = (61., 105., 391., 98.);
const TWO: Rect = (32., 312., 83., 28.);
const THREE: Rect = (120., 312., 83., 28.);
const FOOT: Rect = (209., 312., 90., 28.);
const POWER: Rect = (590., 273., 103., 28.);
const MUTE: Rect = (438., 357., 102., 28.);
const RECORD: Rect = (546., 357., 147., 28.);
fn split(i: usize) -> Rect {
    (32. + i as f32 * 138., 273., 133., 28.)
}
fn pickup(i: usize) -> Rect {
    (310. + i as f32 * 97., 312., 92., 28.)
}
fn tone(i: usize) -> Rect {
    (32. + i as f32 * 100., 357., 95., 28.)
}
pub(super) fn slider(y: f32) -> Rect {
    (SLIDER_X - 5., y - 24., SLIDER_W + 10., 39.)
}

impl PianoView {
    pub(super) fn duo_hit(&self, x: f32, y: f32) -> Option<Target> {
        let on = |r| point_in_rect(x, y, r);
        if on(KEY) {
            return Some(Target::HohnerKey);
        }
        for (r, p) in [
            (TWO, Part::Two),
            (THREE, Part::Three),
            (FOOT, Part::Foot),
            (POWER, Part::Power),
            (MUTE, Part::Mute),
            (RECORD, Part::Recording),
            (slider(142.), Part::ClavVolume),
            (slider(205.), Part::ReedVolume),
        ] {
            if on(r) {
                return Some(Target::Duo(p));
            }
        }
        for i in 0..4 {
            for (r, p) in [
                (split(i), Part::Split(i)),
                (pickup(i), Part::Pickup(i)),
                (tone(i), Part::Tone(i)),
            ] {
                if on(r) {
                    return Some(Target::Duo(p));
                }
            }
        }
        None
    }

    pub(super) fn duo_press(&mut self, cx: &mut EventContext, part: Part) {
        let p = &self.params.duo;
        match part {
            Part::Split(i) => set(cx, &p.split, Registration::ALL[i]),
            Part::Two => toggle(cx, &p.two),
            Part::Three => toggle(cx, &p.three),
            Part::Foot => toggle(cx, &p.foot),
            Part::Pickup(i) => set(cx, &p.pickup, Pickup::ALL[i]),
            Part::Tone(i) => toggle(cx, p.tabs()[i]),
            Part::Power => toggle(cx, &p.power),
            Part::Mute => toggle(cx, &p.mute),
            Part::Recording => toggle(cx, &p.recording),
            Part::ClavVolume | Part::ReedVolume => {}
        }
    }

    pub(super) fn paint_duo(&self, d: &mut Draw) {
        let p = &self.params.duo;
        let strike = &self.strikes[Instrument::Duo.index()];
        let n = strike.note();
        let t = strike.elapsed();
        let down = self.keyboard.is_down(n);
        let dip = if down { (t / 0.08).min(1.) } else { 0. };
        let ring = if down { action::vibration(t, 0.32) } else { 0. };
        let caught = self.params.sustain_mod.pad_is_caught(n);
        let pad_lift = if caught { 1. } else { dip };
        let reed_ring = if caught {
            action::vibration(t, 0.4)
        } else {
            ring
        };
        action::panel(d, WIDTH, "PIANET / CLAVINET DUO", n);
        action::hardware(d, |d| {
            let left = 115. + 9. * dip;
            let right = 127. - 11. * dip;
            action::solid(
                d,
                [
                    (72., left),
                    (394., right),
                    (394., right + 7.),
                    (72., left + 7.),
                ],
                (6., -4.),
                WOOD,
            );
            action::screw(d, 244., 126., 5.);
            let tip = 163. + 4. * dip;
            d.line(155., left + 7., 155., tip - 6., CASTING, 3.);
            action::solid(
                d,
                [(148., tip - 6.), (162., tip - 6.), (162., tip), (148., tip)],
                (4., -2.),
                RUBBER,
            );
            action::solid(
                d,
                [(144., 175.), (167., 175.), (167., 183.), (144., 183.)],
                (3., -2.),
                STEEL,
            );
            d.poly(
                &[
                    (51., 171.),
                    (100., 171. + 4. * ring),
                    (155., 170. + 4. * dip),
                    (211., 171.),
                ],
                STEEL,
                2.,
            );
            for i in 0..9 {
                let x = 176. + i as f32 * 3.8;
                d.line(x - 1., 168., x + 1., 174., WOOD, 1.7);
            }
            let pad = 168. - 18. * pad_lift;
            d.line(376., right + 6., 376., pad - 5., CASTING, 3.);
            action::solid(
                d,
                [(364., pad - 6.), (387., pad - 6.), (389., pad), (362., pad)],
                (4., -2.),
                RUBBER,
            );
            d.poly(
                &[(458., 170.), (421., 170.), (378., 169. + 5. * reed_ring)],
                STEEL,
                3.,
            );
            action::solid(
                d,
                [(449., 164.), (465., 164.), (465., 177.), (449., 177.)],
                (4., -3.),
                CASTING,
            );
        });
        d.text(60., 107., "SHARED KEY", 10., TEXT);
        d.text(99., 207., "TANGENT & ANVIL", 10., MUTED);
        d.text(312., 207., "SUCTION PAD & REED", 10., MUTED);
        let bass = n <= hohner_duo::LAST_BASS;
        let split = p.split.value();
        let (controls, separate_outputs) = p.display_state();
        let [clav, reed] = controls.selected_branches(n);
        let foot = controls.foot;
        for (y, label, v, a) in [
            (142., "CLAVINET", p.clavinet_volume.value(), clav),
            (205., "PIANET", p.pianet_volume.value(), reed),
        ] {
            let tint = if a { TEAL } else { MUTED };
            d.text(SLIDER_X, y - 13., label, 11., tint);
            d.text_right(
                SLIDER_X + SLIDER_W,
                y - 13.,
                &format!("{:.1}", v * 5.),
                12.,
                TEXT,
            );
            d.line(SLIDER_X, y, SLIDER_X + SLIDER_W, y, LINE, 4.);
            let x = SLIDER_X + SLIDER_W * v;
            d.panel((x - 4., y - 8., 8., 16.), tint, LINE);
        }
        d.text(
            32.,
            239.,
            if bass {
                "BASS · F1–C3"
            } else {
                "TREBLE · C♯3–E6"
            },
            11.,
            MUTED,
        );
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        for (i, reg) in Registration::ALL.into_iter().enumerate() {
            d.tab_button(
                self::split(i),
                [
                    "Preset",
                    "Clav low / Pianet high",
                    "Pianet low / Clav high",
                    "Pianet low / Both high",
                ][i],
                reg == split,
                GOLD,
            );
            d.button(
                pickup(i),
                ["CA · Centre", "CB · Bridge", "DA · Both", "DB · Opposed"][i],
                p.pickup.value() == Pickup::ALL[i],
                TEAL,
            );
            d.button(
                tone(i),
                ["Soft", "Medium", "Treble", "Brilliant"][i],
                p.tabs()[i].value(),
                TEAL,
            );
        }
        d.button(
            TWO,
            if p.two.value() { "Tab 2" } else { "Tab 1" },
            p.two.value(),
            GOLD,
        );
        d.button(
            THREE,
            if p.three.value() { "Tab 3" } else { "Tab 1" },
            p.three.value(),
            GOLD,
        );
        d.button(FOOT, if foot { "Pedal B" } else { "Pedal A" }, foot, GOLD);
        d.toggle_switch(POWER, "POWER", p.power.value(), TEAL);
        d.toggle_switch(MUTE, "MUTE", p.mute.value(), TEAL);
        d.button(
            RECORD,
            if separate_outputs {
                "Recording · C / P"
            } else {
                "Combined jack"
            },
            separate_outputs,
            GOLD,
        );
    }
}
