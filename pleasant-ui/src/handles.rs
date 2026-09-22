/// Short side of a grab tag. Vertical and horizontal tags share this shape.
pub const TAG_THICK: f32 = 13.0;
/// Long side of a grab tag. Rotated 90° between cut tags and strength tags.
pub const TAG_LONG: f32 = 30.0;
pub const TAG_RADIUS: f32 = 3.5;
/// Triangle base (the edge attached to the body).
pub const TAG_POINTER_BASE: f32 = 9.0;
/// How far the triangle sticks out toward the line.
pub const TAG_POINTER_LEN: f32 = 6.0;
const TAG_HIT_PAD: f32 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagPointer {
    /// Triangle on the bottom, body above. Vertical cut lines.
    Down,
    /// Triangle on the right, body to the left. Horizontal strength lines.
    Right,
    /// Body centered on the anchor, no triangle. On-line mouse grabs.
    None,
}

pub fn point_in_rect(x: f32, y: f32, r: (f32, f32, f32, f32)) -> bool {
    x >= r.0 && x <= r.0 + r.2 && y >= r.1 && y <= r.1 + r.3
}

/// `(width, height)` of the rounded body. Down is Right rotated 90°.
pub fn tag_body_size(pointer: TagPointer) -> (f32, f32) {
    match pointer {
        TagPointer::Down => (TAG_THICK, TAG_LONG),
        TagPointer::Right | TagPointer::None => (TAG_LONG, TAG_THICK),
    }
}

/// Filled rounded rectangle only (no pointer).
pub fn tag_body_rect(anchor_x: f32, anchor_y: f32, pointer: TagPointer) -> (f32, f32, f32, f32) {
    let (w, h) = tag_body_size(pointer);
    match pointer {
        TagPointer::Down => (anchor_x - w * 0.5, anchor_y - TAG_POINTER_LEN - h, w, h),
        TagPointer::Right => (anchor_x - TAG_POINTER_LEN - w, anchor_y - h * 0.5, w, h),
        TagPointer::None => (anchor_x - w * 0.5, anchor_y - h * 0.5, w, h),
    }
}

/// Hit target including the pointer and a little padding.
pub fn tag_hit_rect(anchor_x: f32, anchor_y: f32, pointer: TagPointer) -> (f32, f32, f32, f32) {
    let (w, h) = tag_body_size(pointer);
    let (x, y, w, h) = match pointer {
        TagPointer::Down => (
            anchor_x - w * 0.5,
            anchor_y - TAG_POINTER_LEN - h,
            w,
            h + TAG_POINTER_LEN,
        ),
        TagPointer::Right => (
            anchor_x - TAG_POINTER_LEN - w,
            anchor_y - h * 0.5,
            w + TAG_POINTER_LEN,
            h,
        ),
        TagPointer::None => tag_body_rect(anchor_x, anchor_y, pointer),
    };
    (
        x - TAG_HIT_PAD,
        y - TAG_HIT_PAD,
        w + TAG_HIT_PAD * 2.0,
        h + TAG_HIT_PAD * 2.0,
    )
}

pub fn tag_contains(anchor_x: f32, anchor_y: f32, pointer: TagPointer, x: f32, y: f32) -> bool {
    point_in_rect(x, y, tag_hit_rect(anchor_x, anchor_y, pointer))
}

/// Vertical grip lines for tags you drag left/right.
pub fn tag_vertical_grips(pointer: TagPointer) -> bool {
    matches!(pointer, TagPointer::Down)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn down_tag_is_right_tag_rotated() {
        let down = tag_body_size(TagPointer::Down);
        let right = tag_body_size(TagPointer::Right);
        assert!((down.0 - right.1).abs() < 0.01);
        assert!((down.1 - right.0).abs() < 0.01);
        assert!(down.1 > down.0);
    }

    #[test]
    fn down_tag_sits_above_anchor() {
        let r = tag_body_rect(100.0, 50.0, TagPointer::Down);
        assert!(r.1 + r.3 <= 50.0);
        assert!((r.0 + r.2 * 0.5 - 100.0).abs() < 0.01);
        assert!(tag_contains(100.0, 50.0, TagPointer::Down, 100.0, 40.0));
        assert!(!tag_contains(100.0, 50.0, TagPointer::Down, 100.0, 80.0));
    }

    #[test]
    fn right_tag_sits_left_of_anchor() {
        let r = tag_body_rect(80.0, 200.0, TagPointer::Right);
        assert!(r.0 + r.2 <= 80.0);
        assert!((r.1 + r.3 * 0.5 - 200.0).abs() < 0.01);
        assert!(tag_contains(80.0, 200.0, TagPointer::Right, 70.0, 200.0));
        assert!(!tag_contains(80.0, 200.0, TagPointer::Right, 120.0, 200.0));
    }

    #[test]
    fn none_tag_is_centered_on_anchor() {
        let r = tag_body_rect(40.0, 10.0, TagPointer::None);
        assert!((r.0 + r.2 * 0.5 - 40.0).abs() < 0.01);
        assert!((r.1 + r.3 * 0.5 - 10.0).abs() < 0.01);
    }
}
