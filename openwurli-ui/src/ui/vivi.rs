//! Museum Vivi-Tone’s picker, double-bearing steel bar and facing pickup.
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
    Expression,
    Bass,
    Sustain,
}
impl Part {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Volume => (36., 289., 158., 62.),
            Self::Expression => (210., 289., 151., 62.),
            Self::Bass => (383., 304., 137., 36.),
            Self::Sustain => (535., 304., 143., 36.),
        }
    }
    pub(super) fn level(self) -> Option<Knob> {
        match self {
            Self::Volume => Some(Knob::ViviVolume),
            Self::Expression => Some(Knob::ViviExpression),
            _ => None,
        }
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (l, _, w, _) = self.rect();
        ((x - l - 6.) / (w - 12.)).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self{Self::Volume=>"Set the 0–100 volume control on the nameboard.",Self::Expression=>"Set the electrical volume pedal. MIDI CC11 controls the same potentiometer.",Self::Bass=>"Lift the bass dampers. Notes played while the rail is raised sustain. MIDI CC66 also moves this rail.",Self::Sustain=>"Lift the rear dampers across the keyboard. MIDI CC64 also moves the full rail."}
    }
}
impl PianoView {
    pub(super) fn vivi_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (36., 98., 396., 141.)) {
            return Some(Target::HohnerKey);
        }
        [Part::Volume, Part::Expression, Part::Bass, Part::Sustain]
            .into_iter()
            .find(|p| point_in_rect(x, y, p.rect()))
            .map(Target::Vivi)
    }
    pub(super) fn vivi_press(&mut self, cx: &mut EventContext, p: Part) {
        match p {
            Part::Bass => toggle(cx, &self.params.vivi.bass_pedal),
            Part::Sustain => toggle(cx, &self.params.vivi.sustain_pedal),
            _ => (),
        }
    }
    pub(super) fn paint_vivi(&self, d: &mut Draw) {
        let (c, pedal, bass) = self.params.vivi.display_state();
        let strike = &self.strikes[Instrument::Vivi.index()];
        let n = strike.note();
        let t = strike.elapsed();
        let down = self.keyboard.is_down(n);
        let key = if down { (t / 0.07).min(1.) } else { 0. };
        let open = down || pedal || (bass && n <= pianet_n::vivi::BASS_HIGH);
        let vibr = if open {
            3.0 * action::vibration(t, 0.76)
        } else {
            0.
        };
        let gap = self.params.regulation.vivi.value() * 7.;
        action::panel(d, WIDTH, "VIVI-TONE  ·  CLAVIER C 1004", n);
        action::hardware(d, |d| {
            // Fig. 3 side elevation, with the specimen's additional bass damper rail.
            d.panel((186., 150., 53., 25.), WOOD, CASTING);
            d.panel((186., 128., 53., 9.), WOOD, CASTING);
            for x in [190., 235.] {
                action::screw(d, x, 140., 3.2);
                action::screw(d, x, 147., 3.2);
            }
            d.poly(
                &[
                    (129., 143. + vibr * 0.6),
                    (155., 143. + vibr * 0.25),
                    (190., 143.),
                    (235., 143.),
                    (263., 143. + vibr * 0.25),
                    (296., 143. + vibr),
                ],
                STEEL,
                3.,
            );
            let lift = action::hammer_lift(t);
            let angle = if down { key * 8. } else { 0. };
            d.line(70., 189., 111., 174. - key * 7., WOOD, 6.);
            d.line(
                111.,
                174. - key * 7.,
                121. - angle,
                156. - 10. * lift,
                WOOD,
                4.,
            );
            d.line(
                120. - angle,
                156. - lift * 10.,
                138. - angle,
                150. - 6. * lift,
                CASTING,
                3.,
            );
            action::screw(d, 111., 174. - key * 7., 3.);
            d.panel((130., 136. - 9. * key, 14., 5.), FELT, LINE);
            let lift = if open { 8. } else { 0. };
            d.line(264., 117., 290., 134. - lift, WOOD, 3.);
            d.panel((283., 135. - lift, 15., 6.), FELT, LINE);
            d.panel((281., 155. + gap, 28., 6.), STEEL, LINE);
            d.panel((286., 161. + gap, 18., 20.), CASTING, LINE);
            for i in 0..6 {
                d.line(
                    286.,
                    163. + gap + i as f32 * 2.5,
                    304.,
                    163. + gap + i as f32 * 2.5,
                    WOOD,
                    1.,
                );
            }
            d.line(304., 174. + gap, 335., 175. + gap, STEEL, 1.);
            d.line(335., 175. + gap, 342., 166. + gap, STEEL, 1.);
        });
        d.text(68., 224., "PICKER JACK", 10., MUTED);
        d.text(171., 224., "BEARINGS", 10., MUTED);
        d.text(264., 224., "ARMATURE", 10., MUTED);
        d.text(451., 123., "PARALLEL PICKUP BANK", 11., TEAL);
        d.text(451., 149., "Steel bar / magnetic pole", 12., TEXT);
        d.text(451., 174., "Nameboard / volume pedal", 12., TEXT);
        d.text(451., 200., "Bass and full damper rails", 11., MUTED);
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        for (p, label, v) in [
            (Part::Volume, "VOLUME", c.volume),
            (Part::Expression, "PEDAL VOL", c.expression),
        ] {
            let (x, y, w, _) = p.rect();
            d.text(x + 6., y + 10., label, 11., TEAL);
            d.text_right(x + w - 6., y + 10., &format!("{:.0}%", 100. * v), 11., TEXT);
            d.line(x + 6., y + 36., x + w - 6., y + 36., LINE, 4.);
            d.panel(
                (x + 3. + (w - 12.) * v as f32, y + 27., 6., 18.),
                TEAL,
                LINE,
            );
        }
        d.button(Part::Bass.rect(), "Bass sustain", bass, GOLD);
        d.button(Part::Sustain.rect(), "Full sustain", pedal, GOLD);
    }
}
