//! Parameter-neutral XY geometry and performance gestures.
use crate::{graph::Viewport, Draw, LINE, PANEL, TEAL};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Interaction {
    #[default]
    Absolute,
    Relative,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    previous: (f32, f32),
    value: (f32, f32),
    mode: Interaction,
    offset: (f32, f32),
}

pub fn normalized(rect: (f32, f32, f32, f32), pointer: (f32, f32)) -> (f32, f32) {
    Viewport::from_tuple(rect).to_normalized_clamped(pointer.0, pointer.1)
}

impl Drag {
    pub fn begin(
        rect: (f32, f32, f32, f32),
        pointer: (f32, f32),
        current: (f32, f32),
        mode: Interaction,
    ) -> Self {
        Self {
            previous: pointer,
            mode,
            offset: (0.0, 0.0),
            value: if mode == Interaction::Absolute {
                normalized(rect, pointer)
            } else {
                current
            },
        }
    }

    pub fn value(self) -> (f32, f32) {
        self.value
    }

    pub fn update(
        &mut self,
        rect: (f32, f32, f32, f32),
        pointer: (f32, f32),
        fine: bool,
    ) -> (f32, f32) {
        let raw = Viewport::from_tuple(rect).to_normalized(pointer.0, pointer.1);
        if self.mode == Interaction::Absolute && !fine {
            self.value = (
                (raw.0 + self.offset.0).clamp(0.0, 1.0),
                (raw.1 + self.offset.1).clamp(0.0, 1.0),
            );
            self.previous = pointer;
            return self.value;
        }
        let scale = if fine { 0.1 } else { 1.0 };
        self.value.0 = (self.value.0 + (pointer.0 - self.previous.0) / rect.2.max(1.0) * scale)
            .clamp(0.0, 1.0);
        self.value.1 = (self.value.1 - (pointer.1 - self.previous.1) / rect.3.max(1.0) * scale)
            .clamp(0.0, 1.0);
        self.previous = pointer;
        if self.mode == Interaction::Absolute {
            self.offset = (self.value.0 - raw.0, self.value.1 - raw.1);
        }
        self.value
    }
}

pub fn draw(d: &mut Draw, rect: (f32, f32, f32, f32), value: (f32, f32)) {
    let (x, y, w, h) = rect;
    d.rounded_rect(x, y, w, h, 8.0, PANEL);
    for i in 1..4 {
        let t = i as f32 / 4.0;
        d.line(x + t * w, y, x + t * w, y + h, LINE, 1.0);
        d.line(x, y + t * h, x + w, y + t * h, LINE, 1.0);
    }
    let (px, py) = Viewport::from_tuple(rect)
        .from_normalized(value.0.clamp(0.0, 1.0), value.1.clamp(0.0, 1.0));
    d.line(x, py, x + w, py, TEAL, 1.0);
    d.line(px, y, px, y + h, TEAL, 1.0);
    d.circle(px, py, 6.0, TEAL, true);
    d.outline(rect, LINE);
}

#[cfg(test)]
mod tests {
    use super::*;
    const RECT: (f32, f32, f32, f32) = (20.0, 30.0, 200.0, 200.0);

    #[test]
    fn absolute_jumps_relative_retains_and_both_clamp() {
        for mode in [Interaction::Absolute, Interaction::Relative] {
            let mut drag = Drag::begin(RECT, (120.0, 130.0), (0.2, 0.8), mode);
            assert_eq!(
                drag.value(),
                if mode == Interaction::Absolute {
                    (0.5, 0.5)
                } else {
                    (0.2, 0.8)
                }
            );
            assert_eq!(drag.update(RECT, (1120.0, -870.0), false), (1.0, 1.0));
            assert_eq!(drag.update(RECT, (-1120.0, 1870.0), false), (0.0, 0.0));
        }
    }

    #[test]
    fn fine_modifier_changes_do_not_jump() {
        let mut drag = Drag::begin(RECT, (120.0, 130.0), (0.5, 0.5), Interaction::Relative);
        let first = drag.update(RECT, (140.0, 110.0), false);
        assert!((first.0 - 0.6).abs() < 1e-6);
        let fine = drag.update(RECT, (160.0, 90.0), true);
        assert!((fine.0 - 0.61).abs() < 1e-6);
        assert_eq!(drag.update(RECT, (160.0, 90.0), false), fine);
    }

    #[test]
    fn absolute_touch_stays_at_the_edge_until_pointer_reenters() {
        let mut drag = Drag::begin(RECT, (120.0, 130.0), (0.5, 0.5), Interaction::Absolute);
        assert_eq!(drag.update(RECT, (400.0, -100.0), false), (1.0, 1.0));
        assert_eq!(drag.update(RECT, (300.0, -50.0), false), (1.0, 1.0));
        assert_eq!(drag.update(RECT, (120.0, 130.0), false), (0.5, 0.5));
        let fine = drag.update(RECT, (140.0, 110.0), true);
        assert_eq!(drag.update(RECT, (140.0, 110.0), false), fine);
    }
}
