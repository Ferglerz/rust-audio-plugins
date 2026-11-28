//! Asymmetric tine, tuning spring, bridle damper and passive panel.
use super::{toggle, EventContext, Instrument, Knob, PianoView, Rect, Target, WIDTH};
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, FELT, RUBBER, STEEL, WOOD},
    point_in_rect,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Volume,
    Bass,
    Treble,
    Sustain,
}
impl Part {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Volume => (37., 292., 147., 57.),
            Self::Bass => (205., 292., 147., 57.),
            Self::Treble => (372., 292., 147., 57.),
            Self::Sustain => (541., 306., 137., 35.),
        }
    }
    pub(super) fn level(self) -> Option<Knob> {
        match self {
            Self::Volume => Some(Knob::Vv64Volume),
            Self::Bass => Some(Knob::Vv64Bass),
            Self::Treble => Some(Knob::Vv64Treble),
            Self::Sustain => None,
        }
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (l, _, w, _) = self.rect();
        ((x - l - 6.) / (w - 12.)).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self {
            Self::Volume => "Move the wiper of the passive output volume pot.",
            Self::Bass => "Turn down the bass control to attenuate the lowest frequencies.",
            Self::Treble => "Turn down the treble control to shunt higher frequencies to ground.",
            Self::Sustain => {
                "Move the pedal rod to lift the tine dampers. MIDI CC64 also operates the rod."
            }
        }
    }
}

impl PianoView {
    pub(super) fn vv64_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (36., 100., 395., 138.)) {
            return Some(Target::HohnerKey);
        }
        [Part::Volume, Part::Bass, Part::Treble, Part::Sustain]
            .into_iter()
            .find(|p| point_in_rect(x, y, p.rect()))
            .map(Target::Vv64)
    }
    pub(super) fn vv64_press(&mut self, cx: &mut EventContext, part: Part) {
        if part == Part::Sustain {
            toggle(cx, &self.params.vv64.sustain_pedal);
        }
    }
    pub(super) fn paint_vv64_action(
        &self,
        d: &mut Draw,
        instrument: Instrument,
        title: &str,
        pedal: bool,
        gap: f32,
    ) -> u8 {
        let strike = &self.strikes[instrument.index()];
        let n = strike.note();
        let t = strike.elapsed();
        let down = self.keyboard.is_down(n);
        let key = if down { (t / 0.065).min(1.) } else { 0. };
        let hammer = action::hammer_lift(t);
        let ring = if down || pedal {
            action::vibration(t, 0.7) * 4.
        } else {
            0.
        };
        let tip = 295. + ((vintage_vibe_64::action::length_mm(n) as f32) - 52.) * 0.30;
        let hit = 119. + (tip - 119.) * (0.22 + 0.19 * f32::from(n.clamp(33, 96) - 33) / 63.);
        action::panel(d, WIDTH, title, n);
        action::hardware(d, |d| {
            // Side elevation. Both arms bolt to the same generator block.
            action::solid(
                d,
                [(92., 134.), (120., 134.), (120., 164.), (92., 164.)],
                (4., -2.),
                CASTING,
            );
            d.poly(
                &[
                    (113., 139.),
                    (154., 125.),
                    (239., 125.),
                    (tip - 5., 132. - 0.6 * ring),
                ],
                STEEL,
                5.,
            );
            d.poly(
                &[
                    (119., 156.),
                    (197., 156. + 0.18 * ring),
                    (244., 156. + 0.52 * ring),
                    (tip, 156. + ring),
                ],
                STEEL,
                2.,
            );
            let spring = 119. + (tip - 119.) * vintage_vibe_64::action::SPRING_AT as f32;
            for i in 0..5 {
                let x = spring + i as f32 * 1.8;
                d.line(
                    x,
                    152. + 0.70 * ring,
                    x + 1.,
                    159. + 0.70 * ring,
                    CASTING,
                    1.4,
                );
            }
            let pole = tip + 7. + gap * 6.;
            d.line(pole, 154., pole + 12., 154., STEEL, 3.);
            d.panel((pole + 10., 147., 29., 14.), CASTING, LINE);
            for i in 0..8 {
                let x = pole + 12. + i as f32 * 3.;
                d.line(x, 147., x + 1., 161., WOOD, 1.3);
            }
            d.line(86., 209., hit, 191. - hammer * 27., WOOD, 5.);
            action::screw(d, 86., 209., 4.);
            let y = 190. - hammer * 27.;
            action::solid(
                d,
                [
                    (hit - 7., y - 7.),
                    (hit + 5., y - 6.),
                    (hit + 7., y + 2.),
                    (hit - 6., y + 1.),
                ],
                (2., -1.),
                RUBBER,
            );
            let damper = if pedal { 1. } else { key };
            d.line(111., 201., 218., 163. + damper * 13., STEEL, 2.);
            d.line(
                211.,
                160. + damper * 13.,
                224.,
                160. + damper * 13.,
                FELT,
                5.,
            );
            d.line(142., 190. - 5. * hammer, 154., 187. + damper * 7., WOOD, 1.);
        });
        d.text(65., 231., "HAMMER / BRIDLE", 10., MUTED);
        d.text(196., 231., "TUNING SPRING", 10., MUTED);
        d.text(327., 231., "PICKUP", 10., MUTED);
        n
    }
    pub(super) fn paint_vv64(&self, d: &mut Draw) {
        let (c, pedal) = self.params.vv64.display_state();
        let gap = self.params.regulation.vv64.value();
        let n = self.paint_vv64_action(
            d,
            Instrument::Vv64,
            "VINTAGE VIBE  ·  64 CLASSIC",
            pedal,
            gap,
        );
        d.text(456., 126., "TINE LENGTH", 11., TEAL);
        d.text(
            456.,
            150.,
            &format!("{:.1} mm", vintage_vibe_64::action::length_mm(n)),
            16.,
            TEXT,
        );
        d.text(456., 180., "PICKUP SETBACK", 11., TEAL);
        d.text(456., 203., &format!("{gap:+.2} mm"), 14., TEXT);
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        for (p, label, v) in [
            (Part::Volume, "VOLUME", c.volume),
            (Part::Bass, "BASS", c.bass),
            (Part::Treble, "TREBLE", c.treble),
        ] {
            let (x, y, w, _) = p.rect();
            d.text(x + 6., y + 10., label, 11., TEAL);
            d.text_right(x + w - 6., y + 10., &format!("{:.0}%", v * 100.), 11., TEXT);
            d.line(x + 6., y + 36., x + w - 6., y + 36., LINE, 4.);
            d.panel(
                (x + 3. + (w - 12.) * v as f32, y + 27., 6., 18.),
                TEAL,
                LINE,
            );
        }
        d.button(Part::Sustain.rect(), "Damper pedal", pedal, GOLD);
    }
}
