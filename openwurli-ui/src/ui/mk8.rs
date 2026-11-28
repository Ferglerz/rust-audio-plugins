//! MK8 tine-to-pole voicing and the standard stereo control panel.
use super::{set, toggle, EventContext, Instrument, Knob, PianoView, Rect, Target, WIDTH};
use nih_plug::prelude::Param;
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, FELT, RUBBER, STEEL, WOOD},
    point_in_rect,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Volume,
    Drive,
    Envelope,
    Low,
    Mid,
    High,
    Frequency,
    Resonance,
    Enable,
    Wave,
    Rate,
    Depth,
    Sustain,
    Board,
    Preamp,
    Effects,
    Alternate,
    Comp,
    Chorus,
    Phaser,
    Delay,
    Tap,
    Fx(FxKnob),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FxKnob {
    Amount,
    Makeup,
    ChorusRate,
    ChorusDepth,
    PhaserRate,
    PhaserDepth,
    Time,
    Feedback,
    Mix,
    Ratio,
    ChorusTime,
    PhaserFreq,
    Spread,
    Jitter,
    DriveOut,
    Send,
}
impl FxKnob {
    pub(super) fn param(self, p: &crate::mk8::Mk8FxTone) -> &nih_plug::prelude::FloatParam {
        match self {
            Self::Amount => &p.amount,
            Self::Makeup => &p.makeup,
            Self::ChorusRate => &p.chorus_rate,
            Self::ChorusDepth => &p.chorus_depth,
            Self::PhaserRate => &p.phaser_rate,
            Self::PhaserDepth => &p.phaser_depth,
            Self::Time => &p.time,
            Self::Feedback => &p.feedback,
            Self::Mix => &p.mix,
            Self::Ratio => &p.ratio,
            Self::ChorusTime => &p.chorus_time,
            Self::PhaserFreq => &p.phaser_freq,
            Self::Spread => &p.spread,
            Self::Jitter => &p.jitter,
            Self::DriveOut => &p.drive_out,
            Self::Send => &p.send,
        }
    }
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Amount => "AMOUNT",
            Self::Makeup => "MAKEUP",
            Self::ChorusRate => "CHORUS RATE",
            Self::ChorusDepth => "CHORUS DEPTH",
            Self::PhaserRate => "PHASER RATE",
            Self::PhaserDepth => "PHASER DEPTH",
            Self::Time => "DELAY TIME",
            Self::Feedback => "FEEDBACK",
            Self::Mix => "DELAY MIX",
            Self::Ratio => "COMP RATIO",
            Self::ChorusTime => "CHORUS TIME",
            Self::PhaserFreq => "PHASER FREQ",
            Self::Spread => "DELAY SPREAD",
            Self::Jitter => "DELAY JITTER",
            Self::DriveOut => "DRIVE OUTPUT",
            Self::Send => "FX SEND",
        }
    }
    pub(super) fn help(self) -> &'static str {
        match self{
            Self::Amount => "Adjust the threshold of the compressor detector.",
            Self::Makeup => "Raise the compressor output level feeding Drive.",
            Self::ChorusRate => "Adjust the clock modulation speed of the two chorus lines.",
            Self::ChorusDepth => "Adjust the excursion of the chorus delay clocks.",
            Self::PhaserRate => "Adjust the speed of the four-stage phaser sweep.",
            Self::PhaserDepth => "Adjust the excursion of the two swept phaser notches.",
            Self::Time => "Set the bucket-brigade delay from 60 to 800 ms. Moving this slider replaces the tapped time.",
            Self::Feedback => "Return part of each delayed signal to its input. High settings sustain the echoes.",
            Self::Mix => "Blend the direct and delayed stereo signals.",
            Self::Ratio => "Change the compressor gain reduction above the threshold.",
            Self::ChorusTime => "Shift the center delay of the chorus sweep.",
            Self::PhaserFreq => "Shift both phaser notches up or down.",
            Self::Spread => "Offset the two delay clocks for a wider stereo image.",
            Self::Jitter => "Randomly modulate the delay clocks for wobble and shimmer.",
            Self::DriveOut => "Adjust the level from the diode drive into the equalizer.",
            Self::Send => "Adjust the level from the equalizer into the onboard effects.",}
    }
    pub(super) fn rect(self) -> Rect {
        fx_rect(match self {
            Self::Amount => 1,
            Self::Makeup => 2,
            Self::ChorusRate => 4,
            Self::ChorusDepth => 5,
            Self::PhaserRate => 8,
            Self::PhaserDepth => 9,
            Self::Time => 11,
            Self::Feedback => 12,
            Self::Mix => 13,
            Self::Ratio => 0,
            Self::ChorusTime => 1,
            Self::PhaserFreq => 2,
            Self::Spread => 3,
            Self::Jitter => 4,
            Self::DriveOut => 5,
            Self::Send => 6,
        })
    }
}
fn fx_rect(i: usize) -> Rect {
    (
        28. + (i % 7) as f32 * 97.,
        276. + (i / 7) as f32 * 59.,
        91.,
        46.,
    )
}
use FxKnob::*;
const EFFECTS: [Part; 14] = [
    Part::Comp,
    Part::Fx(Amount),
    Part::Fx(Makeup),
    Part::Chorus,
    Part::Fx(ChorusRate),
    Part::Fx(ChorusDepth),
    Part::Tap,
    Part::Phaser,
    Part::Fx(PhaserRate),
    Part::Fx(PhaserDepth),
    Part::Delay,
    Part::Fx(Time),
    Part::Fx(Feedback),
    Part::Fx(Mix),
];
const ALTERNATE: [Part; 7] = [
    Part::Fx(Ratio),
    Part::Fx(ChorusTime),
    Part::Fx(PhaserFreq),
    Part::Fx(Spread),
    Part::Fx(Jitter),
    Part::Fx(DriveOut),
    Part::Fx(Send),
];
const TABS: [Part; 3] = [Part::Preamp, Part::Effects, Part::Alternate];

const PARTS: [Part; 13] = [
    Part::Volume,
    Part::Drive,
    Part::Envelope,
    Part::Low,
    Part::Mid,
    Part::High,
    Part::Frequency,
    Part::Resonance,
    Part::Enable,
    Part::Wave,
    Part::Rate,
    Part::Depth,
    Part::Sustain,
];
impl Part {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Sustain => return (460., 199., 191., 30.),
            Self::Board => return (455., 110., 201., 29.),
            Self::Preamp => return (455., 152., 68., 27.),
            Self::Effects => return (526., 152., 65., 27.),
            Self::Alternate => return (594., 152., 67., 27.),
            Self::Fx(k) => return k.rect(),
            Self::Comp => return fx_rect(0),
            Self::Chorus => return fx_rect(3),
            Self::Tap => return fx_rect(6),
            Self::Phaser => return fx_rect(7),
            Self::Delay => return fx_rect(10),
            _ => {}
        }
        let i = PARTS.iter().position(|p| *p == self).unwrap();
        (
            30. + (i % 6) as f32 * 113.,
            276. + (i / 6) as f32 * 59.,
            104.,
            46.,
        )
    }
    pub(super) fn level(self) -> Option<Knob> {
        Some(match self {
            Self::Fx(k) => Knob::Mk8Fx(k),
            Self::Volume => Knob::Mk8Volume,
            Self::Drive => Knob::Mk8Drive,
            Self::Envelope => Knob::Mk8Envelope,
            Self::Low => Knob::Mk8Low,
            Self::Mid => Knob::Mk8Mid,
            Self::High => Knob::Mk8High,
            Self::Frequency => Knob::Mk8Frequency,
            Self::Resonance => Knob::Mk8Resonance,
            Self::Rate => Knob::Mk8Rate,
            Self::Depth => Knob::Mk8Depth,
            _ => return None,
        })
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (l, _, w, _) = self.rect();
        let v = ((x - l - 5.) / (w - 10.)).clamp(0., 1.);
        if self == Self::Fx(Time) {
            v * (800. - 60.) / (2000. - 60.)
        } else {
            v
        }
    }
    pub(super) fn help(self) -> &'static str {
        match self {
            Self::Board => "Choose the standard MK8 S or the factory-fitted MK8 FX board.",
            Self::Preamp => "Show the MK8 preamp, equalizer and stereo Vari-Pan controls.",
            Self::Effects => "Show the factory FX board controls.",
            Self::Alternate => "Show the board's alternative-mode adjustments.",
            Self::Fx(k) => k.help(),
            Self::Comp => "Switch the VCA compressor before the diode drive.",
            Self::Chorus => "Switch the stereo bucket-brigade chorus.",
            Self::Phaser => "Switch the four-stage stereo phaser.",
            Self::Delay => {
                "Switch the stereo delay. Existing repeats continue after switching off."
            }
            Self::Tap => {
                "Tap three or more times to set the delay. Tap times can exceed the dial range."
            }
            Self::Volume => "Set the master volume at the end of the stereo signal path.",
            Self::Drive => "Increase the signal into the preamp's diode drive.",
            Self::Envelope => {
                "Sweep the resonant mid band with the level of the keys. Raise Mid for touch wah."
            }
            Self::Low => "Boost or cut the band centered at 75 Hz.",
            Self::Mid => "Boost or cut the voltage-controlled mid band.",
            Self::High => "Boost or cut the band centered at 3 kHz.",
            Self::Frequency => "Set the frequency of the mid band from 100 Hz to 2 kHz.",
            Self::Resonance => "Adjust the bandwidth of the swept midrange filter.",
            Self::Enable => "Switch the stereo Vari-Pan circuit on or off.",
            Self::Wave => {
                "Cycle the Vari-Pan waveform through square, rising ramp, triangle and sine."
            }
            Self::Rate => "Set the Vari-Pan oscillator between 0.1 Hz and 3 kHz.",
            Self::Depth => "Set how far the signal moves between the left and right outputs.",
            Self::Sustain => "Lift the wool dampers. MIDI CC64 operates the same pedal.",
        }
    }
}
impl PianoView {
    pub(super) fn mk8_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (36., 101., 396., 134.)) {
            return Some(Target::HohnerKey);
        }
        let fx = &self.params.mk8.fx;
        let controls = match fx.page() {
            1 => &EFFECTS[..],
            2 => &ALTERNATE[..],
            _ => &PARTS[..],
        };
        let tabs = if fx.fitted.value() { &TABS[..] } else { &[] };
        [Part::Board, Part::Sustain]
            .into_iter()
            .chain(tabs.iter().copied())
            .chain(controls.iter().copied())
            .find(|p| point_in_rect(x, y, p.rect()))
            .map(Target::Mk8)
    }
    pub(super) fn mk8_press(&mut self, cx: &mut EventContext, part: Part) {
        let p = &self.params.mk8;
        match part {
            Part::Board => {
                p.fx.set_page(if p.fx.fitted.value() { 0 } else { 1 });
                toggle(cx, &p.fx.fitted)
            }
            Part::Preamp => p.fx.set_page(0),
            Part::Effects => p.fx.set_page(1),
            Part::Alternate => p.fx.set_page(2),
            Part::Comp => toggle(cx, &p.fx.compressor),
            Part::Chorus => toggle(cx, &p.fx.chorus),
            Part::Phaser => toggle(cx, &p.fx.phaser),
            Part::Delay => toggle(cx, &p.fx.delay),
            Part::Tap => {
                if let Some(ms) = p.fx.tap() {
                    set(cx, &p.fx.time, ms)
                }
            }
            Part::Enable => toggle(cx, &p.pan_enabled),
            Part::Wave => set(cx, &p.wave, (p.wave.value() + 1) % 4),
            Part::Sustain => toggle(cx, &p.sustain_pedal),
            _ => {}
        }
    }
    pub(super) fn paint_mk8(&self, d: &mut Draw) {
        let p = &self.params.mk8;
        let (c, pedal, fx, page) = p.display_state();
        let strike = &self.strikes[Instrument::Mk8.index()];
        let n = strike.note();
        let t = strike.elapsed();
        let down = self.keyboard.is_down(n);
        let rise = action::hammer_lift(t);
        let ring = if down || pedal {
            action::vibration(t, 0.8) * 4.
        } else {
            0.
        };
        let gap = self.params.regulation.mk8.value();
        action::panel(
            d,
            WIDTH,
            if fx.fitted {
                "RHODES  ·  MK8 FX"
            } else {
                "RHODES  ·  MK8 S"
            },
            n,
        );
        action::hardware(d, |d| {
            let lift = if down || pedal { 11. } else { 0. };
            action::solid(
                d,
                [(80., 144.), (120., 144.), (120., 167.), (80., 167.)],
                (4., -2.),
                CASTING,
            );
            d.line(107., 143., 152., 122., STEEL, 5.);
            d.poly(
                &[(152., 122.), (227., 122.), (319., 132. + 0.5 * ring)],
                STEEL,
                6.,
            );
            action::screw(d, 100., 148., 4.);
            action::screw(d, 113., 148., 4.);
            d.poly(
                &[
                    (117., 157.),
                    (192., 157. + 0.15 * ring),
                    (255., 157. + 0.55 * ring),
                    (318., 157. + ring),
                ],
                STEEL,
                2.,
            );
            for i in 0..5 {
                let x = 292. + 1.8 * i as f32;
                d.line(
                    x,
                    154. + ring * 0.85,
                    x + 1.,
                    160. + ring * 0.85,
                    CASTING,
                    1.5,
                );
            }
            let px = 325. + gap * 6.;
            d.panel((px + 5., 147., 29., 18.), CASTING, LINE);
            for i in 0..8 {
                let x = px + 6. + i as f32 * 3.;
                d.line(x, 149., x + 1., 163., WOOD, 1.4);
            }
            d.square_rect(px, 152., 7., 8., STEEL);
            d.line(84., 209., 174., 193. - 28. * rise, CASTING, 5.);
            action::screw(d, 84., 209., 4.);
            let hy = 192. - 28. * rise;
            action::solid(
                d,
                [
                    (166., hy - 6.),
                    (177., hy - 8.),
                    (180., hy + 1.),
                    (166., hy + 1.),
                ],
                (2., -1.),
                RUBBER,
            );
            d.line(101., 201., 225., 166. + lift, STEEL, 2.);
            d.square_rect(216., 160. + lift, 19., 6., FELT);
        });
        d.text(60., 231., "ABS HAMMER", 10., MUTED);
        d.text(179., 231., "WOOL DAMPER", 10., MUTED);
        d.text(303., 231., "FLAT POLE", 10., MUTED);
        d.button(
            Part::Board.rect(),
            if fx.fitted {
                "Board · MK8 FX"
            } else {
                "Board · MK8 S"
            },
            fx.fitted,
            TEAL,
        );
        if fx.fitted {
            for ((part, label), i) in TABS.into_iter().zip(["Preamp", "Effects", "Alt"]).zip(0..) {
                d.button(part.rect(), label, i == page, TEAL);
            }
        } else {
            d.text(460., 166., &format!("SETBACK   {gap:+0.2} mm"), 11., TEXT);
        }
        d.button(Part::Sustain.rect(), "Damper pedal", pedal, GOLD);
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        if page > 0 {
            let parts = if page == 1 {
                &EFFECTS[..]
            } else {
                &ALTERNATE[..]
            };
            for &part in parts {
                if let Part::Fx(k) = part {
                    let knob = k.param(&p.fx);
                    let v = knob.unmodulated_normalized_value();
                    let handle = if k == Time {
                        (v * (2000. - 60.) / (800. - 60.)).min(1.)
                    } else {
                        v
                    };
                    draw_slider(
                        d,
                        part.rect(),
                        k.label(),
                        &knob.normalized_value_to_string(v, true),
                        handle,
                    );
                } else {
                    let (label, on) = match part {
                        Part::Comp => ("Comp", fx.compressor),
                        Part::Chorus => ("Chorus", fx.chorus),
                        Part::Phaser => ("Phaser", fx.phaser),
                        Part::Delay => ("Delay", fx.delay),
                        Part::Tap => ("Tap tempo", p.fx.tap_active()),
                        _ => unreachable!(),
                    };
                    d.button(part.rect(), label, on, GOLD);
                }
            }
            return;
        }
        for (part, label, knob) in [
            (Part::Volume, "VOLUME", &p.volume),
            (Part::Drive, "DRIVE", &p.drive),
            (Part::Envelope, "ENVELOPE", &p.envelope),
            (Part::Low, "LOW", &p.low),
            (Part::Mid, "MID", &p.mid),
            (Part::High, "HIGH", &p.high),
            (Part::Frequency, "MID FREQ", &p.frequency),
            (Part::Resonance, "MID RES", &p.resonance),
            (Part::Rate, "PAN RATE", &p.rate),
            (Part::Depth, "DEPTH", &p.depth),
        ] {
            let v = knob.unmodulated_normalized_value();
            draw_slider(
                d,
                part.rect(),
                label,
                &knob.normalized_value_to_string(v, true),
                v,
            );
        }
        d.button(Part::Enable.rect(), "Vari-Pan", c.pan_enabled, GOLD);
        d.button(
            Part::Wave.rect(),
            ["Square", "Ramp", "Triangle", "Sine"][p.wave.value() as usize],
            false,
            TEAL,
        )
    }
}

fn draw_slider(d: &mut Draw, rect: Rect, label: &str, value: &str, handle: f32) {
    let (x, y, w, _) = rect;
    d.text(x + 5., y + 9., label, 10., TEAL);
    d.text(x + 5., y + 25., value, 10., TEXT);
    d.line(x + 5., y + 37., x + w - 5., y + 37., LINE, 3.);
    d.square_rect(x + 3. + (w - 10.) * handle, y + 31., 4., 12., TEAL);
}
