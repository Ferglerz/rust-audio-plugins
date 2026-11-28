//! Production Valente with a weighted driver, counter-hammer and pull dampers.
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
    Brilliance,
    Sustain,
}
impl Part {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Volume => (38., 293., 196., 57.),
            Self::Brilliance => (264., 293., 196., 57.),
            Self::Sustain => (501., 305., 175., 35.),
        }
    }
    pub(super) fn level(self) -> Option<Knob> {
        match self {
            Self::Volume => Some(Knob::ValenteVolume),
            Self::Brilliance => Some(Knob::ValenteBrilliance),
            Self::Sustain => None,
        }
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (l, _, w, _) = self.rect();
        ((x - l - 6.) / (w - 12.)).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self{Self::Volume=>"Move the passive volume wiper. A treble bypass changes the balance as the volume falls.",Self::Brilliance=>"Change the passive treble cut at the output jack.",Self::Sustain=>"Pull the shared damper rail to let neighboring reeds resonate. MIDI CC64 also operates the rail."}
    }
}
impl PianoView {
    pub(super) fn valente_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (36., 100., 395., 138.)) {
            return Some(Target::HohnerKey);
        }
        [Part::Volume, Part::Brilliance, Part::Sustain]
            .into_iter()
            .find(|p| point_in_rect(x, y, p.rect()))
            .map(Target::Valente)
    }
    pub(super) fn valente_press(&mut self, cx: &mut EventContext, part: Part) {
        if part == Part::Sustain {
            toggle(cx, &self.params.valente.sustain_pedal);
        }
    }
    pub(super) fn paint_valente(&self, d: &mut Draw) {
        let (c, pedal) = self.params.valente.display_state();
        let strike = &self.strikes[Instrument::Valente.index()];
        let n = strike.note();
        let t = strike.elapsed();
        let down = self.keyboard.is_down(n);
        let key = if down { (t / 0.055).min(1.) } else { 0. };
        let hammer = action::hammer_lift(t);
        let ring = if down || pedal {
            3.5 * action::vibration(t, 0.8)
        } else {
            0.
        };
        let open = if pedal { 1. } else { key };
        let gap = -self.params.regulation.valente.value() * 6.;
        action::panel(d, WIDTH, "VALENTE  ·  ELECTRIC PIANO", n);
        action::hardware(d, |d| {
            // Diagrammatic side elevation of the linked driver, free striker and reed.
            let rear = 194. - key * 6.;
            let front = 195. + key * 4.;
            action::solid(
                d,
                [
                    (106., rear),
                    (368., front),
                    (368., front + 8.),
                    (106., rear + 8.),
                ],
                (4., -3.),
                WOOD,
            );
            action::screw(d, 244., 202., 4.);
            d.line(299., 195., 279., 177. + key * 5., CASTING, 4.);
            d.line(277., 177. + key * 5., 194., 180. - key * 6., CASTING, 5.);
            action::screw(d, 260., 179., 3.);
            d.panel((183., 174. - key * 6., 23., 12.), STEEL, LINE);
            d.line(192., 169., 163., 157. - hammer * 14., STEEL, 3.);
            action::screw(d, 192., 169., 3.);
            d.panel((153., 151. - hammer * 14., 17., 7.), FELT, CASTING);
            d.panel((102., 132., 16., 14.), CASTING, LINE);
            action::screw(d, 109., 138., 3.);
            d.poly(
                &[
                    (115., 138.),
                    (132., 138. + ring * 0.1),
                    (162., 138. + ring * 0.45),
                    (202., 138. + ring),
                ],
                STEEL,
                3.,
            );
            d.line(232., 162. - open * 8., 185., 130. - open * 8., STEEL, 2.);
            d.panel((176., 131. - open * 8., 17., 5.), FELT, LINE);
            d.panel((200., 105. + gap, 28., 15.), CASTING, LINE);
            for i in 0..6 {
                let x = 202. + i as f32 * 4.;
                d.line(x, 107. + gap, x, 117. + gap, WOOD, 2.);
            }
            d.line(202., 120. + gap, 202., 130. + gap, STEEL, 3.);
            d.poly(&[(228., 109. + gap), (247., 104.), (264., 104.)], STEEL, 1.);
        });
        d.text(63., 231., "SHOULDERED REED", 10., MUTED);
        d.text(210., 231., "COUNTER-HAMMER / DRIVER", 10., MUTED);
        d.text(452., 123., "MAGNETIC PICKUP BANK", 11., TEAL);
        d.text(452., 150., "Neodymium magnets", 13., TEXT);
        d.text(452., 177., "Passive volume / brilliance", 12., TEXT);
        d.text(452., 202., "Damper pull", 11., MUTED);
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        for (p, label, v) in [
            (Part::Volume, "VOLUME", c.volume),
            (Part::Brilliance, "BRILLIANCE", c.brilliance),
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
