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
    Some((
        (cursor_x - bounds_x) / scale,
        (cursor_y - bounds_y) / scale,
    ))
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
        assert_eq!(
            local_xy(0.0, 0.0, 0.0, design_w, cursor.0, cursor.1),
            None
        );
    }
}
