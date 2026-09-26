//! SCD-specific artwork and control painting.
use super::*;

pub(super) fn draw_spinner(draw: &mut Draw<'_>, cx: f32, cy: f32, color: Color) {
    draw.fill_poly(
        &[(cx, cy - 4.6), (cx - 3.4, cy - 1.1), (cx + 3.4, cy - 1.1)],
        color,
    );
    draw.fill_poly(
        &[(cx, cy + 4.6), (cx - 3.4, cy + 1.1), (cx + 3.4, cy + 1.1)],
        color,
    );
}

pub(super) fn draw_chevron_right(draw: &mut Draw<'_>, cx: f32, cy: f32, color: Color) {
    draw.fill_poly(
        &[(cx + 3.2, cy), (cx - 1.6, cy - 4.0), (cx - 1.6, cy + 4.0)],
        color,
    );
}

pub(super) fn ensure_img(slot: &Cell<Option<ImageId>>, canvas: &mut Canvas, bytes: &[u8]) {
    if slot.get().is_none() {
        slot.set(
            canvas
                .load_image_mem(bytes, ImageFlags::GENERATE_MIPMAPS)
                .ok(),
        );
    }
}
pub(super) fn blit(draw: &mut Draw<'_>, id: ImageId, x: f32, y: f32, w: f32, h: f32) {
    let px = draw.ox + (x + draw.offset_x) * draw.s;
    let py = draw.oy + y * draw.s;
    let pw = w * draw.s;
    let ph = h * draw.s;
    let mut path = Path::new();
    path.rect(px, py, pw, ph);
    draw.c
        .fill_path(&path, &Paint::image(id, px, py, pw, ph, 0.0, 1.0));
}

pub(super) fn ensure_empty_image(
    canvas: &mut Canvas,
    slot: &Cell<Option<ImageId>>,
    w: usize,
    h: usize,
) -> Option<ImageId> {
    let w = w.max(1);
    let h = h.max(1);
    if let Some(id) = slot.get() {
        if canvas.image_size(id).ok() == Some((w, h)) {
            return Some(id);
        }
        canvas.delete_image(id);
    }
    let flags = ImageFlags::FLIP_Y | ImageFlags::PREMULTIPLIED;
    let id = canvas
        .create_image_empty(w, h, PixelFormat::Rgba8, flags)
        .ok()?;
    slot.set(Some(id));
    Some(id)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn capture_backdrop_blur(
    draw: &mut Draw<'_>,
    shot_slot: &Cell<Option<ImageId>>,
    src_slot: &Cell<Option<ImageId>>,
    dst_slot: &Cell<Option<ImageId>>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Option<ImageId> {
    let s = draw.s;
    let pad = (GLASS_BLUR_PAD * s).ceil();
    let px = draw.ox + (x + draw.offset_x) * s - pad;
    let py = draw.oy + y * s - pad;
    let pw = w * s + pad * 2.0;
    let ph = h * s + pad * 2.0;
    let iw = pw.ceil().max(1.0) as usize;
    let ih = ph.ceil().max(1.0) as usize;

    let shot = draw.c.screenshot().ok()?;
    let shot_id = if let Some(id) = shot_slot.get() {
        if draw.c.image_size(id).ok() == Some((shot.width(), shot.height())) {
            if draw.c.update_image(id, shot.as_ref(), 0, 0).is_err() {
                return None;
            }
            id
        } else {
            draw.c.delete_image(id);
            let id = draw
                .c
                .create_image(shot.as_ref(), ImageFlags::empty())
                .ok()?;
            shot_slot.set(Some(id));
            id
        }
    } else {
        let id = draw
            .c
            .create_image(shot.as_ref(), ImageFlags::empty())
            .ok()?;
        shot_slot.set(Some(id));
        id
    };

    let src = ensure_empty_image(draw.c, src_slot, iw, ih)?;
    let dst = ensure_empty_image(draw.c, dst_slot, iw, ih)?;
    let shot_w = shot.width() as f32;
    let shot_h = shot.height() as f32;

    draw.c.save();
    draw.c.set_render_target(RenderTarget::Image(src));
    draw.c.reset_scissor();
    draw.c.reset_transform();
    draw.c
        .clear_rect(0, 0, iw as u32, ih as u32, Color::rgba(0, 0, 0, 0));
    let mut region = Path::new();
    region.rect(0.0, 0.0, iw as f32, ih as f32);
    draw.c.fill_path(
        &region,
        &Paint::image(shot_id, -px, -py, shot_w, shot_h, 0.0, 1.0),
    );
    draw.c.filter_image(
        dst,
        ImageFilter::GaussianBlur {
            sigma: (GLASS_BLUR_SIGMA * s).clamp(10.0, 40.0),
        },
        src,
    );
    draw.c.restore();
    draw.c.set_render_target(RenderTarget::Screen);
    Some(dst)
}

pub(super) fn fill_rounded_image(
    draw: &mut Draw<'_>,
    id: ImageId,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
) {
    let s = draw.s;
    let pad = (GLASS_BLUR_PAD * s).ceil();
    let px = draw.ox + (x + draw.offset_x) * s;
    let py = draw.oy + y * s;
    let pw = w * s;
    let ph = h * s;
    let mut path = Path::new();
    path.rounded_rect(px, py, pw, ph, radius * s);
    draw.c.fill_path(
        &path,
        &Paint::image(
            id,
            px - pad,
            py - pad,
            pw + pad * 2.0,
            ph + pad * 2.0,
            0.0,
            1.0,
        ),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn fill_rounded_vertical_gradient(
    draw: &mut Draw<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    top: Color,
    bottom: Color,
) {
    let s = draw.s;
    let px = draw.ox + (x + draw.offset_x) * s;
    let py = draw.oy + y * s;
    let pw = w * s;
    let ph = h * s;
    let mut path = Path::new();
    path.rounded_rect(px, py, pw, ph, radius * s);
    draw.c.fill_path(
        &path,
        &Paint::linear_gradient(
            px,
            py,
            px,
            py + ph * 0.55,
            draw.color(top),
            draw.color(bottom),
        ),
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint_glass_modal(
    draw: &mut Draw<'_>,
    blur_shot: &Cell<Option<ImageId>>,
    blur_src: &Cell<Option<ImageId>>,
    blur_dst: &Cell<Option<ImageId>>,
    blur_dirty: &Cell<bool>,
    modal_x: f32,
    modal_y: f32,
    modal_w: f32,
    modal_h: f32,
) {
    let radius = 8.0;
    if blur_dirty.get()
        && capture_backdrop_blur(
            draw, blur_shot, blur_src, blur_dst, modal_x, modal_y, modal_w, modal_h,
        )
        .is_some()
    {
        blur_dirty.set(false);
    }

    draw.rect(0.0, 0.0, WINDOW_W, WINDOW_H, SCRIM);
    draw.rounded_rect(
        modal_x + 1.0,
        modal_y + 8.0,
        modal_w,
        modal_h,
        radius,
        GLASS_SHADOW,
    );
    if let Some(blurred) = blur_dst.get() {
        fill_rounded_image(draw, blurred, modal_x, modal_y, modal_w, modal_h, radius);
    }
    draw.rounded_rect(modal_x, modal_y, modal_w, modal_h, radius, GLASS_TINT);
    fill_rounded_vertical_gradient(
        draw,
        modal_x,
        modal_y,
        modal_w,
        modal_h,
        radius,
        GLASS_SHEEN,
        Color {
            a: 0.0,
            ..GLASS_SHEEN
        },
    );
    draw.outline_rounded(
        modal_x + 1.0,
        modal_y + 1.0,
        modal_w - 2.0,
        modal_h - 2.0,
        radius - 1.0,
        GLASS_INNER,
        1.0,
    );
    draw.outline_rounded(modal_x, modal_y, modal_w, modal_h, radius, GLASS_EDGE, 1.25);
}

/// Channel meter from `UI_Meters.js`: a thin beam whose width grows with level,
/// fading from transparent at the peak to opaque blue at the bottom.
pub(super) fn draw_all_chan_meters(draw: &mut Draw<'_>, kit_vu: &[f32; KitPieceId::COUNT]) {
    let mixer_x = ScdEditorView::mixer_x();
    for (idx, _) in KitPieceId::ALL.iter().enumerate() {
        draw_chan_meter(draw, mixer_x + idx as f32 * STRIP_W, kit_vu[idx]);
    }
}

pub(super) fn draw_chan_meter(draw: &mut Draw<'_>, strip_x: f32, level: f32) {
    let n = level.clamp(0.0, 1.0);
    if n < 0.008 {
        return;
    }
    let left_h = n * FADER_H;
    let core_w = (0.02 * left_h).max(1.2);
    let peak_y = FADER_Y + FADER_H - left_h;
    let cx = strip_x + STRIP_W * 0.5;

    let glow_w = (core_w * 5.0).min(STRIP_W * 0.55);
    fill_meter_beam(
        draw,
        cx - glow_w * 0.5,
        peak_y,
        glow_w,
        left_h,
        CHAN_METER_TOP,
        Color {
            a: 0.28,
            ..CHAN_METER_BOT
        },
    );
    fill_meter_beam(
        draw,
        cx - core_w * 0.5,
        peak_y,
        core_w,
        left_h,
        CHAN_METER_TOP,
        CHAN_METER_BOT,
    );
}

pub(super) fn fill_meter_beam(
    draw: &mut Draw<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    top: Color,
    bottom: Color,
) {
    let s = draw.s;
    let px = draw.ox + (x + draw.offset_x) * s;
    let py = draw.oy + y * s;
    let pw = w * s;
    let ph = h * s;
    let mut path = Path::new();
    path.rect(px, py, pw, ph);
    draw.c.fill_path(
        &path,
        &Paint::linear_gradient(px, py, px, py + ph, draw.color(top), draw.color(bottom)),
    );
}

pub(super) fn stroke_arc(
    draw: &mut Draw<'_>,
    center: (f32, f32),
    radius: f32,
    angles: (f32, f32),
    color: Color,
    width: f32,
) {
    let px = draw.ox + (center.0 + draw.offset_x) * draw.s;
    let py = draw.oy + center.1 * draw.s;
    let mut path = Path::new();
    path.arc(px, py, radius * draw.s, angles.0, angles.1, Solidity::Hole);
    let mut paint = Paint::color(draw.color(color));
    paint.set_line_width(width * draw.s);
    paint.set_line_cap(LineCap::Butt);
    paint.set_line_join(LineJoin::Round);
    draw.c.stroke_path(&path, &paint);
}

/// HISE rotary: 126° from 12 o'clock, screen-space y-down.
pub(super) fn hise_knob(
    draw: &mut Draw<'_>,
    cx: f32,
    cy: f32,
    size: f32,
    norm: f32,
    bipolar: bool,
    label: Option<&str>,
) {
    let start_offset = 126.0_f32.to_radians();
    let min_arc = 4.0_f32.to_radians();
    let start_angle = 144.0_f32.to_radians();
    let sweep = 252.0_f32.to_radians();
    let radius = size * 0.42;
    let n = norm.clamp(0.0, 1.0);

    stroke_arc(
        draw,
        (cx, cy),
        radius,
        (start_angle, start_angle + sweep),
        THEME,
        1.35,
    );

    let (arc_start, arc_end) = if bipolar {
        let center = start_angle + sweep * 0.5;
        if n <= 0.5 {
            let scale = 1.0 - (n / 0.5);
            let left = min_arc + scale * (start_offset - min_arc);
            (center - left, center - min_arc)
        } else {
            let scale = (n - 0.5) / 0.5;
            let right = min_arc + scale * (start_offset - min_arc);
            (center + min_arc, center + right)
        }
    } else {
        let sa = start_angle;
        let mut ea = sa + n * sweep;
        ea = ea.max(sa + 2.0 * min_arc);
        (sa, ea)
    };
    if !bipolar || (n - 0.5).abs() > f32::EPSILON {
        stroke_arc(draw, (cx, cy), radius, (arc_start, arc_end), THEME, 5.0);
    }

    if let Some(label) = label {
        let font_size = size * 2.0 / (3.4 * 3.4);
        draw.text_centered(cx, cy + font_size * 0.35, label, font_size, THEME);
    }
}

pub(super) fn bipolar_slider(draw: &mut Draw<'_>, x: f32, y: f32, w: f32, h: f32, norm: f32) {
    draw.rounded_rect(x, y, w, h, 4.0, THEME_DIM);
    draw.rect(x + w * 0.5 - 0.75, y, 1.5, h, THEME_DIM);
    draw.rounded_rect(x, y, w, h, 4.0, SLIDER_WASH);
    let n = norm.clamp(0.0, 1.0);
    let center = x + w * 0.5;
    let (fill_x, fill_w) = if n < 0.5 {
        let width = (0.5 - n) * w;
        (center - width, width)
    } else {
        (center, (n - 0.5) * w)
    };
    if fill_w > 0.4 {
        let radius = 4.0_f32;
        let left_r = if fill_x <= x + 0.51 {
            radius.min(fill_w * 0.5)
        } else {
            0.0
        };
        let right_r = if fill_x + fill_w >= x + w - 0.51 {
            radius.min(fill_w * 0.5)
        } else {
            0.0
        };
        fill_rounded_varying(
            draw,
            fill_x,
            y,
            fill_w,
            h,
            (left_r, right_r, right_r, left_r),
            THEME,
        );
    }
}

pub(super) fn fill_rounded_varying(
    draw: &mut Draw<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radii: (f32, f32, f32, f32),
    color: Color,
) {
    let s = draw.s;
    let px = draw.ox + (x + draw.offset_x) * s;
    let py = draw.oy + y * s;
    let mut path = Path::new();
    path.rounded_rect_varying(
        px,
        py,
        w * s,
        h * s,
        radii.0 * s,
        radii.1 * s,
        radii.2 * s,
        radii.3 * s,
    );
    draw.c.fill_path(&path, &Paint::color(draw.color(color)));
}

pub(super) fn draw_value_popup(draw: &mut Draw<'_>, cx: f32, y: f32, text: &str, above: bool) {
    let w = (text.len() as f32 * 6.6 + 14.0).max(40.0);
    let h = 18.0;
    let x = cx - w * 0.5;
    let py = if above {
        (y - h - 4.0).max(0.0)
    } else {
        y + 4.0
    };
    draw.rounded_rect(x, py, w, h, 3.0, HEADER);
    draw.outline_rounded(x, py, w, h, 3.0, THEME_DIM, 1.0);
    draw.text_centered(cx, py + h * 0.5 + 4.0, text, 10.0, THEME);
}

pub(super) fn format_db(gain: f32) -> String {
    let db = util::gain_to_db(gain);
    if db <= -71.5 {
        "-inf dB".to_string()
    } else {
        format!("{db:.1} dB")
    }
}

pub(super) fn five_stage_fill(on: bool, hover: bool, press: bool) -> Option<Color> {
    match (on, hover, press) {
        (_, _, true) => Some(STAGE_DOWN),
        (true, true, false) => Some(STAGE_ON_HOVER),
        (true, false, false) => Some(STAGE_ON),
        (false, true, false) => Some(STAGE_HOVER),
        (false, false, false) => None,
    }
}

pub(super) fn draw_five_stage(
    draw: &mut Draw<'_>,
    r: (f32, f32, f32, f32),
    radius: f32,
    on: bool,
    hover: bool,
    press: bool,
) {
    if let Some(fill) = five_stage_fill(on, hover, press) {
        draw.rounded_rect(r.0, r.1, r.2, r.3, radius, fill);
    }
}

pub(super) fn draw_lock(
    draw: &mut Draw<'_>,
    x: f32,
    y: f32,
    size: f32,
    locked: bool,
    hovered: bool,
) {
    let mut color = THEME;
    color.a = if hovered {
        1.0
    } else if locked {
        0.75
    } else {
        0.35
    };
    // Draw the lock directly so its shape does not depend on icon font code points.
    let body_x = x + size * 0.19;
    let body_y = y + size * 0.44;
    let body_w = size * 0.62;
    let body_h = size * 0.43;
    draw.outline_rounded(body_x, body_y, body_w, body_h, size * 0.07, color, 2.0);
    let shackle_x = if locked {
        x + size * 0.32
    } else {
        x + size * 0.51
    };
    let shackle = [
        (shackle_x, body_y),
        (shackle_x, y + size * 0.25),
        (shackle_x + size * 0.06, y + size * 0.17),
        (shackle_x + size * 0.26, y + size * 0.17),
        (shackle_x + size * 0.32, y + size * 0.25),
        (
            shackle_x + size * 0.32,
            if locked { body_y } else { body_y - size * 0.12 },
        ),
    ];
    draw.poly(&shackle, color, 2.0);
}
