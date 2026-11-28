//! Piano key geometry and paint shared by Chordboard and the electric pianos.
use crate::{point_in_rect, with_alpha, Draw, BG, GOLD, LINE, TEXT};
use nih_plug_vizia::vizia::vg::Color;

type Rect = (f32, f32, f32, f32);

#[derive(Clone, Copy)]
pub struct Piano {
    bounds: Rect,
    low: u8,
    high: u8,
}

pub const fn is_black(note: u8) -> bool {
    matches!(note % 12, 1 | 3 | 6 | 8 | 10)
}

// Number of natural keys below a MIDI pitch, including the sentinel at 128.
fn naturals_before(note: u8) -> u8 {
    (note / 12) * 7 + [0, 1, 1, 2, 2, 3, 4, 4, 5, 5, 6, 6][(note % 12) as usize]
}

impl Piano {
    /// The first and last keys are naturals at the ends of the keybed.
    pub const fn new(bounds: Rect, low: u8, high: u8) -> Self {
        assert!(low <= high && high <= 127 && !is_black(low) && !is_black(high));
        Self { bounds, low, high }
    }

    pub fn key_rect(self, note: u8) -> Rect {
        let (x, y, w, h) = self.bounds;
        let first = naturals_before(self.low);
        let width = w / f32::from(naturals_before(self.high + 1) - first);
        let offset = f32::from(naturals_before(note) - first);
        if is_black(note) {
            (x + (offset - 0.31) * width, y, width * 0.62, h * 0.62)
        } else {
            (x + offset * width, y, width, h)
        }
    }

    /// Paint all naturals first so the shorter accidentals sit on top.
    pub fn keys(self) -> impl DoubleEndedIterator<Item = u8> {
        let notes = self.low..=self.high;
        notes
            .clone()
            .filter(|&n| !is_black(n))
            .chain(notes.filter(|&n| is_black(n)))
    }

    pub fn note_at(self, x: f32, y: f32) -> Option<u8> {
        if !point_in_rect(x, y, self.bounds) {
            return None;
        }
        let notes = self.low..=self.high;
        notes
            .clone()
            .filter(|&n| is_black(n))
            .chain(notes.filter(|&n| !is_black(n)))
            .find(|&n| point_in_rect(x, y, self.key_rect(n)))
    }

    pub fn draw_key(d: &mut Draw, r: Rect, black: bool, tint: Option<Color>) {
        let fill = match (black, d.light) {
            (true, true) => TEXT,
            (false, false) => with_alpha(TEXT, 0.65),
            _ => BG,
        };
        d.rect(r.0, r.1, r.2, r.3, fill);
        if let Some(color) = tint {
            d.rect(r.0 + 0.5, r.1, r.2 - 1.0, r.3, color);
        }
        d.outline(r, if black || d.light { LINE } else { BG });
    }

    pub fn highlight(d: &mut Draw, r: Rect, color: Color) {
        d.rounded_rect(
            r.0 + 1.0,
            r.1 + 1.0,
            (r.2 - 2.0).max(1.0),
            r.3 - 2.0,
            1.0,
            color,
        );
    }

    pub fn draw_instrument(self, d: &mut Draw, notes: u128) {
        let hover = d.hover.and_then(|(x, y)| self.note_at(x, y));
        for note in self.keys() {
            let r = self.key_rect(note);
            Self::draw_key(
                d,
                r,
                is_black(note),
                (hover == Some(note)).then(|| with_alpha(GOLD, 0.25)),
            );
            if notes & (1 << note) != 0 {
                Self::highlight(d, r, with_alpha(GOLD, 0.75));
            }
        }
        // In extended mode a two-character octave label can span two naturals.
        // Paint these last so the next key cannot cover half the text.
        for note in (self.low..=self.high).filter(|n| n.is_multiple_of(12)) {
            let r = self.key_rect(note);
            let octave = (note / 12) as i8 - 1;
            let margin = 9.0_f32.min(self.bounds.2 * 0.5);
            d.text_centered(
                (r.0 + r.2 * 0.5).clamp(
                    self.bounds.0 + margin,
                    self.bounds.0 + self.bounds.2 - margin,
                ),
                r.1 + r.3 - 4.0,
                &format!("C{octave}"),
                10.0,
                if d.light { TEXT } else { BG },
            );
        }
    }
}
