//! Velocity-map articulation routing, curve interaction, and painting.
use super::*;

impl ScdEditorView {
    pub(super) fn open_vel_map(&mut self, kit_piece: KitPieceId) {
        self.samples_open = false;
        self.close_preset_menus();
        self.sub_kick_open = false;
        self.vel_hits.clear();
        for arts in &self.params.midi_velocities {
            for velocity in arts {
                velocity.store(0, Ordering::Relaxed);
            }
        }
        self.vel_map_open = Some(kit_piece);
        self.vel_art = ALL_ART;
        self.vel_art_menu_open = false;
        self.vel_selected_node = None;
        self.vel_just_inserted = false;
        self.blur_dirty.set(true);
    }

    pub(super) fn vel_curve(&self) -> Option<VelCurve> {
        self.vel_map_open
            .map(|kit_piece| self.params.vel_maps.curve(kit_piece, self.vel_art))
    }

    pub(super) fn commit_vel_curve(&self, curve: VelCurve) {
        if let Some(kit_piece) = self.vel_map_open {
            Self::set_vel_curve_group(&self.params.vel_maps, kit_piece, self.vel_art, curve);
        }
    }

    pub(super) fn reset_vel_art(&self, kit_piece: KitPieceId) {
        Self::reset_vel_curve_group(&self.params.vel_maps, kit_piece, self.vel_art);
    }

    pub(super) fn set_vel_curve_group(
        state: &VelMapState,
        kit_piece: KitPieceId,
        art: usize,
        curve: VelCurve,
    ) {
        if art == ALL_ART {
            state.set_curve(kit_piece, ALL_ART, curve);
            return;
        }
        for member in Self::vel_art_members(kit_piece, art) {
            state.set_curve(kit_piece, member, curve.clone());
        }
    }

    pub(super) fn reset_vel_curve_group(state: &VelMapState, kit_piece: KitPieceId, art: usize) {
        if art == ALL_ART {
            state.reset_art(kit_piece, ALL_ART);
            return;
        }
        for member in Self::vel_art_members(kit_piece, art) {
            state.reset_art(kit_piece, member);
        }
    }

    pub(super) fn vel_art_options(kit_piece: KitPieceId) -> Vec<(usize, String)> {
        let mut options = vec![(ALL_ART, "ALL".to_string())];
        if kit_piece == KitPieceId::Hihat {
            options.extend([
                (0, "HH Splash".to_string()),
                (1, "HH Stomp".to_string()),
                (2, "HH Shoulder".to_string()),
                (7, "HH Tip".to_string()),
            ]);
        } else {
            options.extend(
                stonehouse()
                    .arts(kit_piece)
                    .iter()
                    .enumerate()
                    .map(|(i, art)| (i, art.name.replace('_', " "))),
            );
        }
        options
    }

    pub(super) fn vel_art_members(kit_piece: KitPieceId, art: usize) -> Vec<usize> {
        if art == ALL_ART {
            return (0..stonehouse().arts(kit_piece).len()).collect();
        }
        match (kit_piece, art) {
            (KitPieceId::Hihat, 2) => (2..7).collect(),
            (KitPieceId::Hihat, 7) => (7..12).collect(),
            _ => vec![art],
        }
    }

    pub(super) fn node_delete_pos(curve: &VelCurve, i: usize) -> (f32, f32) {
        let node = &curve.nodes[i];
        let (nx, ny) = Self::norm_to_graph(node.x, node.y);
        let (ix, iy) = Self::norm_to_graph(node.in_handle.x, node.in_handle.y);
        let (ox, oy) = Self::norm_to_graph(node.out_handle.x, node.out_handle.y);
        let (mut dx, mut dy) = (ox - ix, oy - iy);
        if dx.hypot(dy) < 1.0 {
            let (ax, ay) = Self::norm_to_graph(curve.nodes[i - 1].x, curve.nodes[i - 1].y);
            let (bx, by) = Self::norm_to_graph(curve.nodes[i + 1].x, curve.nodes[i + 1].y);
            dx = bx - ax;
            dy = by - ay;
        }
        let len = dx.hypot(dy).max(1.0);
        let offset = (-dy / len * 20.0, dx / len * 20.0);
        let (gx, gy, gw, gh) = Self::vel_map_graph();
        let candidate = (nx + offset.0, ny + offset.1);
        if Self::hit(
            candidate.0,
            candidate.1,
            gx + 8.0,
            gy + 8.0,
            gw - 16.0,
            gh - 16.0,
        ) {
            candidate
        } else {
            (nx - offset.0, ny - offset.1)
        }
    }

    pub(super) fn hit_vel_delete_x(&self, curve: &VelCurve, x: f32, y: f32) -> Option<usize> {
        let i = self.vel_selected_node?;
        if i == 0 || i + 1 >= curve.nodes.len() {
            return None;
        }
        let (px, py) = Self::node_delete_pos(curve, i);
        ((px - x).hypot(py - y) <= 8.0).then_some(i)
    }

    pub(super) fn hit_vel_node(&self, curve: &VelCurve, x: f32, y: f32) -> Option<usize> {
        const R: f32 = 8.0;
        curve
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                let (px, py) = Self::norm_to_graph(n.x, n.y);
                let d = (px - x).hypot(py - y);
                (d <= R).then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    pub(super) fn hit_vel_handle(&self, curve: &VelCurve, x: f32, y: f32) -> Option<(usize, bool)> {
        let i = self.vel_selected_node?;
        let node = curve.nodes.get(i)?;
        const R: f32 = 6.0;
        let last = curve.nodes.len().saturating_sub(1);
        let (ox, oy) = Self::norm_to_graph(node.out_handle.x, node.out_handle.y);
        let (ix, iy) = Self::norm_to_graph(node.in_handle.x, node.in_handle.y);
        let out_d = (ox - x).hypot(oy - y);
        let in_d = (ix - x).hypot(iy - y);
        let out_ok = i < last && out_d <= R;
        let in_ok = i > 0 && in_d <= R;
        if out_ok && (!in_ok || out_d <= in_d) {
            Some((i, true))
        } else if in_ok {
            Some((i, false))
        } else {
            None
        }
    }

    pub(super) fn draw_velocity_map(&self, draw: &mut Draw<'_>, kit_vu: &[f32; KitPieceId::COUNT]) {
        let Some(kit_piece) = self.vel_map_open else {
            return;
        };
        let (modal_x, modal_y, modal_w, modal_h) = Self::vel_map_modal();
        paint_glass_modal(
            draw,
            &self.blur_shot,
            &self.blur_src,
            &self.blur_dst,
            &self.blur_dirty,
            modal_x,
            modal_y,
            modal_w,
            modal_h,
        );
        draw_all_chan_meters(draw, kit_vu);
        draw.text_centered(
            modal_x + modal_w * 0.5,
            modal_y + 26.0,
            &format!("{} velocity map", kit_piece.name()),
            14.0,
            THEME,
        );
        draw.text(
            modal_x + 24.0,
            modal_y + 48.0,
            "Maps incoming MIDI velocity to sample layers.",
            11.0,
            THEME,
        );
        draw.text(
            modal_x + 24.0,
            modal_y + 64.0,
            "Click the curve to add a point. Drag nodes and handles to shape it.",
            11.0,
            THEME,
        );
        let arts = stonehouse().arts(kit_piece);
        let members = Self::vel_art_members(kit_piece, self.vel_art);
        let art_layers: u32 = members
            .iter()
            .filter_map(|i| arts.get(*i))
            .map(|a| a.layers)
            .sum();
        let piece_layers: u32 = arts.iter().map(|a| a.layers).sum();
        let art_label = Self::vel_art_options(kit_piece)
            .into_iter()
            .find(|(art, _)| *art == self.vel_art)
            .map(|(_, label)| label)
            .unwrap_or_else(|| "Hit".to_string());
        draw.text(
            modal_x + 24.0,
            modal_y + 88.0,
            &format!(
                "{art_layers} samples in {art_label}  ·  {piece_layers} in {}",
                kit_piece.name()
            ),
            11.0,
            THEME,
        );
        let options = Self::vel_art_options(kit_piece);
        let (tx, ty, tw, th) = Self::vel_art_dropdown_rect();
        draw.rounded_rect(tx, ty, tw, th, 4.0, SOF_OFF);
        draw.text(tx + 10.0, ty + th * 0.5 + 4.0, &art_label, 11.0, THEME);
        draw_spinner(draw, tx + tw - 12.0, ty + th * 0.5, THEME);
        let curve = self.params.vel_maps.curve(kit_piece, self.vel_art);
        draw_vel_map_graph(draw, &curve, self.vel_selected_node);
        let (gx, gy, gw, gh) = Self::vel_map_graph();
        for &(piece, art, velocity, at) in &self.vel_hits {
            if piece == kit_piece && members.contains(&art) {
                let alpha = midi_hit_alpha(at.elapsed().as_secs_f32());
                let x = gx + (velocity as f32 - 1.0) / 126.0 * gw;
                draw.line(x, gy, x, gy + gh, Color::rgbaf(1.0, 0.85, 0.4, alpha), 1.5);
            }
        }
        // Paint the open menu last, over the graph.
        if self.vel_art_menu_open {
            let (x, y, w, h) = Self::vel_art_item_rect(0);
            draw.rounded_rect(x, y, w, h * options.len() as f32, 4.0, HEADER);
            draw.outline_rounded(x, y, w, h * options.len() as f32, 4.0, THEME_DIM, 1.0);
            for (i, (art, label)) in options.iter().enumerate() {
                let (x, y, w, h) = Self::vel_art_item_rect(i);
                if *art == self.vel_art {
                    draw.rounded_rect(x + 2.0, y, w - 4.0, h, 3.0, THEME_DIM);
                }
                draw.text(x + 10.0, y + h * 0.5 + 4.0, label, 11.0, THEME);
            }
        }
    }
}

pub(super) fn midi_hit_alpha(age: f32) -> f32 {
    (1.0 - (age - 0.5).max(0.0) / 2.0).clamp(0.0, 1.0)
}

fn draw_vel_map_graph(draw: &mut Draw<'_>, curve: &VelCurve, selected: Option<usize>) {
    let (gx, gy, gw, gh) = ScdEditorView::vel_map_graph();
    draw.rounded_rect(gx, gy, gw, gh, 4.0, THEME_DIM);
    let to_pt = |nx: f32, ny: f32| ScdEditorView::norm_to_graph(nx, ny);
    let pts: Vec<(f32, f32)> = curve
        .sample_points(24)
        .into_iter()
        .map(|(nx, ny)| to_pt(nx, ny))
        .collect();
    if pts.len() >= 2 {
        draw.area(&pts, gy + gh, ENV_FILL);
        draw.poly(&pts, ENV_LINE, 1.4);
    }
    if let Some(i) = selected {
        if let Some(node) = curve.nodes.get(i) {
            let (nx, ny) = to_pt(node.x, node.y);
            if i + 1 < curve.nodes.len() {
                let (ox, oy) = to_pt(node.out_handle.x, node.out_handle.y);
                draw.line(nx, ny, ox, oy, THEME_DIM, 1.0);
                draw.circle(ox, oy, 4.0, THEME, true);
            }
            if i > 0 {
                let (ix, iy) = to_pt(node.in_handle.x, node.in_handle.y);
                draw.line(nx, ny, ix, iy, THEME_DIM, 1.0);
                draw.circle(ix, iy, 4.0, THEME, true);
            }
        }
    }
    if let Some(i) = selected.filter(|i| *i > 0 && *i + 1 < curve.nodes.len()) {
        let (dx, dy) = ScdEditorView::node_delete_pos(curve, i);
        draw_delete_x(draw, dx, dy);
    }
    for (i, node) in curve.nodes.iter().enumerate() {
        let (nx, ny) = to_pt(node.x, node.y);
        let r = if selected == Some(i) { 6.0 } else { 5.0 };
        draw.circle(nx, ny, r, THEME, true);
    }
}

fn draw_delete_x(draw: &mut Draw<'_>, cx: f32, cy: f32) {
    let r = 4.5;
    draw.line(cx - r, cy - r, cx + r, cy + r, THEME, 1.3);
    draw.line(cx - r, cy + r, cx + r, cy - r, THEME, 1.3);
}
