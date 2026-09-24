use super::*;

fn draw_icon_button(d: &mut Draw, r: (f32, f32, f32, f32), fill: C) {
    d.rounded_rect(r.0, r.1, r.2, r.3, 4.0, fill);
}

fn draw_node_chrome(
    d: &mut Draw,
    node_x: f32,
    node_y: f32,
    layout: NodeChromeLayout,
    gx: f32,
    gw: f32,
    is_solo: bool,
) {
    let (solo, close) = node_chrome_rects(node_x, node_y, layout, gx, gw);
    draw_icon_button(
        d,
        solo,
        if is_solo {
            C::rgba(110, 85, 30, 180)
        } else {
            C::rgba(35, 40, 48, 150)
        },
    );
    d.headphones(
        solo.0 + solo.2 * 0.5,
        solo.1 + solo.3 * 0.5,
        if is_solo { GOLD } else { MUTED },
    );
    draw_icon_button(d, close, C::rgba(35, 40, 48, 150));
    d.close_icon(close.0 + close.2 * 0.5, close.1 + close.3 * 0.5, MUTED);
}

fn draw_main_row(
    d: &mut Draw,
    bx: f32,
    by: f32,
    enabled: bool,
    color: C,
    shape_label: &str,
    is_cut: bool,
    order: u8,
    freq: f64,
    gain_text: &str,
    gain_active: bool,
    q_text: &str,
    q_active: bool,
    value_rects: [(f32, f32, f32, f32); 3],
    bypass_hovered: bool,
    bypass_click: f32,
) {
    let bypass = hud_bypass_rect(bx, by);
    let shape = hud_shape_rect(bx, by, 0.0);
    d.bypass_button(bypass, !enabled, color, bypass_hovered, bypass_click);
    let baseline = hud_control_text_y();
    for (r, label, value, active) in [
        (value_rects[0], "FREQ", hz(freq), true),
        (value_rects[1], "GAIN", gain_text.to_string(), gain_active),
        (value_rects[2], "Q", q_text.to_string(), q_active),
    ] {
        let label_w = if label == "Q" { 20.0 } else { 52.0 };
        d.text(r.0 + 4.0, baseline, label, HUD_LABEL_SIZE, MUTED);
        d.text(
            r.0 + 4.0 + label_w,
            baseline,
            &value,
            HUD_VALUE_SIZE,
            if enabled && active { color } else { MUTED },
        );
    }
    draw_icon_button(d, shape, C::rgba(35, 40, 48, 150));
    d.text(
        shape.0 + 10.0,
        baseline,
        shape_label,
        HUD_SHAPE_TEXT,
        if enabled { TEXT } else { MUTED },
    );
    if is_cut {
        let order_r = hud_order_rect(bx, by);
        draw_icon_button(d, order_r, C::rgba(35, 40, 48, 150));
        d.text(
            order_r.0 + 8.0,
            baseline,
            &format!("{} dB/oct ▾", order * 6),
            HUD_LABEL_SIZE,
            if enabled { TEXT } else { MUTED },
        );
    }
}

fn draw_dyn_row(
    d: &mut Draw,
    b: &Band,
    color: C,
    range: f64,
    gx: f32,
    gw: f32,
    highlighted: Option<usize>,
) {
    for (i, label, value, n, bipolar) in [
        (
            1,
            "RANGE",
            format!("{:.1}", b.range),
            ((b.range + MAX_RANGE_DB) / (2.0 * MAX_RANGE_DB)) as f32,
            true,
        ),
        (
            2,
            "RATIO",
            format!("{:.1}:1", b.ratio),
            ((b.ratio - 1.0) / 19.0) as f32,
            false,
        ),
        (
            3,
            "ATTACK",
            format!("{:.1}", b.attack),
            (b.attack / 0.1).log(2000.0) as f32,
            false,
        ),
        (
            4,
            "RELEASE",
            format!("{:.0}", b.release),
            (b.release / 10.0).log(200.0) as f32,
            false,
        ),
    ] {
        let r = hud_dyn_field_rect_at(b, range, i, gx, gw);
        let val_r = hud_dyn_value_rect_at(b, range, i, gx, gw);
        let text_y = hud_control_text_y();
        if i != 1 {
            d.outline(r, if highlighted == Some(i) { color } else { LINE });
        }
        d.text(r.0 + 8.0, text_y, label, 15.0, MUTED);
        d.text_centered(
            val_r.0 + val_r.2 * 0.5,
            text_y,
            &value,
            HUD_VALUE_SIZE,
            if b.enabled { color } else { MUTED },
        );
        if i == 1 {
            continue;
        }
        let bar_x = r.0 + 8.0;
        let bar_w = r.2 - 16.0;
        let bar_y = r.1 + 24.0;
        let bar_h = 5.0;
        d.rect(bar_x, bar_y, bar_w, bar_h, LINE);
        let n = n.clamp(0.0, 1.0);
        let fill_color = if b.enabled { color } else { MUTED };
        if bipolar {
            let mid_x = bar_x + bar_w * 0.5;
            if n > 0.5 {
                d.rect(mid_x, bar_y, bar_w * (n - 0.5), bar_h, fill_color);
            } else if n < 0.5 {
                let fill_w = bar_w * (0.5 - n);
                d.rect(mid_x - fill_w, bar_y, fill_w, bar_h, fill_color);
            }
            d.line(
                mid_x,
                bar_y - 1.0,
                mid_x,
                bar_y + bar_h + 1.0,
                if b.enabled { TEXT } else { MUTED },
                1.0,
            );
        } else {
            d.rect(bar_x, bar_y, bar_w * n, bar_h, fill_color);
        }
    }
}

fn draw_right_cluster(
    d: &mut Draw,
    g: HudGeom,
    gx: f32,
    gw: f32,
    enabled: bool,
    color: C,
    dyn_on: bool,
    dyn_hover: bool,
    dyn_click: f32,
    cog_hover: bool,
    page_on: bool,
) {
    if !g.show_dyn_btn {
        return;
    }
    if g.show_cog {
        let cog = hud_cog_rect_at(gx, gw, g.by, g.bh);
        let cog_color = if !enabled {
            MUTED
        } else if cog_hover {
            color
        } else {
            TEXT
        };
        if page_on {
            d.text_centered(
                cog.0 + cog.2 * 0.5,
                cog.1 + cog.3 * 0.5 + 5.0,
                ">",
                14.0,
                cog_color,
            );
        } else {
            d.cog_icon(cog.0 + cog.2 * 0.5, cog.1 + cog.3 * 0.5, cog_color);
        }
    }
    let (cx, cy) = hud_dyn_btn_center(g, gx, gw);
    let dyn_r = hud_dyn_btn_rect_at(gx, gw, g.by, g.bh);
    let mut fill = C::rgba(12, 12, 14, 230);
    if dyn_click > 0.01 {
        fill.a = 1.0;
    }
    d.rounded_rect(dyn_r.0, dyn_r.1, dyn_r.2, dyn_r.3, 6.0, fill);
    let ring = if !enabled {
        MUTED
    } else if dyn_on {
        color
    } else if dyn_hover {
        TEXT
    } else {
        MUTED
    };
    d.outline_rounded(dyn_r.0, dyn_r.1, dyn_r.2, dyn_r.3, 6.0, ring, 1.2);
    d.text_centered(cx, cy + HUD_DYN_TEXT * 0.32, "DYN", HUD_DYN_TEXT, ring);
}

pub(super) fn band_hud(
    d: &mut Draw,
    b: &Band,
    is_solo: bool,
    color: C,
    range: f64,
    gx: f32,
    gw: f32,
    dyn_hover: bool,
    dyn_click: f32,
    cog_hover: bool,
    page_eased: f32,
    page_on: bool,
    eq_clip: (f32, f32, f32, f32),
    bypass_hovered: bool,
    bypass_click: f32,
    highlighted_dyn_field: Option<usize>,
) {
    let g = hud_geom_for_at(b, range, gx, gw);
    draw_right_cluster(
        d, g, gx, gw, b.enabled, color, b.dynamic, dyn_hover, dyn_click, cog_hover, page_on,
    );
    let saved = d.offset_x;
    // Expand left so bypass hover/click halo is not clipped at the HUD content edge.
    let local = (
        g.bx + saved - HUD_BYPASS_HALO_PAD,
        g.by,
        g.bw + HUD_BYPASS_HALO_PAD,
        g.bh,
    );
    if let Some(clip) = intersect_rect(eq_clip, local) {
        d.scissor(clip.0, clip.1, clip.2, clip.3);
        let eased = quintic_page_progress(page_eased);
        d.offset_x = saved + (0.0 - eased) * g.bw;
        let gain_text = if b.shape.has_gain() {
            format!("{:+.2}", b.gain)
        } else {
            "—".into()
        };
        let q_text = if b.shape.is_cut() && b.order == 1 {
            "—".into()
        } else {
            format!("{:.2}", b.q)
        };
        draw_main_row(
            d,
            g.bx,
            g.by,
            b.enabled,
            color,
            &format!("{} ▾", b.shape.name()),
            b.shape.is_cut(),
            b.order,
            b.freq,
            &gain_text,
            b.shape.has_gain(),
            &q_text,
            !(b.shape.is_cut() && b.order == 1),
            [
                hud_value_rect_at(b, range, 0, gx, gw),
                hud_value_rect_at(b, range, 1, gx, gw),
                hud_value_rect_at(b, range, 2, gx, gw),
            ],
            bypass_hovered,
            bypass_click,
        );
        d.offset_x = saved + (1.0 - eased) * g.bw;
        draw_dyn_row(d, b, color, range, gx, gw, highlighted_dyn_field);
        d.offset_x = saved;
        d.scissor(eq_clip.0, eq_clip.1, eq_clip.2, eq_clip.3);
    }
    let node_x = freq_x_at(b.freq, gx, gw);
    let node_y = db_y(if b.shape.has_gain() { b.gain } else { 0.0 }, range);
    draw_node_chrome(
        d,
        node_x,
        node_y,
        band_chrome_layout(b, range, gx, gw, true),
        gx,
        gw,
        is_solo,
    );
}

pub(super) fn band_hud_lift(
    d: &mut Draw,
    b: &LiftBand,
    is_solo: bool,
    color: C,
    range: f64,
    gx: f32,
    gw: f32,
    bypass_hovered: bool,
    bypass_click: f32,
) {
    let (bx, by, _, _) = hud_rect_for_lift_at(b, range, gx, gw);
    let gain_text = if b.gain <= -99.5 {
        "-inf".into()
    } else {
        format!("{:.1}", b.gain)
    };
    let q_text = if b.shape.is_cut() && b.order == 1 {
        "—".into()
    } else {
        format!("{:.2}", b.q)
    };
    draw_main_row(
        d,
        bx,
        by,
        b.enabled,
        color,
        &format!("{} LIFT ▾", b.shape.uppercase_name()),
        b.shape.is_cut(),
        b.order,
        b.freq,
        &gain_text,
        true,
        &q_text,
        !(b.shape.is_cut() && b.order == 1),
        [
            hud_value_rect_lift_at(b, range, 0, gx, gw),
            hud_value_rect_lift_at(b, range, 1, gx, gw),
            hud_value_rect_lift_at(b, range, 2, gx, gw),
        ],
        bypass_hovered,
        bypass_click,
    );
    let node_x = freq_x_at(b.freq, gx, gw);
    let node_y = lift_gain_y(b.gain);
    draw_node_chrome(
        d,
        node_x,
        node_y,
        if node_chrome_above(b.gain, None) {
            NodeChromeLayout::Above
        } else {
            NodeChromeLayout::Below
        },
        gx,
        gw,
        is_solo,
    );
}
