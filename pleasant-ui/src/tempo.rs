//! Shared header presentation. Callers own parameter events and value editing.
use crate::{Draw, GOLD, LINE, MUTED, TEAL, TEXT};
pub type Rect = (f32, f32, f32, f32);

#[derive(Clone, Copy, Debug)]
pub struct Header {
    pub source: Rect,
    pub bpm: Rect,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_hits_source_and_bpm_without_claiming_the_gap() {
        let header = Header {
            source: (10.0, 20.0, 130.0, 30.0),
            bpm: (150.0, 20.0, 150.0, 30.0),
        };
        assert_eq!(header.hit(20.0, 25.0), Some(Hit::Source));
        assert_eq!(header.hit(160.0, 25.0), Some(Hit::Bpm));
        assert_eq!(header.hit(145.0, 25.0), None);
        assert_eq!(header.hit(20.0, 55.0), None);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    Source,
    Bpm,
}

impl Header {
    pub fn hit(self, x: f32, y: f32) -> Option<Hit> {
        let inside = |r: Rect| x >= r.0 && x <= r.0 + r.2 && y >= r.1 && y <= r.1 + r.3;
        if inside(self.source) {
            Some(Hit::Source)
        } else if inside(self.bpm) {
            Some(Hit::Bpm)
        } else {
            None
        }
    }

    pub fn draw(self, d: &mut Draw, host: bool, bpm: f64, manual_norm: f32, hover: f32) {
        let color = if host { TEAL } else { GOLD };
        let r = self.source;
        d.button_surface(r, color, 0.0, hover, 0.0);
        let x = r.0 + 16.0;
        let y = r.1 + r.3 * 0.5;
        if host {
            // Two linked loops, independent of the caller's icon font.
            d.circle(x - 2.0, y - 2.0, 4.0, color, false);
            d.circle(x + 2.0, y + 2.0, 4.0, color, false);
        } else {
            d.poly(
                &[
                    (x - 6.0, y + 6.0),
                    (x, y - 7.0),
                    (x + 6.0, y + 6.0),
                    (x - 6.0, y + 6.0),
                ],
                color,
                1.4,
            );
            d.line(x, y + 3.0, x + 5.0, y - 5.0, color, 1.4);
        }
        d.text(
            r.0 + 30.0,
            y + 4.5,
            if host { "Host tempo" } else { "Manual tempo" },
            12.0,
            color,
        );
        let r = self.bpm;
        if host {
            d.text(
                r.0 + 16.0,
                r.1 + r.3 * 0.5 + 4.5,
                &format!("{bpm:.1} bpm"),
                13.0,
                TEXT,
            );
        } else {
            let track = (r.0 + 12.0, r.1 + 11.0, (r.2 - 92.0).max(12.0), 3.0);
            d.rounded_rect(track.0, track.1, track.2, track.3, 1.5, LINE);
            d.rounded_rect(
                track.0,
                track.1,
                track.2 * manual_norm.clamp(0.0, 1.0),
                track.3,
                1.5,
                GOLD,
            );
            d.text_right(
                r.0 + r.2 - 12.0,
                r.1 + 16.0,
                &format!("{bpm:.1}"),
                11.0,
                GOLD,
            );
            d.text_right(r.0 + r.2 - 12.0, r.1 + 28.0, "bpm", 9.0, MUTED);
        }
    }
}
