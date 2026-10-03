use super::{
    controls::{AXIS_LABEL_Y, DROP_TIME_SLIDER},
    *,
};

pub(super) const GRAPH_X: f32 = 60.0;
pub(super) const GRAPH_Y: f32 = 108.0;
pub(super) const GRAPH_W: f32 = 852.0 * 0.8;
pub(super) const GRAPH_H: f32 = 252.0;

const CURVE_NODE_HIT: f32 = 16.0;

impl TapeStopView {
    pub(super) fn curve_node_pos() -> (f32, f32) {
        (GRAPH_X + GRAPH_W * 0.5, GRAPH_Y + GRAPH_H * 0.5)
    }

    pub(super) fn live_playhead(speed: f32, curve_exp: f32, time_scale: f32) -> (f32, f32) {
        let y_n = 1.0 - speed.clamp(0.0, 1.0);
        let graph_time = inv_s_curve(y_n, curve_exp) * time_scale;
        // The slower channel's curve ends at the graph edge before it reaches zero speed.
        let visible_drop = if graph_time > 1.0 {
            s_curve(1.0 / time_scale, curve_exp)
        } else {
            y_n
        };
        (
            GRAPH_X + graph_time.min(1.0) * GRAPH_W,
            GRAPH_Y + visible_drop * GRAPH_H,
        )
    }

    pub(super) fn hit_curve_node(px: f32, py: f32) -> bool {
        let (cx, cy) = Self::curve_node_pos();
        let dx = px - cx;
        let dy = py - cy;
        dx * dx + dy * dy <= CURVE_NODE_HIT * CURVE_NODE_HIT
    }

    pub(super) fn draw_graph(&self, d: &mut Draw) {
        let gx = GRAPH_X;
        let gy = GRAPH_Y;
        let gw = GRAPH_W;
        let gh = GRAPH_H;
        let curve_exp = self.params.drop_curve.value();
        let drop_time = self.params.drop_time.value();
        let braking = self.telemetry.is_braking.load(Ordering::Relaxed);
        let crossfading = self.telemetry.is_crossfading.load(Ordering::Relaxed);
        let prog_l = self.telemetry.brake_progress_l.load(Ordering::Relaxed);
        let prog_r = self.telemetry.brake_progress_r.load(Ordering::Relaxed);
        let speed_l = self.telemetry.speed_l.load(Ordering::Relaxed);
        let speed_r = self.telemetry.speed_r.load(Ordering::Relaxed);

        d.rect(gx, gy, gw, gh, rgb(23, 27, 33));

        // Draw Speed grid as percentages
        for i in 0..=4 {
            let y = gy + gh * (i as f32 / 4.0);
            d.line(
                gx,
                y,
                gx + gw,
                y,
                if i == 2 { rgb(75, 82, 92) } else { LINE },
                1.0,
            );
            let pct = ((1.0 - i as f32 / 4.0) * 100.0).round() as i32;
            d.text(gx - 40.0, y + 4.0, &format!("{pct}%"), 10.5, MUTED);
        }
        for i in 0..=5 {
            let x = gx + gw * (i as f32 / 5.0);
            d.line(x, gy, x, gy + gh, LINE, 1.0);
            if i < 5 && !self.show_axis_controls {
                let t = drop_time * i as f32 / 5.0;
                d.text_centered(x, AXIS_LABEL_Y, &format_ms(t), 10.5, MUTED);
            }
        }
        d.text_centered(
            DROP_TIME_SLIDER.0 + DROP_TIME_SLIDER.2 * 0.5,
            gy - 8.0,
            "DROP",
            10.0,
            MUTED,
        );
        d.outline(
            (gx, gy, gw, gh),
            if braking || crossfading { TEAL } else { LINE },
        );

        // High-density curve sampling (2 samples per pixel) to ensure smooth curve without hard corners
        const CURVE_SAMPLES: usize = 1840;
        let mut curve = Vec::with_capacity(CURVE_SAMPLES + 1);
        for i in 0..=CURVE_SAMPLES {
            let t = i as f32 / CURVE_SAMPLES as f32;
            let y = gy + s_curve(t, curve_exp) * gh;
            curve.push((gx + t * gw, y));
        }

        // Grey line when bypassing
        let is_bypassed = self.params.bypass.value();
        let curve_color = if is_bypassed { MUTED } else { GOLD };
        let mut fill = curve_color;
        fill.a = 0.12;
        d.area(&curve, gy + gh, fill);

        let div = self.params.stereo_div.value();
        if !is_bypassed && div.abs() > 0.05 {
            let (left_color, right_color) = stereo_div_colors();
            let left = stereo_brake_points(
                gx,
                gy,
                gw,
                gh,
                curve_exp,
                stereo_time_scale(div, false),
                CURVE_SAMPLES,
            );
            let right = stereo_brake_points(
                gx,
                gy,
                gw,
                gh,
                curve_exp,
                stereo_time_scale(div, true),
                CURVE_SAMPLES,
            );
            d.poly(&left, left_color, 2.0);
            d.poly(&right, right_color, 2.0);
        }
        d.poly(&curve, curve_color, 2.0);

        let (nx, ny) = Self::curve_node_pos();
        let node_hovered = self
            .idle_hover()
            .is_some_and(|(x, y)| Self::hit_curve_node(x, y))
            || matches!(self.drag, Some(DragState::Curve { .. }));
        let target = if node_hovered { 1.0 } else { 0.0 };
        let previous = self.curve_hover_anim.get();
        let hover = previous + (target - previous) * 0.2;
        self.curve_hover_anim.set(hover);
        let mut node_color = curve_color;
        node_color.r += (TEAL.r - node_color.r) * hover;
        node_color.g += (TEAL.g - node_color.g) * hover;
        node_color.b += (TEAL.b - node_color.b) * hover;
        if hover > 0.01 {
            let mut halo_color = node_color;
            halo_color.a = 0.15 * hover;
            d.circle(nx, ny, 12.0 + 3.0 * hover, halo_color, true);
        }
        d.circle(nx, ny, 8.0 + 2.0 * hover, node_color, false);
        d.circle(nx, ny, 3.5 + hover, node_color, true);

        if let Some(edit) = &self.edit {
            if edit.target == KnobId::Curve {
                d.value_edit(edit, GOLD);
            }
        }

        if braking || speed_l < 0.99 || speed_r < 0.99 {
            let (live_x_l, y_l) =
                Self::live_playhead(speed_l, curve_exp, stereo_time_scale(div, false));
            d.circle(live_x_l, y_l, 6.0, TEAL, false);
            d.circle(live_x_l, y_l, 3.0, TEXT, true);
            if (speed_l - speed_r).abs() > 0.01 || (prog_l - prog_r).abs() > 0.01 {
                let (live_x_r, y_r) =
                    Self::live_playhead(speed_r, curve_exp, stereo_time_scale(div, true));
                d.circle(live_x_r, y_r, 6.0, COLORS[4], false);
                d.circle(live_x_r, y_r, 3.0, TEXT, true);
            }
        }

        self.draw_drop_time(d);
    }
}

/// Matches the engine: percent / 200, left faster when divergence is positive.
pub(super) fn stereo_time_scale(div_pct: f32, right: bool) -> f32 {
    let div_mult = div_pct / 200.0;
    if right {
        (1.0 + div_mult).max(0.1)
    } else {
        (1.0 - div_mult).max(0.1)
    }
}

/// Brake curve in graph time. Ends at the 0% speed crossing, or the right edge if that crossing is past the graph.
pub(super) fn stereo_brake_points(
    gx: f32,
    gy: f32,
    gw: f32,
    gh: f32,
    curve_exp: f32,
    scale: f32,
    samples: usize,
) -> Vec<(f32, f32)> {
    let scale = scale.max(0.1);
    let end_t = scale.min(1.0);
    let steps = ((samples as f32) * end_t).ceil().max(1.0) as usize;
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = (i as f32 / samples as f32).min(end_t);
        let prog = (t / scale).clamp(0.0, 1.0);
        points.push((gx + t * gw, gy + s_curve(prog, curve_exp) * gh));
    }
    points
}

fn stereo_div_colors() -> (Color, Color) {
    let mut left = rgb(78, 176, 210);
    left.a = 0.22;
    let mut right = rgb(132, 126, 214);
    right.a = 0.22;
    (left, right)
}
