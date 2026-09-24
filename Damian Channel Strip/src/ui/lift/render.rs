use super::super::*;

pub(in crate::ui) struct LiftRenderFrame<'a> {
    pub offset_lift: f32,
    pub lift_on: bool,
    pub page: usize,
    pub next_p: f32,
    pub target_p: f32,
    pub gx: f32,
    pub gw: f32,
    pub eq_b: (f32, f32, f32, f32),
    pub spectrum: &'a [(f32, f32)],
    pub sr: f64,
    pub lift_bands: &'a [LiftBand],
    pub smooth_lift_gr: &'a [f64; 256],
    pub smooth_lift_uncapped: &'a [f64; 256],
}

impl StripView {
    pub(in crate::ui) fn render_lift_page(&self, mut d: &mut Draw, frame: LiftRenderFrame<'_>) {
        let LiftRenderFrame {
            offset_lift,
            lift_on,
            page,
            next_p,
            target_p,
            gx,
            gw,
            eq_b,
            spectrum,
            sr,
            lift_bands,
            smooth_lift_gr,
            smooth_lift_uncapped,
        } = frame;
        // --- DRAW LIFT (if visible) ---
        if offset_lift > -GRAPH_CLIP_W && offset_lift < GRAPH_CLIP_W {
            d.offset_x = offset_lift;
            let lift_bypassed = !lift_on;
            for db in [0, -20, -40, -60, -80, -100] {
                let y = lift_gain_y(db as f64);
                d.line(
                    gx,
                    y,
                    gx + gw,
                    y,
                    if db == 0 { C::rgb(75, 81, 85) } else { LINE },
                    1.0,
                );
                d.text_centered(
                    gx - 22.0,
                    axis_label_y(y, 13.0),
                    &axis_db_text(db),
                    13.0,
                    MUTED,
                );
            }
            for (freq, label) in FREQ_AXIS {
                let x = freq_x_at(freq, gx, gw);
                d.line(x, GY, x, GRAPH_BOTTOM, LINE, 1.0);
                if !self.hud_visible() {
                    d.text_centered(x, EQ_AXIS_LABEL_Y, label, 12.0, MUTED);
                }
            }
            d.area(
                &spectrum,
                GY + GH,
                if lift_bypassed {
                    C::rgba(80, 90, 100, 10)
                } else {
                    C::rgba(125, 143, 159, 28)
                },
            );
            d.poly_above(
                &spectrum,
                GY + GH,
                if lift_bypassed {
                    C::rgba(90, 100, 110, 20)
                } else {
                    C::rgba(144, 161, 175, 62)
                },
                1.0,
            );

            for b in lift_bands {
                if !b.enabled {
                    continue;
                }
                let color = if lift_bypassed { MUTED } else { LIFT_COLOR };
                let points = lift_curve_points(b, &smooth_lift_gr, gx, gw, sr);
                draw_lift_influence(
                    &mut d,
                    b,
                    &points,
                    color,
                    (
                        if lift_bypassed { 0.05 } else { 0.22 },
                        if lift_bypassed { 0.25 } else { 0.90 },
                    ),
                    (gx, gw),
                );
                let overflow = !lift_bypassed
                    && smooth_lift_uncapped
                        .iter()
                        .zip(smooth_lift_gr.iter())
                        .any(|(uncapped, capped)| *uncapped > *capped + 0.15);
                if overflow {
                    let over_pts = lift_curve_points(b, &smooth_lift_uncapped, gx, gw, sr);
                    draw_lift_influence(&mut d, b, &over_pts, MUTED, (0.0, 0.55), (gx, gw));
                }
            }
            if let Some(b) = lift_bands
                .iter()
                .find(|b| b.id == self.shared.solo_id.load(Ordering::Relaxed))
            {
                let lift_pts = lift_curve_points(b, &smooth_lift_gr, gx, gw, sr);
                let lift_color = if lift_bypassed { MUTED } else { LIFT_COLOR };
                let mut fill = lift_color;
                fill.a = if lift_bypassed { 0.05 } else { 0.22 };
                draw_solo_shade(
                    &mut d,
                    b.shape,
                    b.freq,
                    b.q,
                    gx,
                    gw,
                    eq_b,
                    &lift_pts,
                    lift_color,
                    1.8,
                    Some((lift_pts.as_slice(), GY + GH, fill)),
                );
            }

            for (idx, b) in lift_bands.iter().enumerate() {
                let color = if !b.enabled || lift_bypassed {
                    MUTED
                } else {
                    LIFT_COLOR
                };
                let x = freq_x_at(b.freq, gx, gw);
                let y = lift_gain_y(b.gain);
                if Some(b.id) == self.selected {
                    d.circle(x, y, 13.0, color, false);
                }
                d.circle(x, y, 7.0, color, true);
                let label = if lift_bands.len() > 1 {
                    format!("{} LIFT {}", b.shape.uppercase_name(), idx + 1)
                } else {
                    format!("{} LIFT", b.shape.uppercase_name())
                };
                d.text(x - 12.0, y - 17.0, &label, 10.0, color);
            }

            if let Some(b) = lift_bands.iter().find(|b| Some(b.id) == self.selected) {
                let color = if !b.enabled || lift_bypassed {
                    MUTED
                } else {
                    LIFT_COLOR
                };
                let is_solo = self.is_auditioning(b.id);
                if self.hud_visible() {
                    let (bx, by, _, _) = self.hud_rect_for_lift(b);
                    let bypass_r = hud_bypass_rect(bx, by);
                    let bypass_hovered = self
                        .idle_hover()
                        .is_some_and(|(hx, hy)| inside(hx, hy, bypass_r));
                    band_hud_lift(
                        &mut d,
                        b,
                        is_solo,
                        color,
                        self.graph_db,
                        gx,
                        gw,
                        bypass_hovered,
                        self.band_bypass_anim.step(),
                    );
                }
                let c = if lift_on { LIFT_COLOR } else { MUTED };
                for (i, label, val, n) in [
                    (
                        0,
                        "THRESHOLD",
                        format!("{:.1}", b.threshold),
                        (b.threshold + 60.0) / 60.0,
                    ),
                    (
                        1,
                        "RATIO",
                        format!("{:.1}:1", b.ratio),
                        (b.ratio - 1.0) / 19.0,
                    ),
                    (
                        2,
                        "ATTACK",
                        format!("{:.1} ms", b.attack),
                        (b.attack / 0.1).log(2000.0),
                    ),
                    (
                        3,
                        "RELEASE",
                        format!("{:.0} ms", b.release),
                        (b.release / 10.0).log(200.0),
                    ),
                    (4, "RANGE", format!("{:.1}", b.range), b.range / 24.0),
                ] {
                    d.control(band_rect_at(i, gx, gw), label, &val, n as f32, c);
                }
            }

            if page == EQ_PAGE_LIFT && (next_p - target_p).abs() < 0.05 {
                if let Some((x, y)) = self.idle_hover() {
                    if inside(x, y, (gx, GY, gw, GH))
                        && self.drag.is_none()
                        && self.menu.is_none()
                        && self.edit.is_none()
                        && !lift_bypassed
                        && !self.over_selected_hud(x, y)
                        && !(self.is_lift_selected() && inside(x, y, (gx, GY, gw, LIFT_DOCK_H)))
                    {
                        let near = lift_bands.iter().any(|b| {
                            (x - freq_x_at(b.freq, gx, gw)).abs() < 15.0
                                && (y - lift_gain_y(b.gain)).abs() < 15.0
                        });
                        if !near {
                            let shape = infer_shape((x - gx) / gw, (y - GY) / GH, false);
                            if self.selected.is_none() {
                                let preview_band = LiftBand {
                                    id: 0,
                                    shape,
                                    freq: x_freq_at(x, gx, gw).clamp(20.0, 20000.0),
                                    gain: lift_y_gain(y),
                                    ..LiftBand::default()
                                };
                                let points =
                                    lift_curve_points(&preview_band, &smooth_lift_gr, gx, gw, sr);
                                draw_lift_influence(
                                    &mut d,
                                    &preview_band,
                                    &points,
                                    MUTED,
                                    (0.08, 0.40),
                                    (gx, gw),
                                );
                            }
                            draw_hover_preview(
                                &mut d,
                                x,
                                y,
                                gx,
                                gw,
                                hover_preview_label(self.selected.is_some(), shape),
                            );
                        }
                    }
                }
            }
        }
    }
}
