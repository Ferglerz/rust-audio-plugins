//! EP-61C side elevation and its two passive controls.
use super::{toggle, EventContext, Instrument, Knob, PianoView, Rect, Target, WIDTH};
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, FELT, STEEL, WOOD},
    point_in_rect,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Volume,
    Tone,
    Alignment,
    Sustain,
}
impl Part {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Volume => (37., 292., 147., 57.),
            Self::Tone => (205., 292., 147., 57.),
            Self::Alignment => (372., 292., 147., 57.),
            Self::Sustain => (541., 306., 137., 35.),
        }
    }
    pub(super) fn level(self) -> Option<Knob> {
        match self {
            Self::Volume => Some(Knob::ElepianVolume),
            Self::Tone => Some(Knob::ElepianTone),
            Self::Alignment => Some(Knob::ElepianAlignment),
            Self::Sustain => None,
        }
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (l, _, w, _) = self.rect();
        ((x - l - 6.) / (w - 12.)).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self{
        Self::Volume=>"Move the wiper of the 100 kΩ volume pot at the passive output.",
        Self::Tone=>"Move the 100 kΩ tone pot. Its wiper connects the 0.047 µF capacitor to the pickup signal.",
        Self::Alignment=>"Move the lower pickup screw across the reed tip to change the balance of fundamental and harmonics.",
        Self::Sustain=>"Move the pedal rod to lift the rear reed dampers. MIDI CC64 also operates the rod."
    }
    }
}

impl PianoView {
    pub(super) fn elepian_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (36., 100., 395., 138.)) {
            return Some(Target::HohnerKey);
        }
        [Part::Volume, Part::Tone, Part::Alignment, Part::Sustain]
            .into_iter()
            .find(|p| point_in_rect(x, y, p.rect()))
            .map(Target::Elepian)
    }
    pub(super) fn elepian_press(&mut self, cx: &mut EventContext, part: Part) {
        if part == Part::Sustain {
            toggle(cx, &self.params.elepian.sustain_pedal);
        }
    }
    pub(super) fn paint_elepian(&self, d: &mut Draw) {
        let (c, pedal) = self.params.elepian.display_state();
        let strike = &self.strikes[Instrument::Elepian.index()];
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
        let gap = -self.params.regulation.elepian.value() * 6.;
        let align = -c.alignment_mm as f32 * 8.;
        let rail = (if pedal { 1. } else { key }) * 10.;
        action::panel(d, WIDTH, "COLUMBIA  ·  ELEPIAN EP-61C", n);
        action::hardware(d, |d| {
            // Original action sheet, viewed from the side: key front to the
            // right, vertical reed on the left, coil above its weighted tip.
            let rear = 203. - key * 5.;
            action::solid(
                d,
                [
                    (126., rear),
                    (372., 199. + key * 4.),
                    (372., 207. + key * 4.),
                    (126., rear + 8.),
                ],
                (5., -3.),
                WOOD,
            );
            action::screw(d, 267., 207., 4.);
            d.line(202., rear, 202. + key * 2., 177. - key * 5., CASTING, 4.);
            d.line(179., 181., 225., 165. - key * 4., CASTING, 4.);
            action::screw(d, 179., 181., 4.);
            let hx = 164. - hammer * 37.;
            d.line(182., 178., hx + 2., 135., STEEL, 4.);
            action::solid(
                d,
                [
                    (hx - 10., 127.),
                    (hx + 11., 131.),
                    (hx + 12., 140.),
                    (hx - 12., 137.),
                ],
                (3., -1.),
                FELT,
            );
            d.line(195., 180., 211., 149., STEEL, 2.);
            d.line(206., 147., 217., 153., FELT, 4.);
            action::solid(
                d,
                [(107., 177.), (124., 177.), (124., 184.), (107., 184.)],
                (4., -2.),
                CASTING,
            );
            d.poly(
                &[
                    (115., 177.),
                    (115. + ring * 0.18, 155.),
                    (115. + ring * 0.57, 135.),
                    (115. + ring, 117.),
                ],
                STEEL,
                3.,
            );
            d.panel((111. + ring, 116., 9., 4.), CASTING, LINE);
            d.line(79. - rail, 166., 95. - rail, 139., STEEL, 2.);
            d.panel((93. - rail, 135., 20., 8.), FELT, CASTING);
            d.line(
                114. + align,
                108. + gap,
                114. + align,
                114. + gap,
                STEEL,
                3.,
            );
            d.panel((100. + align, 96. + gap, 28., 13.), CASTING, LINE);
            for i in 0..7 {
                d.line(
                    101. + align,
                    98. + gap + i as f32 * 1.3,
                    126. + align,
                    97. + gap + i as f32 * 1.3,
                    WOOD,
                    1.,
                );
            }
            d.poly(
                &[(125. + align, 102. + gap), (143., 99.), (164., 99.)],
                STEEL,
                1.,
            );
        });
        d.text(66., 231., "REED / DAMPER", 10., MUTED);
        d.text(204., 231., "JACK / BACKCHECK", 10., MUTED);
        d.text(455., 122., "SERIES PICKUP BANK", 11., TEAL);
        d.text(455., 147., "10 kΩ  ·  PASSIVE PANEL", 12., TEXT);
        d.text(455., 172., "0.047 µF  ·  TONE", 12., TEXT);
        d.text(455., 197., "15 kΩ  ·  NOMINAL OUTPUT", 11., MUTED);
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        for (p, label, v) in [
            (Part::Volume, "VOLUME", c.volume),
            (Part::Tone, "TONE", c.tone),
            (
                Part::Alignment,
                "ALIGNMENT",
                (c.alignment_mm + 1.) * 0.5,
            ),
        ] {
            let (x, y, w, _) = p.rect();
            d.text(x + 6., y + 10., label, 11., TEAL);
            d.text_right(
                x + w - 6.,
                y + 10.,
                &if p == Part::Alignment {
                    format!("{:+.2} mm", c.alignment_mm)
                } else {
                    format!("{:.0}%", v * 100.)
                },
                11.,
                TEXT,
            );
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
