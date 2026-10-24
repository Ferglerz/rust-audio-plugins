use super::*;
use nih_plug_vizia::vizia::vg::{Paint, Path};

const ROW_H: f32 = 40.0;
const ROW_TOP: f32 = 116.0;
const FADE_H: f32 = 22.0;
pub(super) fn toggle_rect() -> Rect {
    (
        ARP_SURFACE.0 + ARP_SURFACE.2 - 104.0,
        ARP_SURFACE.1 + 10.0,
        92.0,
        31.0,
    )
}
pub(super) fn row_rect(index: usize, scroll: f32) -> Rect {
    let r = PERF_SURFACE;
    (
        r.0 + 12.0,
        r.1 + ROW_TOP + index as f32 * ROW_H - scroll,
        r.2 - 24.0,
        ROW_H - 4.0,
    )
}
fn rows_viewport(r: Rect) -> Rect {
    (
        r.0 + 12.0,
        r.1 + ROW_TOP,
        r.2 - 24.0,
        (r.3 - ROW_TOP - 28.0).max(0.0),
    )
}
fn max_scroll(r: Rect, count: usize) -> f32 {
    (count as f32 * ROW_H - 4.0 - rows_viewport(r).3).max(0.0)
}
fn edge_fade(d: &mut Draw, viewport: Rect, bottom: bool, strength: f32) {
    if strength <= 0.0 {
        return;
    }
    let x = d.ox + (viewport.0 + d.offset_x) * d.s;
    let y = d.oy + (viewport.1 + if bottom { viewport.3 - FADE_H } else { 0.0 }) * d.s;
    let h = FADE_H * d.s;
    let mut path = Path::new();
    path.rect(x, y, viewport.2 * d.s, h);
    let opaque = d.color(alpha(PANEL, strength));
    let clear = d.color(alpha(PANEL, 0.0));
    let paint = Paint::linear_gradient(
        x,
        y,
        x,
        y + h,
        if bottom { clear } else { opaque },
        if bottom { opaque } else { clear },
    );
    d.c.fill_path(&path, &paint);
}
pub(super) fn action_rect(index: usize) -> Rect {
    let r = PERF_SURFACE;
    let widths = [100.0, 90.0, 90.0, 90.0, 92.0];
    (
        r.0 + 16.0 + widths[..index].iter().sum::<f32>() + index as f32 * 8.0,
        r.1 + 48.0,
        widths[index],
        28.0,
    )
}
fn duration_button(r: Rect, plus: bool) -> Rect {
    (
        r.0 + r.2 - if plus { 40.0 } else { 120.0 },
        r.1 + 4.0,
        28.0,
        28.0,
    )
}
impl ChordboardView {
    pub(super) fn close_tracker(&mut self, cx: &mut EventContext) {
        if !self.tracker_open {
            return;
        }
        self.tracker_start = self.tracker_progress;
        self.tracker_elapsed = 0.0;
        self.tracker_open = false;
        self.tracker_selection = false;
        self.tracker_symbol_edit = false;
        self.tracker_symbol.clear();
        if self.tracker_drag.take().is_some() {
            cx.release();
        }
        cx.needs_redraw();
    }

    pub(super) fn scroll_tracker(&mut self, x: f32, y: f32, dy: f32, precise: bool) -> bool {
        let r = perf_surface_rect(self.expand_t());
        let panel_x = x - (1.0 - self.tracker_progress) * r.2;
        if !self.tracker_open || !hit(r, panel_x, y) || self.tracker_drag.is_some() {
            return false;
        }
        let limit = max_scroll(r, self.params.arpeggiator.snapshot().rows.len());
        self.tracker_scroll_target = (self.tracker_scroll_target - dy).clamp(0.0, limit);
        if precise {
            // AppKit already supplies the gesture's momentum; follow it directly.
            self.tracker_scroll = self.tracker_scroll_target;
        }
        true
    }

    pub(super) fn tick_tracker_scroll(&mut self, dt: f32) -> bool {
        let r = perf_surface_rect(self.expand_t());
        let limit = max_scroll(r, self.params.arpeggiator.snapshot().rows.len());
        let previous = self.tracker_scroll;
        self.tracker_scroll_target = self.tracker_scroll_target.clamp(0.0, limit);
        self.tracker_scroll = self.tracker_scroll.clamp(0.0, limit);
        let remaining = self.tracker_scroll_target - self.tracker_scroll;
        self.tracker_scroll = if remaining.abs() < 0.05 {
            self.tracker_scroll_target
        } else {
            self.tracker_scroll + remaining * (1.0 - (-dt / 0.055).exp())
        };
        self.tracker_scroll != previous
    }

    pub(super) fn draw_tracker(&self, d: &mut Draw) {
        let saved_disabled = d.disabled;
        d.disabled = !self.params.arp_enabled.value();
        if self.tracker_progress > 0.001 {
            let r = perf_surface_rect(self.expand_t());
            let saved_offset = d.offset_x;
            d.offset_x += (1.0 - self.tracker_progress) * r.2;
            d.scissor(r.0, r.1, r.2, r.3);
            d.rounded_rect(r.0, r.1, r.2, r.3, 8.0, PANEL);
            d.text(
                r.0 + 18.0,
                module_title_y(r.1, MODULE_TITLE_SIZE),
                "COMPOSITION TRACKER",
                MODULE_TITLE_SIZE,
                TEXT,
            );
            let rows = self.params.arpeggiator.snapshot().rows;
            d.text_middle(
                r.0 + r.2 - 90.0,
                r.1 + 22.0,
                &format!("{} rows", rows.len()),
                13.0,
                MUTED,
            );
            for (i, label) in ["+ Add row", "Copy", "Paste", "Remove"].iter().enumerate() {
                self.arp_button(d, action_rect(i), label, false, TEAL);
            }
            if self.tracker_selection || self.tracker_symbol_edit {
                let done = action_rect(4);
                let middle = done.1 + done.3 * 0.5;
                self.button_surface(d, done, false, TEAL);
                d.line(
                    done.0 + 14.0,
                    middle,
                    done.0 + 18.0,
                    middle + 4.0,
                    TEAL,
                    1.8,
                );
                d.line(
                    done.0 + 18.0,
                    middle + 4.0,
                    done.0 + 25.0,
                    middle - 5.0,
                    TEAL,
                    1.8,
                );
                d.text_middle(done.0 + 34.0, middle, "Done", 13.0, TEXT);
            }
            d.text(r.0 + 62.0, r.1 + 103.0, "CHORD", 12.0, MUTED);
            d.text_centered(r.0 + r.2 - 80.0, r.1 + 103.0, "LOOPS", 12.0, MUTED);
            let viewport = rows_viewport(r);
            let scroll = self.tracker_scroll.min(max_scroll(r, rows.len()));
            let first = (scroll / ROW_H).floor() as usize;
            let visible = (viewport.3 / ROW_H).ceil() as usize + 1;
            // Intersect with the panel clip while it slides in.
            let clip_x = (viewport.0 + d.offset_x).max(r.0);
            let clip_right = (viewport.0 + d.offset_x + viewport.2).min(r.0 + r.2);
            d.scissor(
                clip_x,
                viewport.1,
                (clip_right - clip_x).max(0.0),
                viewport.3,
            );
            for (i, row) in rows.iter().enumerate().skip(first).take(visible) {
                let rr = row_rect(i, scroll);
                let selected = i == self.arp_ui.edit_page;
                let dragging = self.tracker_drag.is_some_and(|(from, _)| from == i);
                d.rounded_rect(
                    rr.0,
                    rr.1,
                    rr.2,
                    rr.3,
                    5.0,
                    alpha(
                        if selected { TEAL } else { MUTED },
                        if selected { 0.14 } else { 0.04 },
                    ),
                );
                for dx in [0.0, 5.0] {
                    for dy in [-5.0, 0.0, 5.0] {
                        d.circle(
                            rr.0 + 12.0 + dx,
                            rr.1 + rr.3 * 0.5 + dy,
                            1.25,
                            if dragging { TEAL } else { MUTED },
                            true,
                        );
                    }
                }
                d.text_middle(
                    rr.0 + 31.0,
                    rr.1 + rr.3 * 0.5,
                    &(i + 1).to_string(),
                    12.0,
                    MUTED,
                );
                let chord = if self.tracker_symbol_edit && selected {
                    format!("{}▏", self.tracker_symbol)
                } else {
                    row.chord
                        .map(harmony::chord_name)
                        .unwrap_or_else(|| "Choose chord…".into())
                };
                let chord = self.fit_text(d, &chord, rr.2 - 198.0, 14.0);
                d.text_middle(
                    rr.0 + 50.0,
                    rr.1 + rr.3 * 0.5,
                    &chord,
                    14.0,
                    if selected && self.tracker_selection {
                        TEAL
                    } else if row.chord.is_some() {
                        TEXT
                    } else {
                        MUTED
                    },
                );
                for (plus, label) in [(false, "−"), (true, "+")] {
                    self.arp_button(d, duration_button(rr, plus), label, false, TEAL);
                }
                d.text_centered(
                    rr.0 + rr.2 - 66.0,
                    rr.1 + rr.3 * 0.5 + 4.5,
                    &row.duration.to_string(),
                    13.0,
                    TEXT,
                );
                if self.snapshot.arp_running && self.snapshot.arp_row_index as usize == i {
                    d.circle(rr.0 + rr.2 - 6.0, rr.1 + rr.3 * 0.5, 3.0, GOLD, true);
                }
            }
            if let Some((from, target)) = self.tracker_drag {
                let rr = row_rect(target, scroll);
                let y = if target > from {
                    rr.1 + rr.3 + 2.0
                } else {
                    rr.1 - 2.0
                };
                d.line(rr.0, y, rr.0 + rr.2, y, TEAL, 2.0);
            }
            edge_fade(d, viewport, false, (scroll / FADE_H).min(1.0));
            edge_fade(
                d,
                viewport,
                true,
                ((max_scroll(r, rows.len()) - scroll) / FADE_H).min(1.0),
            );
            d.scissor(r.0, r.1, r.2, r.3);
            if self.tracker_selection {
                d.text(
                    r.0 + 18.0,
                    r.1 + r.3 - 10.0,
                    "Choose a chord above · Double-click to type",
                    12.0,
                    TEAL,
                );
            }
            d.reset_scissor();
            d.offset_x = saved_offset;
        }
        self.arp_button(d, toggle_rect(), "Tracker", self.tracker_open, TEAL);
        d.disabled = saved_disabled;
    }

    fn finish_tracker_symbol(&mut self) {
        if let Some(chord) = harmony::parse_chord_symbol(&self.tracker_symbol) {
            let index = self.arp_ui.edit_page;
            self.params.arpeggiator.edit(|s| {
                if let Some(row) = s.rows.get_mut(index) {
                    row.chord = Some(SavedChord {
                        root: 60 + chord.root,
                        ..chord
                    });
                }
            });
        }
        self.tracker_symbol_edit = false;
        self.tracker_symbol.clear();
    }

    pub(super) fn tracker_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        if matches!(event, WindowEvent::MouseScroll(_, _))
            && std::mem::take(&mut self.tracker_skip_line_scroll)
        {
            return true;
        }
        if let Some((from, target)) = self.tracker_drag {
            match event {
                WindowEvent::MouseMove(_, _) => {
                    let count = self.params.arpeggiator.snapshot().rows.len();
                    let r = perf_surface_rect(self.expand_t());
                    let viewport = rows_viewport(r);
                    if y < viewport.1 {
                        self.tracker_scroll -= 8.0;
                    } else if y > viewport.1 + viewport.3 {
                        self.tracker_scroll += 8.0;
                    }
                    self.tracker_scroll = self.tracker_scroll.clamp(0.0, max_scroll(r, count));
                    self.tracker_scroll_target = self.tracker_scroll;
                    let target = ((self.tracker_scroll + y - viewport.1).max(0.0) / ROW_H) as usize;
                    let target = target.min(count - 1);
                    self.tracker_drag = Some((from, target));
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    let selected = self.arp_ui.edit_page;
                    self.params.arpeggiator.edit(|s| {
                        s.move_row(from, target as isize - from as isize);
                    });
                    self.arp_ui.edit_page = if selected == from {
                        target
                    } else if from < selected && selected <= target {
                        selected - 1
                    } else if target <= selected && selected < from {
                        selected + 1
                    } else {
                        selected
                    };
                    self.tracker_proxy_row = usize::MAX;
                    self.tracker_drag = None;
                    cx.release();
                }
                WindowEvent::KeyDown(Code::Escape, _) => {
                    self.tracker_drag = None;
                    cx.release();
                }
                _ => {}
            }
            cx.needs_redraw();
            return true;
        }
        let press = matches!(
            event,
            WindowEvent::MouseDown(MouseButton::Left)
                | WindowEvent::MouseDoubleClick(MouseButton::Left)
                | WindowEvent::MouseTripleClick(MouseButton::Left)
        );
        let r = perf_surface_rect(self.expand_t());
        let panel_x = x - (1.0 - self.tracker_progress) * r.2;
        if press
            && self.tracker_open
            && (self.tracker_selection || self.tracker_symbol_edit)
            && hit(action_rect(4), panel_x, y)
        {
            if self.tracker_symbol_edit {
                self.finish_tracker_symbol();
            }
            self.tracker_selection = false;
            cx.needs_redraw();
            return true;
        }
        if self.tracker_symbol_edit {
            match event {
                WindowEvent::CharInput(ch) if !ch.is_control() => self.tracker_symbol.push(*ch),
                WindowEvent::KeyDown(Code::Backspace, _) => {
                    self.tracker_symbol.pop();
                }
                WindowEvent::KeyDown(Code::Escape, _) => {
                    self.tracker_symbol_edit = false;
                    self.tracker_symbol.clear();
                }
                WindowEvent::KeyDown(Code::Enter | Code::NumpadEnter, _) => {
                    self.finish_tracker_symbol()
                }
                _ if press => self.finish_tracker_symbol(),
                WindowEvent::KeyDown(_, _) | WindowEvent::KeyUp(_, _) => return true,
                _ => return false,
            }
            cx.needs_redraw();
            if !press {
                return true;
            }
        }
        if matches!(event, WindowEvent::KeyDown(Code::Escape, _)) && self.tracker_selection {
            self.tracker_selection = false;
            cx.needs_redraw();
            return true;
        }
        if let WindowEvent::MouseScroll(_, dy) = event {
            if self.scroll_tracker(x, y, *dy * ROW_H, false) {
                cx.needs_redraw();
                return true;
            }
        }
        if !press {
            return false;
        }
        if hit(toggle_rect(), x, y) {
            if !self.tracker_open {
                if !self.params.arp_enabled.value() {
                    return true;
                }
                self.tracker_restore_expand = self.expand_target >= 0.5;
                self.tracker_start = self.tracker_progress;
                self.tracker_elapsed = 0.0;
                self.tracker_open = true;
                if self.expand_target >= 0.5 {
                    self.toggle_expand();
                }
            } else {
                self.close_tracker(cx);
            }
            cx.needs_redraw();
            return true;
        }
        if !self.tracker_open || !hit(r, panel_x, y) {
            return false;
        }
        cx.focus();
        self.focused = true;
        let rows = self.params.arpeggiator.snapshot().rows;
        if let Some(action) = (0..4).find(|&i| hit(action_rect(i), panel_x, y)) {
            let selected = self.arp_ui.edit_page.min(rows.len() - 1);
            match action {
                0 => self
                    .params
                    .arpeggiator
                    .edit(|s| self.arp_ui.edit_page = s.add_row_after(selected)),
                1 => self.arp_ui.tracker_clipboard = rows.get(selected).cloned(),
                2 => {
                    if let Some(row) = self.arp_ui.tracker_clipboard.clone() {
                        self.params
                            .arpeggiator
                            .edit(|s| s.paste_row(selected, &row));
                    }
                }
                3 => self
                    .params
                    .arpeggiator
                    .edit(|s| self.arp_ui.edit_page = s.delete_row(selected)),
                _ => {}
            }
            let count = self.params.arpeggiator.snapshot().rows.len();
            let top = self.arp_ui.edit_page as f32 * ROW_H;
            let bottom = top + ROW_H - 4.0;
            self.tracker_scroll_target = self
                .tracker_scroll_target
                .max(bottom - rows_viewport(r).3)
                .min(top)
                .clamp(0.0, max_scroll(r, count));
            self.tracker_proxy_row = usize::MAX;
            cx.needs_redraw();
            return true;
        }
        if let Some(index) = (0..rows.len()).find(|&i| {
            hit(rows_viewport(r), panel_x, y) && hit(row_rect(i, self.tracker_scroll), panel_x, y)
        }) {
            let rr = row_rect(index, self.tracker_scroll);
            if panel_x < rr.0 + 26.0 {
                self.tracker_drag = Some((index, index));
                cx.capture();
            } else if let Some(plus) = [false, true]
                .into_iter()
                .find(|&plus| hit(duration_button(rr, plus), panel_x, y))
            {
                self.params.arpeggiator.edit(|s| {
                    let duration = &mut s.rows[index].duration;
                    *duration = if plus {
                        duration.saturating_add(1)
                    } else {
                        duration.saturating_sub(1).max(1)
                    };
                });
            } else {
                self.arp_ui.edit_page = index;
                self.tracker_selection = true;
                if matches!(event, WindowEvent::MouseDoubleClick(MouseButton::Left)) {
                    self.tracker_selection = false;
                    self.tracker_symbol = rows[index]
                        .chord
                        .map(harmony::chord_name)
                        .unwrap_or_default();
                    self.tracker_symbol_edit = true;
                }
            }
            cx.needs_redraw();
        }
        true
    }
    pub(super) fn assign_tracker_chord(&mut self, chord: harmony::KeyboardChord) {
        if !self.tracker_selection {
            return;
        }
        self.params.arpeggiator.edit(|s| {
            if let Some(row) = s.rows.get_mut(self.arp_ui.edit_page) {
                row.chord = Some(SavedChord {
                    root: (self.params.keyboard_octave.value() + chord.root as i32).clamp(0, 127)
                        as u8,
                    quality: chord.quality,
                    second: None,
                    additions: 0,
                    inversion: 0,
                    spread: 0,
                    transpose: 0,
                });
            }
        });
    }
}
