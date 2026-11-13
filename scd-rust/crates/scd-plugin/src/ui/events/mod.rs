//! Input routing for the SCD editor.
use super::*;
use nih_plug_vizia::vizia::events::EventMeta;

mod mixer;
mod modals;
mod presets;
mod velocity_map;

fn consume_redraw(cx: &mut EventContext, window_event: &WindowEvent, meta: &mut EventMeta) {
    nih_plug_vizia::consume_window_event(cx, window_event, meta);
    cx.needs_redraw();
}

impl ScdEditorView {
    pub(super) fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|param_event: &RawParamEvent, _| {
            if matches!(param_event, RawParamEvent::ParametersChanged) {
                cx.needs_redraw();
            }
        });
        event.map(|_: &NoteLearnPoll, _| {
            let now = Instant::now();
            self.vel_hits
                .retain(|(_, _, _, at)| now.duration_since(*at).as_secs_f32() < 2.5);
            for piece in KitPieceId::ALL {
                for art in 0..crate::vel_map::MAX_ARTS {
                    let velocity =
                        self.params.midi_velocities[piece as usize][art].swap(0, Ordering::Relaxed);
                    if velocity != 0
                        && self.vel_map_open == Some(piece)
                        && Self::vel_art_members(piece, self.vel_art).contains(&art)
                    {
                        // One trail per velocity keeps memory bounded even under dense MIDI.
                        self.vel_hits
                            .retain(|(p, a, v, _)| (*p, *a, *v) != (piece, art, velocity));
                        self.vel_hits.push((piece, art, velocity, now));
                    }
                }
            }
            if self.apply_note_learn() {
                cx.needs_redraw();
            }
        });
        event.map(|window_event, meta| {
            let bounds = cx.bounds();
            if bounds.w <= 0.0 || bounds.h <= 0.0 {
                return;
            }
            let scale = artwork_scale(bounds.w);
            let mouse_x = (cx.mouse().cursorx - bounds.x) / scale;
            let mouse_y = (cx.mouse().cursory - bounds.y) / scale;
            let prev_y = self.mouse.1;
            self.mouse = (mouse_x, mouse_y);

            if self.handle_note_edit_event(cx, window_event, meta, mouse_x, mouse_y) {
                return;
            }

            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    let x = mouse_x;
                    let y = mouse_y;

                    if self.handle_add_preset_mouse_down(cx, window_event, meta, x, y)
                        || self.handle_sub_kick_mouse_down(cx, window_event, meta, x, y)
                        || self.handle_vel_map_mouse_down(cx, window_event, meta, x, y)
                        || self.handle_cc_menu_mouse_down(cx, window_event, meta, x, y)
                        || self.handle_preset_menu_mouse_down(cx, window_event, meta, x, y)
                        || self.handle_samples_menu_mouse_down(cx, window_event, meta, x, y)
                        || self.handle_header_dropdown_mouse_down(cx, window_event, meta, x, y)
                    {
                        return;
                    }

                    self.handle_mixer_mouse_down(cx, window_event, meta, x, y);
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    self.handle_mouse_up(cx, window_event, meta, mouse_x, mouse_y);
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if self.reset_control_at(cx, mouse_x, mouse_y) {
                        consume_redraw(cx, window_event, meta);
                        return;
                    }
                    self.handle_vel_map_double_click(cx, window_event, meta, mouse_x, mouse_y);
                }
                WindowEvent::MouseMove(..) => {
                    self.handle_drag_move(cx, window_event, meta, mouse_x, mouse_y, prev_y);
                }
                WindowEvent::MouseOut => {
                    if self.hover_sof.is_some()
                        || self.hover_lock
                        || self.hover_invert
                        || self.hover_note.is_some()
                    {
                        self.hover_sof = None;
                        self.hover_lock = false;
                        self.hover_invert = false;
                        self.hover_note = None;
                        cx.needs_redraw();
                    }
                }
                WindowEvent::MouseScroll(..) if self.modal_open() => {
                    nih_plug_vizia::consume_window_event(cx, window_event, meta);
                }
                WindowEvent::MouseScroll(_, dy)
                    if *dy != 0.0
                        && (Self::hit(
                            mouse_x,
                            mouse_y,
                            CC_SELECT.0,
                            CC_SELECT.1,
                            CC_SELECT.2,
                            CC_SELECT.3,
                        ) || self.cc_menu_open) =>
                {
                    let delta = if *dy > 0.0 { 1 } else { -1 };
                    self.set_cc_number(cx, self.params.cc_number.value() + delta);
                    consume_redraw(cx, window_event, meta);
                }
                WindowEvent::CharInput(_) | WindowEvent::KeyDown(_, _) => {
                    self.handle_add_preset_key_event(cx, window_event, meta);
                }
                _ => {}
            }
        });
    }
}
