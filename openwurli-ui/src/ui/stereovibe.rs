//! Original 64 Classic with its serviced stereo preamp and optical panning pair.
use super::{set, toggle, EventContext, Instrument, Knob, PianoView, Rect, Target};
use pleasant_ui::{
    draw::Draw,
    point_in_rect,
    theme::{GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
};
use vintage_vibe_64::stereo::Input;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    Volume,
    Bass,
    Treble,
    Rate,
    Depth,
    Tremolo,
    Half,
    Input,
    Sustain,
}
impl Part {
    const ALL: [Self; 9] = [
        Self::Volume,
        Self::Bass,
        Self::Treble,
        Self::Rate,
        Self::Depth,
        Self::Tremolo,
        Self::Half,
        Self::Input,
        Self::Sustain,
    ];
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Volume => (32., 281., 119., 46.),
            Self::Bass => (169., 281., 119., 46.),
            Self::Treble => (306., 281., 119., 46.),
            Self::Rate => (443., 281., 119., 46.),
            Self::Depth => (580., 281., 119., 46.),
            Self::Tremolo => (32., 345., 134., 33.),
            Self::Half => (180., 345., 134., 33.),
            Self::Input => (328., 345., 184., 33.),
            Self::Sustain => (527., 345., 170., 33.),
        }
    }
    pub(super) fn level(self) -> Option<Knob> {
        Some(match self {
            Self::Volume => Knob::StereoVolume,
            Self::Bass => Knob::StereoBass,
            Self::Treble => Knob::StereoTreble,
            Self::Rate => Knob::StereoRate,
            Self::Depth => Knob::StereoDepth,
            _ => return None,
        })
    }
    pub(super) fn slider_value(self, x: f32) -> f32 {
        let (l, _, w, _) = self.rect();
        ((x - l - 6.) / (w - 12.)).clamp(0., 1.)
    }
    pub(super) fn help(self) -> &'static str {
        match self {
            Self::Volume => "Set the active preamplifier's output volume.",
            Self::Bass => "Cut or boost low frequencies. The middle position is flat.",
            Self::Treble => "Cut or boost high frequencies. The middle position is flat.",
            Self::Rate => "Turn the inner speed control for the stereo tremolo oscillator.",
            Self::Depth => "Turn the outer depth control to vary the balance between the dry and panned signals.",
            Self::Tremolo => "Push the speed control to switch stereo tremolo on or off.",
            Self::Half => "Push the tone control to halve the tremolo oscillator rate.",
            Self::Input => "Cycle the under-lid DIP bank through the Classic, high-impedance FET and parallel input paths.",
            Self::Sustain => "Lift the tine dampers with the pedal rod. MIDI CC64 also operates the rod.",
        }
    }
}
impl PianoView {
    pub(super) fn stereovibe_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (36., 100., 395., 138.)) {
            return Some(Target::HohnerKey);
        }
        Part::ALL
            .into_iter()
            .find(|p| point_in_rect(x, y, p.rect()))
            .map(Target::StereoVibe)
    }
    pub(super) fn stereovibe_press(&mut self, cx: &mut EventContext, part: Part) {
        let p = &self.params.stereovibe;
        match part {
            Part::Sustain => toggle(cx, &p.sustain_pedal),
            Part::Tremolo => toggle(cx, &p.tremolo),
            Part::Half => toggle(cx, &p.half_speed),
            Part::Input => set(cx, &p.input, (p.input.value() + 1) % 3),
            _ => {}
        }
    }
    pub(super) fn paint_stereovibe(&self, d: &mut Draw) {
        let (c, pedal) = self.params.stereovibe.display_state();
        let gap = self.params.regulation.stereovibe.value();
        self.paint_vv64_action(
            d,
            Instrument::StereoVibe,
            "VINTAGE VIBE  ·  STEREOVIBE",
            pedal,
            gap,
        );
        let input = match c.input {
            Input::Classic => "Classic",
            Input::Fet => "FET",
            Input::Parallel => "Parallel",
        };
        d.text(456., 125., "STEREOVIBE  ·  REV 4.1", 11., TEAL);
        d.text(456., 151., "Harp send", 12., TEXT);
        d.line(524., 147., 530., 147., LINE, 2.);
        d.text(538., 151., input, 12., TEXT);
        d.line(595., 147., 610., 147., LINE, 2.);
        d.text(620., 151., "EQ", 12., TEXT);
        d.line(629., 158., 629., 173., LINE, 2.);
        d.line(510., 173., 645., 173., LINE, 2.);
        for (x, label) in [(510., "Left"), (645., "Right")] {
            d.line(x, 173., x, 184., LINE, 2.);
            d.text(
                x - 15.,
                203.,
                label,
                12.,
                if c.tremolo { TEAL } else { MUTED },
            );
        }
        d.panel((18., 264., 692., 134.), PANEL, LINE);
        for (p, label, v, value) in [
            (
                Part::Volume,
                "VOLUME",
                c.volume,
                format!("{:.0}%", 100. * c.volume),
            ),
            (
                Part::Bass,
                "BASS",
                c.bass,
                format!("{:+.0} dB", 24. * (c.bass - 0.5)),
            ),
            (
                Part::Treble,
                "TREBLE",
                c.treble,
                format!("{:+.0} dB", 24. * (c.treble - 0.5)),
            ),
            (
                Part::Rate,
                "SPEED",
                c.rate,
                format!("{:.1} Hz", c.rate_hz()),
            ),
            (
                Part::Depth,
                "DEPTH",
                c.depth,
                format!("{:.0}%", 100. * c.depth),
            ),
        ] {
            let (x, y, w, _) = p.rect();
            d.text(x + 6., y + 10., label, 10., TEAL);
            d.text_right(x + w - 6., y + 10., &value, 10., TEXT);
            d.line(x + 6., y + 32., x + w - 6., y + 32., LINE, 4.);
            d.panel(
                (x + 3. + (w - 12.) * v as f32, y + 25., 6., 14.),
                TEAL,
                LINE,
            );
        }
        d.button(Part::Tremolo.rect(), "Tremolo", c.tremolo, GOLD);
        d.button(Part::Half.rect(), "Half speed", c.half_speed, GOLD);
        d.button(
            Part::Input.rect(),
            &format!("Input · {input}"),
            c.input != Input::Classic,
            TEAL,
        );
        d.button(Part::Sustain.rect(), "Damper pedal", pedal, GOLD);
    }
}
