/// Cursor used for idle highlights.
///
/// While a drag is active this is `None`, so other controls do not light up
/// under the pointer.
pub fn idle_hover(hover: Option<(f32, f32)>, dragging: bool) -> Option<(f32, f32)> {
    hover.filter(|_| !dragging)
}

/// Map a Vizia mouse position into design-space coordinates for a full-bleed view.
///
/// `bounds` and `cursor` must share the same space (Vizia physical pixels).
/// `design_w` is the artwork width used when constructing [`crate::Draw`]
/// (`scale = bounds.w / design_w`). Inverse of `Draw`'s `ox + x * s` mapping.
pub fn local_xy(
    bounds_x: f32,
    bounds_y: f32,
    bounds_w: f32,
    design_w: f32,
    cursor_x: f32,
    cursor_y: f32,
) -> Option<(f32, f32)> {
    if !bounds_w.is_finite() || bounds_w < 8.0 || !design_w.is_finite() || design_w <= 0.0 {
        return None;
    }
    let scale = bounds_w / design_w;
    if !scale.is_finite() || scale <= 0.01 {
        return None;
    }
    Some(((cursor_x - bounds_x) / scale, (cursor_y - bounds_y) / scale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_hover_clears_while_dragging() {
        assert_eq!(idle_hover(Some((10.0, 20.0)), false), Some((10.0, 20.0)));
        assert_eq!(idle_hover(Some((10.0, 20.0)), true), None);
        assert_eq!(idle_hover(None, false), None);
    }

    #[test]
    fn local_xy_inverts_draw_scale_mapping() {
        let bounds_x = 12.0;
        let bounds_y = 8.0;
        let design_w = 1000.0;
        let scale = 1.5;
        let bounds_w = design_w * scale;
        let design = (250.0, 100.0);
        let cursor = (bounds_x + design.0 * scale, bounds_y + design.1 * scale);
        assert_eq!(
            local_xy(bounds_x, bounds_y, bounds_w, design_w, cursor.0, cursor.1),
            Some(design)
        );
        assert_eq!(local_xy(0.0, 0.0, 0.0, design_w, cursor.0, cursor.1), None);
    }
}

/// A readout press becomes a drag after a small movement in logical UI pixels.
/// Once dragging, returning to the press position must not turn it back into a click.
#[derive(Clone, Copy, Debug)]
pub struct ValuePress<T> {
    pub target: T,
    pub rect: (f32, f32, f32, f32),
    pub origin: (f32, f32),
    dragged: bool,
}

impl<T> ValuePress<T> {
    pub fn new(target: T, rect: (f32, f32, f32, f32), origin: (f32, f32)) -> Self {
        Self {
            target,
            rect,
            origin,
            dragged: false,
        }
    }

    pub fn update(&mut self, x: f32, y: f32) -> bool {
        self.dragged |= (x - self.origin.0).hypot(y - self.origin.1) >= 4.0;
        self.dragged
    }

    pub fn released_as_click(&mut self, x: f32, y: f32) -> bool {
        !self.update(x, y)
            && x >= self.rect.0
            && x <= self.rect.0 + self.rect.2
            && y >= self.rect.1
            && y <= self.rect.1 + self.rect.3
    }
}

#[cfg(test)]
mod value_press_tests {
    use super::*;

    #[test]
    fn click_tolerates_jitter_but_requires_release_inside() {
        let mut press = ValuePress::new((), (0.0, 0.0, 80.0, 20.0), (20.0, 10.0));
        assert!(!press.update(21.0, 11.0));
        assert!(press.released_as_click(22.0, 10.0));
        let mut edge = ValuePress::new((), (0.0, 0.0, 80.0, 20.0), (79.0, 10.0));
        assert!(!edge.released_as_click(81.0, 10.0));
    }

    #[test]
    fn drag_in_either_direction_never_opens_text_on_release() {
        for (dx, dy) in [(5.0, 0.0), (-5.0, 0.0), (0.0, 5.0), (0.0, -5.0)] {
            let mut press = ValuePress::new((), (0.0, 0.0, 80.0, 20.0), (20.0, 10.0));
            assert!(press.update(20.0 + dx, 10.0 + dy));
            assert!(!press.released_as_click(20.0, 10.0));
        }
    }

    #[test]
    fn distant_release_without_move_event_is_not_a_click() {
        let mut press = ValuePress::new((), (0.0, 0.0, 80.0, 20.0), (20.0, 10.0));
        assert!(!press.released_as_click(50.0, 10.0));
    }
}

/// Relative readout movement: right or up increases, with no jump to the track.
pub fn readout_drag_delta(dx: f32, dy: f32, fine: bool) -> f32 {
    let movement = if dx.abs() >= dy.abs() { dx } else { -dy };
    movement * if fine { 0.0004 } else { 0.004 }
}

#[cfg(test)]
mod readout_drag_tests {
    use super::*;

    #[test]
    fn readout_drag_supports_both_axes_and_fine_control() {
        assert_eq!(
            readout_drag_delta(10.0, 0.0, false),
            readout_drag_delta(0.0, -10.0, false)
        );
        assert!(readout_drag_delta(-10.0, 0.0, false) < 0.0);
        assert!(readout_drag_delta(0.0, 10.0, false) < 0.0);
        assert!(
            (readout_drag_delta(10.0, 0.0, true) * 10.0 - readout_drag_delta(10.0, 0.0, false))
                .abs()
                < 0.0001
        );
        assert_eq!(readout_drag_delta(0.0, 0.0, false), 0.0);
    }
}
