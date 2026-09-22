/// Cursor used for idle highlights.
///
/// While a drag is active this is `None`, so other controls do not light up
/// under the pointer.
pub fn idle_hover(hover: Option<(f32, f32)>, dragging: bool) -> Option<(f32, f32)> {
    hover.filter(|_| !dragging)
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
}
