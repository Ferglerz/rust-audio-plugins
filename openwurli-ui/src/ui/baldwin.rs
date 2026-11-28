//! Cutaway of the spinet action and the preloaded ceramic within its aluminum U-channel.
use super::{toggle, EventContext, Instrument, Knob, PianoView, Rect, Target, WIDTH};
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, FELT, RUBBER, STEEL, WOOD},
    point_in_rect,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Part {
    Volume,
    Soft,
    Sustain,
}
impl Part {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Volume => (40., 290., 185., 54.),
            Self::Soft => (255., 303., 192., 33.),
            Self::Sustain => (474., 303., 190., 33.),
        }
    }
    pub(super) fn level(self) -> Option<Knob> {
        (self == Self::Volume).then_some(Knob::BaldwinVolume)
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (l, _, w, _) = self.rect();
        ((x - l - 7.) / (w - 14.)).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self{
        Self::Volume=>"Adjust the classroom piano’s electrical output.",
        Self::Soft=>"Engage the electronic attenuation pedal. MIDI CC67 also operates it.",
        Self::Sustain=>"Lift the string dampers and allow unplayed courses to respond through the rail. MIDI CC64 also operates the pedal.",
    }
    }
}
impl PianoView {
    pub(super) fn baldwin_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (43., 103., 360., 128.)) {
            return Some(Target::HohnerKey);
        }
        [Part::Volume, Part::Soft, Part::Sustain]
            .into_iter()
            .find(|p| point_in_rect(x, y, p.rect()))
            .map(Target::Baldwin)
    }
    pub(super) fn baldwin_press(&mut self, cx: &mut EventContext, part: Part) {
        match part {
            Part::Soft => toggle(cx, &self.params.baldwin.soft),
            Part::Sustain => toggle(cx, &self.params.baldwin.sustain),
            Part::Volume => {}
        }
    }
    pub(super) fn paint_baldwin(&self, d: &mut Draw) {
        let s = &self.strikes[Instrument::Baldwin.index()];
        let n = s.note();
        let age = s.elapsed();
        let down = self.keyboard.is_down(n);
        let (soft, sustain) = self.params.baldwin.display();
        let stroke = action::hammer_lift(age);
        let ring = if down || sustain {
            action::vibration(age, 0.65) * 2.1
        } else {
            0.
        };
        action::panel(d, WIDTH, "BALDWIN  ·  CLASSROOM ELECTROPIANO", n);
        action::hardware(d, |d| {
            // Vertical action. The moving hammer lifts away after striking the string.
            action::screw(d, 99., 202., 4.);
            let x = 144. + stroke * 36.;
            d.line(99., 202., x, 147., WOOD, 4.);
            d.circle(x + 3., 144., 8., FELT, true);
            d.poly(
                &[
                    (194., 110.),
                    (194. + ring, 145.),
                    (194. + 0.6 * ring, 173.),
                    (194., 197.),
                    (212., 211.),
                ],
                STEEL,
                2.,
            );
            let damper_x = if down || sustain { 221. } else { 201. };
            d.square_rect(damper_x - 7., 162., 8., 16., FELT);
            d.line(damper_x + 1., 169., 239., 188., WOOD, 2.);
            action::screw(d, 240., 190., 3.);
            // Magnified section of the U-rail. The upper lip and silvered pickup
            // transfer the bearing load into the lower arm and felt-covered plate.
            d.poly(
                &[(283., 151.), (358., 151.), (358., 191.), (283., 191.)],
                CASTING,
                7.,
            );
            d.poly(&[(265., 118.), (301., 149.), (337., 121.)], STEEL, 2.);
            d.square_rect(292., 155., 22., 9., RUBBER);
            d.square_rect(292., 164., 22., 3., GOLD);
            d.square_rect(292., 167., 22., 18., TEAL);
            d.line(313., 165., 339., 174., GOLD, 1.5);
            d.square_rect(278., 197., 83., 5., FELT);
        });
        d.text(68., 229., "FELT HAMMER", 10., MUTED);
        d.text(171., 229., "STRING", 10., MUTED);
        d.text(282., 229., "PIEZO RAIL", 10., TEAL);
        d.text(
            445.,
            122.,
            if baldwin_electropiano::rail::section(n) == 0 {
                "BASS RAIL"
            } else {
                "TREBLE RAIL"
            },
            12.,
            TEAL,
        );
        d.text(
            445.,
            150.,
            &format!(
                "{:.0} mm string",
                baldwin_electropiano::action::length_m(n) * 1000.
            ),
            15.,
            TEXT,
        );
        d.text(445., 178., "PIEZO POSITION ALONG RAIL", 10., MUTED);
        d.line(448., 201., 667., 201., STEEL, 2.);
        let offset = self.params.regulation.baldwin.value();
        for i in 0..6 {
            let x = 456. + i as f32 * 39. + offset * 1.4;
            d.square_rect(x, 195., 11., 13., TEAL);
        }
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        let (x, y, w, _) = Part::Volume.rect();
        let volume = self.params.baldwin.volume.value();
        d.text(x + 7., y + 11., "VOLUME", 11., TEAL);
        d.text_right(
            x + w - 7.,
            y + 11.,
            &format!("{:.0}%", 100. * volume),
            11.,
            TEXT,
        );
        d.line(x + 7., y + 35., x + w - 7., y + 35., LINE, 4.);
        d.panel((x + 4. + volume * (w - 14.), y + 27., 6., 16.), TEAL, LINE);
        d.button(Part::Soft.rect(), "Electronic soft pedal", soft, GOLD);
        d.button(Part::Sustain.rect(), "Damper pedal", sustain, GOLD);
    }
}
