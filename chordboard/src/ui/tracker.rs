//! Chord roll: sequence order and loop duration on X, chromatic root on Y.
use super::*;
use crate::arpeggiator::{State, TrackerRow};

const INSERT_BAND_H: f32 = 14.0;
const GRID_TOP: f32 = MODULE_HEADER_H + 2.0 + INSERT_BAND_H;
pub(super) const LANE_H: f32 = (TRACKER_SURFACE.3 - GRID_TOP - 24.0) / 12.0;
pub(super) const LOOP_W: f32 = 96.0;
// Fit roughly two dozen readable entries, retaining musical duration ratios.
pub(super) fn loop_width(rows: &[TrackerRow]) -> f32 {
    let total = rows.iter().map(|r| r.duration as f64).sum::<f64>() + 1.0;
    let shortest = rows.iter().map(|r| r.duration).min().unwrap_or(1).max(1) as f32;
    (grid_rect().2 as f64 / total).min(LOOP_W as f64).max((64.0 / shortest) as f64) as f32
}
const DRAG_THRESHOLD: f32 = 4.0;

pub(super) fn grid_rect() -> Rect {
    let r = TRACKER_SURFACE;
    (r.0 + 52.0, r.1 + GRID_TOP, r.2 - 68.0, 12.0 * LANE_H)
}
pub(super) fn column_header_rect() -> Rect {
    let grid = grid_rect();
    (grid.0, grid.1 - INSERT_BAND_H, grid.2, INSERT_BAND_H)
}
fn row_start_from(rows: &[TrackerRow], index: usize, origin: usize) -> f32 {
    let count = if index == rows.len() { rows.len() } else { (index + rows.len() - origin) % rows.len() };
    (0..count).map(|i| rows[(origin + i) % rows.len()].duration as f64).sum::<f64>() as f32 * loop_width(rows)
}
fn row_start(rows: &[TrackerRow], index: usize) -> f32 { row_start_from(rows, index, 0) }
fn pitch(chord: SavedChord) -> i16 {
    (chord.root as i16 + chord.transpose as i16).rem_euclid(12)
}
#[cfg(test)]
pub(super) fn row_rect(rows: &[TrackerRow], index: usize, scroll: f32) -> Rect {
    block_rect(loop_width(rows), &rows[index], row_start(rows, index), scroll)
}
fn block_rect(width: f32, row: &TrackerRow, start: f32, scroll: f32) -> Rect {
    let grid = grid_rect();
    (
        grid.0 + start - scroll + 2.0,
        grid.1 + (11 - row.chord.map_or(0, pitch)) as f32 * LANE_H + 2.0,
        row.duration as f32 * width - 4.0,
        LANE_H - 4.0,
    )
}
fn blocks(rows: &[TrackerRow], scroll: f32, origin: usize) -> impl Iterator<Item = (usize, Rect)> + '_ {
    let width = loop_width(rows);
    (0..rows.len()).map(move |i| { let index = (origin + i) % rows.len(); (index, &rows[index]) })
        .scan(0.0_f64, move |start, (i, row)| {
            let rect = block_rect(width, row, (*start * width as f64) as f32, scroll);
            *start += row.duration as f64;
            Some((i, rect))
        })
}

pub(super) fn blank_column(rows: &[TrackerRow], scroll: f32) -> Rect {
    let grid = grid_rect();
    (
        grid.0 + row_start(rows, rows.len()) - scroll,
        grid.1,
        loop_width(rows),
        grid.3,
    )
}

fn insert_rect_from(rows: &[TrackerRow], index: usize, scroll: f32, origin: usize) -> Rect {
    let grid = grid_rect();
    (
        grid.0 + row_start_from(rows, index, origin) - scroll - 14.0,
        grid.1 - INSERT_BAND_H,
        28.0,
        INSERT_BAND_H - 1.0,
    )
}

#[cfg(test)]
pub(super) fn insert_rect(rows: &[TrackerRow], index: usize, scroll: f32) -> Rect { insert_rect_from(rows, index, scroll, 0) }
fn insert_at(rows: &[TrackerRow], scroll: f32, x: f32, y: f32, origin: usize) -> Option<usize> {
    let grid = grid_rect();
    if x < grid.0 || x >= grid.0 + grid.2 {
        return None;
    }
    (1..rows.len()).map(|i| (origin + i) % rows.len()).find(|&index| hit(insert_rect_from(rows, index, scroll, origin), x, y))
}

fn cell_pitch(y: f32) -> u8 {
    11 - ((y - grid_rect().1) / LANE_H).floor().clamp(0.0, 11.0) as u8
}

fn max_scroll(rows: &[TrackerRow]) -> f32 {
    (row_start(rows, rows.len()) + loop_width(rows) - grid_rect().2).max(0.0)
}
pub(super) const ACTIONS: [&str; 4] = ["Duplicate", "Copy", "Paste", "Delete"];
pub(super) fn action_rect(index: usize) -> Rect {
    let r = TRACKER_SURFACE;
    (
        r.0 + 140.0 + index as f32 * 100.0,
        r.1 + 8.0,
        92.0,
        28.0,
    )
}
pub(super) fn transport_rect() -> Rect {
    let r = TRACKER_SURFACE;
    (
        r.0 + 916.0,
        r.1 + 8.0,
        28.0,
        28.0,
    )
}

pub(super) fn duration_button(plus: bool) -> Rect {
    let r = TRACKER_SURFACE;
    (
        r.0 + if plus { 864.0 } else { 704.0 },
        r.1 + 8.0,
        28.0,
        28.0,
    )
}
pub(super) fn scrollbar_rect() -> Rect {
    let grid = grid_rect();
    (grid.0, grid.1 + grid.3 + 8.0, grid.2, 8.0)
}
fn scrollbar_thumb(rows: &[TrackerRow], scroll: f32) -> Rect {
    let r = scrollbar_rect();
    let width = (r.2 * r.2 / (row_start(rows, rows.len()) + loop_width(rows)).max(r.2)).clamp(24.0, r.2);
    let limit = max_scroll(rows);
    (
        r.0 + if limit > 0.0 {
            scroll / limit * (r.2 - width)
        } else {
            0.0
        },
        r.1,
        width,
        r.3,
    )
}
pub(super) fn scroll_delta(x: f32, y: f32) -> f32 {
    if x.abs() > y.abs() {
        x
    } else {
        y
    }
}

// Move both absolute inputs together so interval-derived chords retain their recipe.
fn transpose_chord(mut chord: SavedChord, requested: i16) -> SavedChord {
    if requested == 0 {
        return chord;
    }
    let root = chord.root as i16;
    let second = chord.second.map_or(root, i16::from);
    let sounding = root + chord.transpose as i16;
    let low = (-root).max(-second).max(-sounding);
    let high = (127 - root).min(127 - second).min(127 - sounding);
    if low <= high {
        let delta = requested.clamp(low, high);
        chord.root = (root + delta) as u8;
        chord.second = chord.second.map(|n| (n as i16 + delta) as u8);
    }
    chord
}

#[derive(Clone, Copy)]
pub(super) struct BlockDrag {
    from: usize,
    view_origin: usize,
    id: u64,
    original_chord: Option<SavedChord>,
    original_duration: u32,
    press: (f32, f32),
    pointer: (f32, f32),
    start_scroll: f32,
    resize: bool,
    active: bool,
    target: usize,
    chord: Option<SavedChord>,
    duration: u32,
}
#[derive(Clone, Copy)]
pub(super) enum TrackerDrag {
    Block(BlockDrag),
    Scroll { press: f32, start: f32 },
}
impl TrackerDrag {
    pub(super) fn block(
        state: &State,
        from: usize,
        press: (f32, f32),
        scroll: f32,
        resize: bool,
    ) -> Self {
        let row = &state.rows[from];
        Self::Block(BlockDrag {
            from,
            view_origin: state.view_row_start(),
            id: row.id,
            original_chord: row.chord,
            original_duration: row.duration,
            press,
            pointer: press,
            start_scroll: scroll,
            resize,
            active: false,
            target: from,
            chord: row.chord,
            duration: row.duration,
        })
    }
    pub(super) fn valid(self, state: &State) -> bool {
        match self {
            Self::Block(d) => state.rows.get(d.from).is_some_and(|r| {
                r.id == d.id && r.chord == d.original_chord && r.duration == d.original_duration && state.view_row_start() == d.view_origin
            }),
            Self::Scroll { .. } => true,
        }
    }
}
impl BlockDrag {
    fn update(&mut self, rows: &[TrackerRow], scroll: f32, pointer: (f32, f32)) {
        self.pointer = pointer;
        let physical_dx = pointer.0 - self.press.0;
        let dy = pointer.1 - self.press.1;
        self.active |= physical_dx.abs().max(dy.abs()) >= DRAG_THRESHOLD;
        if !self.active {
            return;
        }
        let dx = physical_dx + scroll - self.start_scroll;
        if self.resize {
            self.duration = (self.original_duration as f64 + (dx / loop_width(rows)).round() as f64)
                .clamp(1.0, u32::MAX as f64) as u32;
        } else {
            self.chord = self.original_chord.map(|chord| {
                let destination =
                    (pitch(chord) as f32 - (dy / LANE_H).round()).clamp(0.0, 11.0) as i16;
                transpose_chord(chord, destination - pitch(chord))
            });
            // Compare the moving block's center with the other blocks' centers.
            // Keeping the pointer offset avoids an immediate jump on wide chords.
            let center =
                row_start_from(rows, self.from, self.view_origin) + self.original_duration as f32 * loop_width(rows) * 0.5 + dx;
            self.target = blocks(rows, 0.0, self.view_origin)
                .filter(|(i, r)| *i != self.from && center > r.0 - grid_rect().0 + r.2 * 0.5)
                .count();
        }
    }
}

impl ChordboardView {
    pub(super) fn toggle_tracker_transport(&mut self, cx: &mut EventContext) {
        if self.snapshot.arp_running {
            self.bridge.send(Command::TrackerPause);
        } else {
            let state = self.arp_state();
            let selected = self.arp_ui.edit_page.min(state.rows.len() - 1);
            Self::emit(cx, self.params.arp_enabled.as_ptr(), 1.0);
            self.bridge.send(Command::TrackerPlay(selected));
        }
        cx.needs_redraw();
    }

    pub(super) fn draw_tracker_transport(&self, d: &mut Draw, r: Rect) {
        let playing = self.snapshot.arp_running;
        self.button_surface(d, r, playing, TEAL);
        let (r, _) = self.button_content(d, r, "");
        let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
        let color = if playing { TEAL } else { TEXT };
        if playing {
            d.rect(x - 5.0, y - 6.0, 3.0, 12.0, color);
            d.rect(x + 2.0, y - 6.0, 3.0, 12.0, color);
        } else {
            d.poly(&[(x - 4.0, y - 6.0), (x + 6.0, y), (x - 4.0, y + 6.0), (x - 4.0, y - 6.0)], color, 1.8);
        }
    }

    pub(super) fn tracker_visible(&self) -> bool {
        self.programming
            && !self.fx_ui.open
            && !self.top_slide.active()
            && self.panel != Some(Panel::Custom)
            && !self.quick_midi.show
    }

    pub(super) fn scroll_tracker(&mut self, x: f32, y: f32, delta: f32, precise: bool) -> bool {
        if !self.tracker_visible()
            || !hit(TRACKER_SURFACE, x, y)
            || self.tracker_drag.is_some()
            || self.menu.is_some()
        {
            return false;
        }
        let state = self.arp_state();
        self.tracker_scroll_target =
            (self.tracker_scroll_target - delta).clamp(0.0, max_scroll(&state.rows));
        if precise {
            self.tracker_scroll = self.tracker_scroll_target;
        }
        true
    }

    pub(super) fn tick_tracker_scroll(&mut self, dt: f32) -> bool {
        let state = self.arp_state();
        let mut limit = max_scroll(&state.rows);
        let previous = self.tracker_scroll;
        if let Some(TrackerDrag::Block(mut drag)) = self.tracker_drag {
            if drag.active && TrackerDrag::Block(drag).valid(&state) {
                let grid = grid_rect();
                let speed = if drag.pointer.0 < grid.0 + 24.0 {
                    -480.0
                } else if drag.pointer.0 > grid.0 + grid.2 - 24.0 {
                    480.0
                } else {
                    0.0
                };
                // Resizing can extend beyond the current sequence's right edge.
                if drag.resize {
                    limit = limit.max(
                        row_start_from(&state.rows, drag.from, drag.view_origin) + drag.duration as f32 * loop_width(&state.rows) - grid.2,
                    );
                    if speed > 0.0 {
                        limit += speed * dt;
                    }
                }
                self.tracker_scroll_target = (self.tracker_scroll + speed * dt).clamp(0.0, limit);
                self.tracker_scroll = self.tracker_scroll_target;
                drag.update(&state.rows, self.tracker_scroll, drag.pointer);
                self.tracker_drag = Some(TrackerDrag::Block(drag));
            }
        }
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
        d.disabled = false;
        let r = TRACKER_SURFACE;
        let grid = grid_rect();
        self.surface(d, r);
        d.scissor(r.0, r.1, r.2, r.3);
        self.module_title(d, r.0 + 18.0, r.1, "TRACKER", TEAL);
        let state = self.arp_state();
        let rows = &state.rows;
        let origin = state.view_row_start();
        let selected = self.arp_ui.edit_page.min(rows.len() - 1);
        for (i, label) in ACTIONS.into_iter().enumerate() {
            self.arp_button(d, action_rect(i), label, false, TEAL);
        }
        let preview = match self.tracker_drag {
            Some(TrackerDrag::Block(drag))
                if drag.active && TrackerDrag::Block(drag).valid(&state) =>
            {
                Some(drag)
            }
            _ => None,
        };
        let duration = preview
            .filter(|drag| drag.from == selected)
            .map_or(rows[selected].duration, |drag| drag.duration);
        d.text_middle(r.0 + 652.0, r.1 + 22.0, "Bars", TEXT_LABEL, MUTED);
        for (plus, label) in [(false, "−"), (true, "+")] {
            self.arp_button(d, duration_button(plus), label, false, TEAL);
        }
        d.text_centered(r.0 + 798.0, r.1 + 27.0, &duration.to_string(), 13.0, TEXT);
        self.draw_tracker_transport(d, transport_rect());
        self.draw_tempo_header(d);
        d.line(
            r.0 + 18.0,
            r.1 + 44.0,
            r.0 + r.2 - 18.0,
            r.1 + 44.0,
            TEAL,
            2.0,
        );
        for pc in 0..12 {
            let y = grid.1 + (11 - pc) as f32 * LANE_H;
            let accidental = matches!(pc, 1 | 3 | 6 | 8 | 10);
            d.rounded_rect(
                grid.0,
                y,
                grid.2,
                LANE_H,
                0.0,
                alpha(MUTED, if accidental { 0.09 } else { 0.025 }),
            );
            d.line(grid.0, y, grid.0 + grid.2, y, LINE, 0.5);
            d.text_middle(
                r.0 + 14.0,
                y + LANE_H * 0.5,
                [
                    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                ][pc],
                TEXT_LABEL,
                MUTED,
            );
        }
        let scroll = self.tracker_scroll;
        // Only draw column lines within the viewport, even for very long sequences.
        d.scissor(
            grid.0,
            grid.1 - INSERT_BAND_H,
            grid.2,
            grid.3 + INSERT_BAND_H,
        );
        let width = loop_width(rows);
        let first = (scroll / width).floor() as u64;
        for n in first..=first + (grid.2 / width).ceil() as u64 + 1 {
            let x = grid.0 + n as f32 * width - scroll;
            d.line(x, grid.1, x, grid.1 + grid.3, LINE, 0.6);
        }
        let playing = self
            .snapshot
            .arp_running
            .then_some(self.snapshot.arp_row_index as usize);
        if let Some(index) = playing.filter(|&index| index < rows.len()) {
            let x = grid.0 + row_start_from(rows, index, origin) - scroll;
            let w = rows[index].duration as f32 * width;
            d.rect(x, grid.1, w, grid.3, alpha(GOLD, 0.24));
        }
        let selected_x = grid.0 + row_start_from(rows, selected, origin) - scroll;
        let selected_w = rows[selected].duration as f32 * width;
        if playing != Some(selected) {
            d.rect(selected_x, grid.1, selected_w, grid.3, alpha(TEAL, 0.12));
        }
        d.outline_rounded(
            selected_x + 1.0,
            grid.1,
            selected_w - 2.0,
            grid.3,
            2.0,
            TEAL,
            2.0,
        );
        if self.tracker_visible() && self.tracker_drag.is_none() {
            if let Some(index) = self
                .pointer
                .and_then(|(x, y)| insert_at(rows, scroll, x, y, origin))
            {
                let marker = insert_rect_from(rows, index, scroll, origin);
                let x = marker.0 + marker.2 * 0.5;
                d.fill_poly(
                    &[
                        (x - 8.0, marker.1 + 2.0),
                        (x + 8.0, marker.1 + 2.0),
                        (x, marker.1 + marker.3 - 2.0),
                    ],
                    TEAL,
                );
            }
        }
        d.scissor(grid.0, grid.1, grid.2, grid.3);
        let blank = blank_column(rows, scroll);
        d.rect(blank.0, blank.1, blank.2, blank.3, alpha(TEAL, 0.055));
        d.outline_rounded(
            blank.0 + 1.0,
            blank.1,
            blank.2 - 2.0,
            blank.3,
            2.0,
            alpha(TEAL, 0.4),
            1.0,
        );
        if self.tracker_visible() && self.tracker_drag.is_none() {
            if let Some((x, y)) = self.pointer.filter(|&(x, y)| hit(grid, x, y)) {
                let empty = if hit(blank, x, y) {
                    Some(blank)
                } else {
                    blocks(rows, scroll, origin)
                        .find(|(i, rr)| {
                            rows[*i].chord.is_none() && x >= rr.0 - 2.0 && x < rr.0 + rr.2 + 2.0
                        })
                        .map(|(_, rr)| (rr.0 - 2.0, grid.1, rr.2 + 4.0, grid.3))
                };
                if let Some(column) = empty {
                    let y = grid.1 + (11 - cell_pitch(y)) as f32 * LANE_H;
                    d.rounded_rect(
                        column.0 + 2.0,
                        y + 2.0,
                        column.2 - 4.0,
                        LANE_H - 4.0,
                        3.0,
                        alpha(TEAL, 0.4),
                    );
                    d.text_centered(
                        column.0 + column.2 * 0.5,
                        y + LANE_H * 0.65,
                        "+",
                        16.0,
                        TEAL,
                    );
                }
            }
        }
        // Paint the moving chord last so adjacent blocks cannot obscure the preview.
        let stationary =
            blocks(rows, scroll, origin).filter(|(i, _)| !preview.is_some_and(|drag| drag.from == *i));
        let moving = preview.map(|drag| {
            (
                drag.from,
                block_rect(loop_width(rows), &rows[drag.from], row_start_from(rows, drag.from, origin), scroll),
            )
        });
        for (i, mut rr) in stationary.chain(moving) {
            let mut chord = rows[i].chord;
            if let Some(drag) = preview.filter(|drag| drag.from == i) {
                if drag.resize {
                    rr.2 = drag.duration as f32 * loop_width(rows) - 4.0;
                } else {
                    rr.0 += drag.pointer.0 - drag.press.0 + scroll - drag.start_scroll;
                    chord = drag.chord;
                    rr.1 = grid.1 + (11 - chord.map_or(0, pitch)) as f32 * LANE_H + 2.0;
                }
            }
            if rr.0 + rr.2 <= grid.0 || rr.0 >= grid.0 + grid.2 {
                continue;
            }
            if chord.is_none() {
                continue;
            }
            let color = if i == selected { TEAL } else { TEXT };
            d.rounded_rect(
                rr.0,
                rr.1,
                rr.2,
                rr.3,
                3.0,
                alpha(color, if i == selected { 0.55 } else { 0.12 }),
            );
            d.outline_rounded(rr.0, rr.1, rr.2, rr.3, 3.0, alpha(color, 0.6), 1.0);
            let label = if self.tracker_symbol_edit && i == selected {
                format!("{}▏", self.tracker_symbol)
            } else { chord.map(harmony::chord_name).unwrap_or_else(|| "Unassigned".into()) };
            let left = rr.0.max(grid.0);
            let width = (rr.0 + rr.2).min(grid.0 + grid.2) - left;
            let label = self.fit_text(d, &label, (width - 18.0).max(0.0), TEXT_LABEL);
            if width > 20.0 {
                d.text_middle(left + 6.0, rr.1 + rr.3 * 0.5, &label, TEXT_LABEL, color);
            }
            d.line(
                rr.0 + rr.2 - 5.0,
                rr.1 + 5.0,
                rr.0 + rr.2 - 5.0,
                rr.1 + rr.3 - 5.0,
                color,
                1.0,
            );
            if playing == Some(i) {
                d.outline_rounded(rr.0, rr.1, rr.2, rr.3, 3.0, GOLD, 2.5);
            }
        }
        if let Some(drag) = preview.filter(|drag| !drag.resize && drag.target != (drag.from + rows.len() - origin) % rows.len()) {
            let boundary = if drag.target > (drag.from + rows.len() - origin) % rows.len() {
                drag.target + 1
            } else {
                drag.target
            };
            let index = if boundary == rows.len() { boundary } else { (origin + boundary) % rows.len() };
            let x = grid.0 + row_start_from(rows, index, origin) - scroll;
            d.line(x, grid.1, x, grid.1 + grid.3, TEAL, 2.5);
        }
        d.scissor(r.0, r.1, r.2, r.3);
        let track = scrollbar_rect();
        d.rounded_rect(track.0, track.1, track.2, track.3, 4.0, alpha(MUTED, 0.12));
        let thumb = scrollbar_thumb(rows, scroll.min(max_scroll(rows)));
        d.rounded_rect(thumb.0, thumb.1, thumb.2, thumb.3, 4.0, alpha(TEAL, 0.65));
        d.reset_scissor();
        d.disabled = saved_disabled;
    }

    fn tracker_drag_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(mut drag) = self.tracker_drag else {
            return false;
        };
        let state = self.arp_state();
        if !drag.valid(&state)
            || matches!(
                event,
                WindowEvent::KeyDown(Code::Escape, _) | WindowEvent::FocusOut
            )
        {
            self.tracker_drag = None;
            cx.release();
            cx.needs_redraw();
            return true;
        }
        match event {
            WindowEvent::MouseMove(_, _) | WindowEvent::MouseUp(MouseButton::Left) => {
                match &mut drag {
                    TrackerDrag::Block(drag) => {
                        drag.update(&state.rows, self.tracker_scroll, (x, y))
                    }
                    TrackerDrag::Scroll { press, start } => {
                        let travel = scrollbar_rect().2 - scrollbar_thumb(&state.rows, 0.0).2;
                        if travel > 0.0 {
                            self.tracker_scroll = (*start
                                + (x - *press) / travel * max_scroll(&state.rows))
                            .clamp(0.0, max_scroll(&state.rows));
                            self.tracker_scroll_target = self.tracker_scroll;
                        }
                    }
                }
                if matches!(event, WindowEvent::MouseUp(MouseButton::Left)) {
                    if let TrackerDrag::Block(drag) = drag {
                        if drag.active {
                            self.params.arpeggiator.edit(|s| {
                                // Snapshot validation above protects gestures across preset restores.
                                if let Some(row) = s.rows.get_mut(drag.from).filter(|r| {
                                    r.id == drag.id
                                        && r.chord == drag.original_chord
                                        && r.duration == drag.original_duration
                                }) {
                                    row.chord = drag.chord;
                                    row.duration = drag.duration;
                                    if drag.view_origin == 0 {
                                        self.arp_ui.edit_page = s.move_row(drag.from, drag.target as isize - drag.from as isize);
                                    } else {
                                    let first_id = s.rows[0].id;
                                    s.rows.rotate_left(drag.view_origin);
                                    let from = (drag.from + s.rows.len() - drag.view_origin) % s.rows.len();
                                    s.move_row(from, drag.target as isize - from as isize);
                                    let start = s.rows.iter().position(|r| r.id == first_id).unwrap_or(0);
                                    s.rows.rotate_left(start);
                                    self.arp_ui.edit_page = s.rows.iter().position(|r| r.id == drag.id).unwrap_or(0);
                                    }
                                }
                            });
                            self.tracker_proxy_row = usize::MAX;
                        }
                    }
                    self.tracker_drag = None;
                    cx.release();
                    if matches!(drag, TrackerDrag::Block(_)) {
                        self.reveal_tracker_selection();
                    }
                } else {
                    self.tracker_drag = Some(drag);
                }
                cx.needs_redraw();
            }
            _ => {}
        }
        true
    }

    pub(super) fn tracker_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
    ) -> bool {
        if !self.tracker_visible() {
            return false;
        }
        if self.tracker_drag_event(cx, event, x, y) {
            return true;
        }
        // Popovers are drawn over the roll, so they retain pointer/keyboard priority.
        if self.menu.is_some() {
            return false;
        }
        if matches!(event, WindowEvent::MouseDown(MouseButton::Right)) {
            let state = self.arp_state();
            let grid = grid_rect();
            if hit(column_header_rect(), x, y) || hit(grid, x, y) {
                if let Some((index, _)) = blocks(&state.rows, self.tracker_scroll, state.view_row_start())
                    .find(|(_, r)| x >= r.0 - 2.0 && x < r.0 + r.2 + 2.0) {
                    self.params.arpeggiator.set_view_row_origin(state.rows[index].id);
                    self.tracker_scroll = 0.0;
                    self.tracker_scroll_target = 0.0;
                    cx.needs_redraw();
                    return true;
                }
            }
        }
        let press = matches!(
            event,
            WindowEvent::MouseDown(MouseButton::Left)
                | WindowEvent::MouseDoubleClick(MouseButton::Left)
                | WindowEvent::MouseTripleClick(MouseButton::Left)
        );
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
        if y < TRACKER_SURFACE.1 + MODULE_HEADER_H && x >= METER_NUM.0 {
            return false;
        }
        if let WindowEvent::MouseScroll(dx, dy) = event {
            if std::mem::take(&mut self.tracker_skip_line_scroll)
                || self.scroll_tracker(x, y, scroll_delta(*dx, *dy) * 40.0, false)
            {
                cx.needs_redraw();
                return true;
            }
        }
        if !press || !hit(TRACKER_SURFACE, x, y) {
            return false;
        }
        cx.focus();
        self.focused = true;
        let state = self.arp_state();
        let rows = &state.rows;
        let origin = state.view_row_start();
        let selected = self.arp_ui.edit_page.min(rows.len() - 1);
        if hit(transport_rect(), x, y) {
            self.toggle_tracker_transport(cx);
        } else if let Some(index) = insert_at(rows, self.tracker_scroll, x, y, origin) {
            self.params.arpeggiator.edit(|s| {
                self.arp_ui.edit_page =
                    s.insert_blank_row_after((index + rows.len() - 1) % rows.len());
            });
            self.tracker_selection = true;
            self.reveal_tracker_selection();
        } else if let Some(action) = (0..ACTIONS.len()).find(|&i| hit(action_rect(i), x, y))
        {
            match action {
                0 => self
                    .params
                    .arpeggiator
                    .edit(|s| self.arp_ui.edit_page = s.add_row_after(selected)),
                1 => self.arp_ui.tracker_clipboard = Some(rows[selected].clone()),
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
            self.reveal_tracker_selection();
        } else if let Some(plus) = [false, true]
            .into_iter()
            .find(|&plus| hit(duration_button(plus), x, y))
        {
            self.params.arpeggiator.edit(|s| {
                let duration = &mut s.rows[selected].duration;
                *duration = if plus {
                    duration.saturating_add(1)
                } else {
                    duration.saturating_sub(1).max(1)
                };
            });
        } else if hit(scrollbar_rect(), x, y) {
            let thumb = scrollbar_thumb(rows, self.tracker_scroll);
            if !hit(thumb, x, y) {
                let travel = scrollbar_rect().2 - thumb.2;
                if travel > 0.0 {
                    self.tracker_scroll = ((x - scrollbar_rect().0 - thumb.2 * 0.5) / travel
                        * max_scroll(rows))
                    .clamp(0.0, max_scroll(rows));
                }
            }
            self.tracker_scroll_target = self.tracker_scroll;
            self.tracker_drag = Some(TrackerDrag::Scroll {
                press: x,
                start: self.tracker_scroll,
            });
            cx.capture();
        } else if y < grid_rect().1 && hit(column_header_rect(), x, y) {
            if let Some((index, _)) = blocks(rows, self.tracker_scroll, origin)
                .find(|(_, rr)| x >= rr.0 - 2.0 && x < rr.0 + rr.2 + 2.0)
            {
                self.arp_ui.edit_page = index;
                self.tracker_selection = true;
            }
        } else if hit(grid_rect(), x, y) {
            let append = hit(blank_column(rows, self.tracker_scroll), x, y);
            let empty = blocks(rows, self.tracker_scroll, origin)
                .find(|(i, rr)| {
                    rows[*i].chord.is_none()
                        && x >= rr.0 - 2.0
                        && x < rr.0 + rr.2 + 2.0
                        && matches!(event, WindowEvent::MouseDown(MouseButton::Left))
                })
                .map(|(i, _)| i);
            if append || empty.is_some() {
                let root = 60 + cell_pitch(y);
                self.params.arpeggiator.edit(|s| {
                    let index = if append {
                        s.insert_blank_row_after((origin + s.rows.len() - 1) % s.rows.len())
                    } else {
                        empty.unwrap()
                    };
                    s.rows[index].chord = Some(SavedChord {
                        root,
                        second: None,
                        additions: 0,
                        quality: 0,
                        inversion: 0,
                        spread: 0,
                        transpose: 0,
                    });
                    self.arp_ui.edit_page = index;
                });
                self.tracker_selection = true;
                self.reveal_tracker_selection();
                // Keep the next blank column within reach after appending.
                if append {
                    let state = self.arp_state();
                    self.tracker_scroll_target =
                        (row_start(&state.rows, state.rows.len()) + loop_width(&state.rows) - grid_rect().2)
                            .max(0.0);
                }
            } else if let Some((index, rr)) =
                blocks(rows, self.tracker_scroll, origin).find(|(_, r)| hit(*r, x, y))
            {
                self.arp_ui.edit_page = index;
                self.tracker_selection = true;
                if matches!(event, WindowEvent::MouseDoubleClick(MouseButton::Left)) {
                    self.tracker_selection = false;
                    self.tracker_symbol = rows[index]
                        .chord
                        .map(harmony::chord_name)
                        .unwrap_or_default();
                    self.tracker_symbol_edit = true;
                } else {
                    self.tracker_scroll_target = self.tracker_scroll;
                    self.tracker_drag = Some(TrackerDrag::block(
                        &state,
                        index,
                        (x, y),
                        self.tracker_scroll,
                        x >= rr.0 + rr.2 - 8.0,
                    ));
                    cx.capture();
                }
            }
        }
        cx.needs_redraw();
        true
    }

    fn reveal_tracker_selection(&mut self) {
        let state = self.arp_state();
        let selected = self.arp_ui.edit_page.min(state.rows.len() - 1);
        let left = row_start_from(&state.rows, selected, state.view_row_start());
        let right = left + state.rows[selected].duration as f32 * loop_width(&state.rows);
        self.tracker_scroll_target = self
            .tracker_scroll_target
            .max(right - grid_rect().2)
            .min(left)
            .clamp(0.0, max_scroll(&state.rows));
        self.tracker_scroll = self.tracker_scroll.min(max_scroll(&state.rows));
        self.tracker_proxy_row = usize::MAX;
    }
    pub(super) fn finish_tracker_symbol(&mut self) {
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
