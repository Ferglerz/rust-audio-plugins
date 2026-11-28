//! Folding upright action, magnetic bar and load-bearing bridge.
use super::{set, toggle, EventContext, Instrument, PianoView, Rect, Target, WIDTH};
use pleasant_ui::{
    draw::Draw,
    instrument_action::{self as action, CASTING, FELT, STEEL, WOOD},
    point_in_rect,
    theme::{GOLD, LINE, MUTED, TEAL, TEXT},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Part {
    Jack,
    Board,
    Pedal,
}
impl Part {
    pub(super) fn rect(self) -> Rect {
        match self {
            Self::Jack => (39., 310., 142., 36.),
            Self::Board => (193., 310., 172., 36.),
            Self::Pedal => (459., 310., 205., 36.),
        }
    }
    pub(super) fn help(self) -> &'static str {
        match self{
        Self::Jack=>"Listen to the single instrument jack fed by the bass, middle and treble magnetic strips.",
        Self::Board=>"Listen to the modeled soundboard surface motion. The vibrating bridge also loads the strings heard through the jack.",
        Self::Pedal=>"Lift the upright dampers so released and sympathetic strings can ring. MIDI CC64 moves this pedal."}
    }
}
impl PianoView {
    pub(super) fn road_hit(&self, x: f32, y: f32) -> Option<Target> {
        if point_in_rect(x, y, (30., 96., 390., 140.)) {
            return Some(Target::HohnerKey);
        }
        [Part::Jack, Part::Board, Part::Pedal]
            .into_iter()
            .find(|p| point_in_rect(x, y, p.rect()))
            .map(Target::Road)
    }
    pub(super) fn road_press(&mut self, cx: &mut EventContext, p: Part) {
        match p {
            Part::Jack => set(cx, &self.params.road.board, false),
            Part::Board => set(cx, &self.params.road.board, true),
            Part::Pedal => toggle(cx, &self.params.road.sustain_pedal),
        }
    }
    pub(super) fn paint_road(&self, d: &mut Draw) {
        let strike = &self.strikes[Instrument::Road.index()];
        let note = strike.note();
        let time = strike.elapsed();
        let down = self.keyboard.is_down(note);
        let (board, pedal) = self.params.road.display_state();
        let band = if note < 54 {
            0
        } else if note < 75 {
            1
        } else {
            2
        };
        let gap = [
            &self.params.regulation.road_bass,
            &self.params.regulation.road_mid,
            &self.params.regulation.road_high,
        ][band]
            .value();
        let lift = action::hammer_lift(time);
        let wiggle = if down || pedal {
            action::vibration(time, 0.72) * 3.1
        } else {
            0.
        };
        let key = if down { 1. } else { 0. };
        let damper = if pedal || down { 8. } else { 0. };
        action::panel(d, WIDTH, "HELPINSTILL  ·  ROADMASTER 64", note);
        action::hardware(d, |d| {
            action::solid(
                d,
                [
                    (56., 202. + 3. * key),
                    (174., 189. - 4. * key),
                    (177., 196. - 4. * key),
                    (59., 210. + 3. * key),
                ],
                (3., -2.),
                WOOD,
            );
            d.line(
                172.,
                191. - 4. * key,
                160. + lift * 5.,
                167. - lift * 6.,
                CASTING,
                4.,
            );
            d.line(158., 173., 217. + lift * 19., 144. - lift * 9., WOOD, 4.);
            d.circle(218. + lift * 19., 144. - lift * 9., 7., FELT, true);
            // Two strings travel across a bridge fixed to the spruce board.
            for x in [249., 255.] {
                d.poly(&[(x, 105.), (x + wiggle, 145.), (x, 206.)], STEEL, 1.4);
            }
            d.line(241. - damper, 159., 224. - damper, 168., CASTING, 3.);
            d.line(246. - damper, 154., 246. - damper, 165., FELT, 4.);
            let bar_x = 265. + 5. * gap;
            d.square_rect(bar_x, 123., 9., 24., CASTING);
            d.line(bar_x + 2., 125., bar_x + 2., 145., TEAL, 2.);
            action::solid(
                d,
                [(259., 174.), (290., 181.), (290., 189.), (258., 181.)],
                (3., -3.),
                WOOD,
            );
            action::solid(
                d,
                [
                    (288., 102.),
                    (298., 102.),
                    (298. + wiggle * 0.3, 209.),
                    (287., 209.),
                ],
                (4., -2.),
                WOOD,
            );
            d.line(bar_x + 8., 136., 338., 136., LINE, 1.4);
        });
        d.text(59., 232., "KEY / HAMMER", 10., MUTED);
        d.text(212., 232., "UNISON", 10., MUTED);
        d.text(282., 232., "BOARD", 10., MUTED);
        d.text(446., 120., "MAGNETIC SENSOR", 11., TEAL);
        d.text(
            446.,
            148.,
            ["Bass strip", "Middle strip", "Treble strip"][band],
            15.,
            TEXT,
        );
        d.text(446., 178., "BAR SETBACK", 11., TEAL);
        d.text(446., 203., &format!("{gap:+.2} mm"), 13., TEXT);
        d.panel((18., 264., 692., 134.), pleasant_ui::theme::PANEL, LINE);
        d.text(39., 295., "MONITOR", 11., TEAL);
        d.text(459., 295., "DAMPERS", 11., TEAL);
        d.button(Part::Jack.rect(), "Mono jack", !board, GOLD);
        d.button(Part::Board.rect(), "Soundboard", board, GOLD);
        d.button(Part::Pedal.rect(), "Damper pedal", pedal, GOLD);
    }
}
