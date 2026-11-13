use super::*;

impl ScdEditorView {
    pub(super) fn handle_vel_map_mouse_down(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        x: f32,
        y: f32,
    ) -> bool {
        if self.vel_map_open.is_none() {
            return false;
        }
        let (modal_x, modal_y, modal_w, modal_h) = Self::vel_map_modal();
        if Self::hit(x, y, modal_x, modal_y, modal_w, modal_h) {
            if let Some(kit_piece) = self.vel_map_open {
                let (dx, dy, dw, dh) = Self::vel_art_dropdown_rect();
                if Self::hit(x, y, dx, dy, dw, dh) {
                    self.vel_art_menu_open = !self.vel_art_menu_open;
                    self.vel_ignore_up = true;
                    consume_redraw(cx, window_event, meta);
                    return true;
                }
                if self.vel_art_menu_open {
                    for (i, (art, _)) in Self::vel_art_options(kit_piece).iter().enumerate() {
                        let (sx, sy, sw, sh) = Self::vel_art_item_rect(i);
                        if Self::hit(x, y, sx, sy, sw, sh) {
                            self.vel_art = *art;
                            self.vel_selected_node = None;
                            break;
                        }
                    }
                    self.vel_art_menu_open = false;
                    self.vel_ignore_up = true;
                    consume_redraw(cx, window_event, meta);
                    return true;
                }
            }
            if let Some(curve) = self.vel_curve() {
                if let Some(i) = self.hit_vel_delete_x(&curve, x, y) {
                    self.vel_ignore_up = true;
                    let mut curve = curve;
                    if curve.delete(i) {
                        self.vel_selected_node = None;
                        self.commit_vel_curve(curve);
                    }
                    consume_redraw(cx, window_event, meta);
                    return true;
                }
                if let Some((i, is_out)) = self.hit_vel_handle(&curve, x, y) {
                    self.drag = Some(if is_out {
                        DragTarget::VelMapHandleOut
                    } else {
                        DragTarget::VelMapHandleIn
                    });
                    self.vel_selected_node = Some(i);
                    self.vel_just_inserted = false;
                    cx.capture();
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    return true;
                }
                if let Some(i) = self.hit_vel_node(&curve, x, y) {
                    self.drag = Some(DragTarget::VelMapNode);
                    self.vel_selected_node = Some(i);
                    self.vel_just_inserted = false;
                    cx.capture();
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                    return true;
                }
            }
            self.commit_note_edit();
            nih_plug_vizia::consume_window_event(cx, window_event, meta);
            true
        } else {
            self.commit_note_edit();
            self.vel_map_open = None;
            self.vel_selected_node = None;
            consume_redraw(cx, window_event, meta);
            true
        }
    }

    pub(super) fn handle_vel_map_mouse_up(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        mouse_x: f32,
        mouse_y: f32,
        was_vel_drag: bool,
    ) {
        if self.vel_ignore_up {
            self.vel_ignore_up = false;
        } else if !was_vel_drag && self.vel_map_open.is_some() {
            let (gx, gy, gw, gh) = Self::vel_map_graph();
            if Self::hit(mouse_x, mouse_y, gx, gy, gw, gh) {
                if let Some(mut curve) = self.vel_curve() {
                    if self.hit_vel_node(&curve, mouse_x, mouse_y).is_none() {
                        let (nx, _) = Self::graph_to_norm(mouse_x, mouse_y);
                        if let Some(idx) = curve.insert_at(nx) {
                            self.vel_selected_node = Some(idx);
                            self.vel_just_inserted = true;
                            self.commit_vel_curve(curve);
                            consume_redraw(cx, window_event, meta);
                        }
                    }
                }
            }
        }
    }

    pub(super) fn handle_vel_map_double_click(
        &mut self,
        cx: &mut EventContext,
        window_event: &WindowEvent,
        meta: &mut EventMeta,
        mouse_x: f32,
        mouse_y: f32,
    ) {
        if self.vel_map_open.is_none() {
            return;
        }
        let (modal_x, modal_y, modal_w, modal_h) = Self::vel_map_modal();
        let (gx, gy, gw, gh) = Self::vel_map_graph();
        if self.vel_art_menu_open || !Self::hit(mouse_x, mouse_y, gx, gy, gw, gh) {
            nih_plug_vizia::consume_window_event(cx, window_event, meta);
            return;
        }
        if Self::hit(mouse_x, mouse_y, modal_x, modal_y, modal_w, modal_h) {
            if let Some(kit_piece) = self.vel_map_open {
                if let Some(mut curve) = self.vel_curve() {
                    if let Some(i) = self.hit_vel_node(&curve, mouse_x, mouse_y) {
                        if self.vel_just_inserted || !curve.delete(i) {
                            self.reset_vel_art(kit_piece);
                            self.vel_selected_node = None;
                        } else {
                            self.vel_selected_node = None;
                            self.commit_vel_curve(curve);
                        }
                    } else {
                        self.reset_vel_art(kit_piece);
                        self.vel_selected_node = None;
                    }
                }
            }
            self.vel_just_inserted = false;
            self.vel_ignore_up = true;
            self.drag = None;
            consume_redraw(cx, window_event, meta);
        }
    }
}
