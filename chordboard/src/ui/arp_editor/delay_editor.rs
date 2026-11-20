use super::*;
use pleasant_ui::draw::{READABLE_AXIS_SIZE, READABLE_LABEL_SIZE};

pub(in crate::ui) fn delay_power_rect() -> Rect {
    (DELAY_SURFACE.0 + 12.0, DELAY_SURFACE.1 + 8.0, 28.0, 28.0)
}
pub(super) fn delay_sync_rect() -> Rect {
    (DELAY_SURFACE.0 + 20.0, DELAY_SURFACE.1 + 48.0, 106.0, 26.0)
}
pub(super) fn delay_time_rect() -> Rect {
    (DELAY_SURFACE.0 + 134.0, DELAY_SURFACE.1 + 48.0, 144.0, 26.0)
}
pub(super) fn delay_gain_rect() -> Rect {
    (DELAY_SURFACE.0 + 286.0, DELAY_SURFACE.1 + 48.0, 132.0, 26.0)
}
pub(in crate::ui) fn delay_feedback_rect() -> Rect {
    (DELAY_SURFACE.0 + 426.0, DELAY_SURFACE.1 + 48.0, 140.0, 26.0)
}
pub(super) fn tap_lane_rect() -> Rect {
    (DELAY_SURFACE.0 + 574.0, DELAY_SURFACE.1 + 48.0, 140.0, 26.0)
}
pub(super) fn ping_rect() -> Rect {
    (DELAY_SURFACE.0 + 730.0, DELAY_SURFACE.1 + 48.0, 104.0, 26.0)
}
pub(in crate::ui) fn delay_leading_rect() -> Rect {
    (DELAY_SURFACE.0 + 850.0, DELAY_SURFACE.1 + 48.0, 144.0, 26.0)
}
pub(in crate::ui) fn delay_follow_rect() -> Rect {
    (
        DELAY_SURFACE.0 + 1002.0,
        DELAY_SURFACE.1 + 48.0,
        144.0,
        26.0,
    )
}
// Keep the add control in the timeline, beside the next tap position.
pub(super) fn next_tap_time(delay: &DelaySettings) -> f32 {
    let last = delay.taps[..delay.count as usize]
        .iter()
        .map(|t| t.time)
        .fold(0.0, f32::max);
    (last + if delay.sync { 0.5 } else { 250.0 }).min(time_range(delay.sync).1)
}
pub(in crate::ui) fn add_rect(delay: &DelaySettings, domain: f32) -> Rect {
    let g = tap_graph();
    let next = g.0 + next_tap_time(delay) / domain * g.2;
    let last = delay.taps[..delay.count as usize]
        .iter()
        .map(|t| tap_x(t, domain))
        .fold(g.0, f32::max);
    let x = next
        .max(last + 42.0)
        .min(super::super::midi_fx::source_rect(0).0 - 28.0);
    (x - 12.0, g.1 + g.3 * 0.5 - 12.0, 24.0, 24.0)
}
pub(in crate::ui) fn tap_remove_rect(tap: &DelayTap, domain: f32) -> Rect {
    (
        tap_x(tap, domain) - 9.0,
        tap_graph().1 + tap_graph().3 + 18.0,
        18.0,
        18.0,
    )
}
#[cfg(test)]
pub(in crate::ui) fn source_rect(index: usize) -> Rect {
    super::super::midi_fx::source_rect(index)
}

pub(in crate::ui) fn tap_graph() -> Rect {
    (
        DELAY_SURFACE.0 + 26.0,
        DELAY_SURFACE.1 + 94.0,
        DELAY_SURFACE.2 - super::super::midi_fx::SOURCE_COLUMN_W - 96.0,
        DELAY_SURFACE.3 - 132.0,
    )
}
pub(super) fn time_range(sync: bool) -> (f32, f32) {
    if sync {
        (1.0 / 64.0, 16.0)
    } else {
        (1.0, 8000.0)
    }
}
// Keep a stable domain while dragging so the selected tap never runs away from the pointer.
pub(in crate::ui) fn time_domain(delay: &DelaySettings) -> f32 {
    delay.taps[..delay.count as usize]
        .iter()
        .map(|t| t.time)
        .fold(if delay.sync { 2.0 } else { 1000.0 }, f32::max)
        // Show the source tap and its first three feedback repeats, including
        // taps at the maximum interval. This is a display range, not a time limit.
        .mul_add(4.25, 0.0)
}
pub(in crate::ui) fn tap_x(tap: &DelayTap, domain: f32) -> f32 {
    let g = tap_graph();
    g.0 + (tap.time / domain).clamp(0.0, 1.0) * g.2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delay_spotlight_uses_only_visible_playable_repeats() {
        let mut tap = DelayTap {
            time: 0.5,
            velocity: 1.0,
            feedback: 0.5,
            ..DelayTap::default()
        };
        assert_eq!(
            echo_repeats(&tap, 1.5).collect::<Vec<_>>(),
            [(1.0, 0.5), (1.5, 0.25)]
        );
        tap.feedback = 0.0;
        assert_eq!(echo_repeats(&tap, 4.0).count(), 0);
        tap.feedback = 0.5;
        tap.velocity = 0.01;
        assert_eq!(echo_repeats(&tap, 4.0).count(), 0);
        tap.velocity = 1.0;
        tap.enabled = false;
        assert_eq!(echo_repeats(&tap, 4.0).count(), 0);
    }
}

fn nearest_tap(delay: &DelaySettings, domain: f32, x: f32) -> Option<usize> {
    delay.taps[..delay.count as usize]
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (tap_x(a, domain) - x)
                .abs()
                .total_cmp(&(tap_x(b, domain) - x).abs())
        })
        .map(|(index, _)| index)
}

fn tap_handle_at(delay: &DelaySettings, domain: f32, x: f32, y: f32) -> Option<usize> {
    let top = tap_graph().1;
    if !(top - 18.0..top).contains(&y) {
        return None;
    }
    nearest_tap(delay, domain, x).filter(|&i| (x - tap_x(&delay.taps[i], domain)).abs() < 22.0)
}

fn echo_repeats(tap: &DelayTap, domain: f32) -> impl Iterator<Item = (f32, f32)> + '_ {
    let mut decay = 1.0;
    let playable = tap.enabled && tap.time > 0.0 && tap.velocity >= 1.0 / 127.0;
    (2..=64).map_while(move |generation| {
        decay *= tap.feedback;
        let time = tap.time * generation as f32;
        (playable && time <= domain && tap.velocity * decay >= 1.0 / 127.0).then_some((time, decay))
    })
}

fn draw_delay_icon(d: &mut Draw, r: Rect, on: bool, icon: style::Icon) {
    d.button_surface(r, TEAL, on as u8 as f32, d.is_hovered(r) as u8 as f32, 0.0);
    icon.draw(
        d,
        r.0 + r.2 * 0.5,
        r.1 + r.3 * 0.5,
        if on { TEAL } else { MUTED },
    );
}

fn draw_delay_value(d: &mut Draw, r: Rect, label: &str, value: &str, norm: f32) {
    d.button_surface(r, GOLD, 0.0, d.is_hovered(r) as u8 as f32, 0.0);
    let label_width = d.with_label_font(|d| d.text_width(label, READABLE_LABEL_SIZE));
    let gap = 5.0;
    let width = label_width + gap + d.text_width(value, READABLE_LABEL_SIZE);
    let x = r.0 + (r.2 - width) * 0.5;
    let y = r.1 + r.3 * 0.5 + 2.0;
    d.with_label_font(|d| d.text(x, y, label, READABLE_LABEL_SIZE, MUTED));
    d.text(x + label_width + gap, y, value, READABLE_LABEL_SIZE, TEXT);
    let rail = (r.0 + 8.0, r.1 + r.3 - 4.0, r.2 - 16.0);
    d.line(rail.0, rail.1, rail.0 + rail.2, rail.1, LINE, 2.0);
    d.line(
        rail.0,
        rail.1,
        rail.0 + rail.2 * norm.clamp(0.0, 1.0),
        rail.1,
        GOLD,
        2.0,
    );
}

impl ChordboardView {
    fn delay_spotlight(&self, delay: &DelaySettings, x: f32, y: f32) -> Option<usize> {
        if !self.fx_ui.open
            || !self.fx_delay_expanded()
            || self.menu.is_some()
            || self.quick_midi.show
            || self.fx_ui.drag.is_some()
            || self.arp_ui.drag.is_some()
            || self.arp_ui.dropdown.is_some()
            || !hit(super::super::midi_fx::VIEWPORT, x, y)
            || crate::midi_learn::value(
                &self.params,
                crate::midi_learn::fx_power_target(crate::midi_fx::MidiFxId::Delay),
            ) == 0
        {
            return None;
        }
        let y = self.fx_delay_local_y(y);
        if hit(delay_feedback_rect(), x, y) {
            return Some(self.arp_ui.selected_tap.min(delay.count as usize - 1));
        }
        tap_handle_at(delay, time_domain(delay), x, y)
    }

    pub(super) fn delay_spotlight_hint(&self, x: f32, y: f32) -> Option<String> {
        let state = self.arp_state();
        let delay = &state.midi_fx.delay;
        let index = self.delay_spotlight(delay, x, y)?;
        let tap = &delay.taps[index];
        Some(format!(
            "Settings preview · Tap {} · Interval {} · Feedback {:.0}%{}",
            index + 1,
            time_label(tap.time, delay.sync),
            tap.feedback * 100.0,
            if tap.enabled { "" } else { " · Tap off" }
        ))
    }

    pub(super) fn delay_editor_event(
        &mut self,
        cx: &mut EventContext,
        event: &WindowEvent,
        x: f32,
        y: f32,
        press: bool,
    ) -> bool {
        let state = self.arp_state();
        let delay = &state.midi_fx.delay;
        let selected = self.arp_ui.selected_tap.min(delay.count as usize - 1);
        let tap = delay.taps[selected];
        let reset = cx.modifiers().intersects(Modifiers::CTRL | Modifiers::LOGO);
        if let WindowEvent::MouseScroll(_, dy) = event {
            if ![delay_time_rect(), delay_gain_rect(), delay_feedback_rect()]
                .iter()
                .any(|r| hit(*r, x, y))
            {
                return false;
            }
            if *dy == 0.0 {
                return true;
            }
            self.params.arpeggiator.edit(|s| {
                let t = &mut s.midi_fx.delay.taps[selected];
                if hit(delay_time_rect(), x, y) {
                    if delay.sync {
                        let divisions = [
                            1.0 / 64.0,
                            1.0 / 32.0,
                            1.0 / 16.0,
                            1.0 / 8.0,
                            1.0 / 6.0,
                            1.0 / 4.0,
                            1.0 / 3.0,
                            0.5,
                            0.75,
                            1.0,
                            1.5,
                            2.0,
                            3.0,
                            4.0,
                            8.0,
                            16.0,
                        ];
                        t.time = if *dy > 0.0 {
                            divisions
                                .iter()
                                .copied()
                                .find(|v| *v > t.time + 0.0001)
                                .unwrap_or(16.0)
                        } else {
                            divisions
                                .iter()
                                .rev()
                                .copied()
                                .find(|v| *v < t.time - 0.0001)
                                .unwrap_or(1.0 / 64.0)
                        };
                    } else {
                        t.time = (t.time + dy.signum() * 10.0).clamp(1.0, 8000.0)
                    }
                } else if hit(delay_gain_rect(), x, y) {
                    t.velocity = (t.velocity + dy.signum() * 0.01).clamp(0.0, 2.0)
                } else if hit(delay_feedback_rect(), x, y) {
                    t.feedback = (t.feedback + dy.signum() * 0.01).clamp(0.0, 0.95)
                }
            });
            cx.needs_redraw();
            return true;
        }
        if !press {
            return matches!(event, WindowEvent::MouseDown(_) | WindowEvent::MouseUp(_));
        }
        cx.focus();
        self.focused = true;
        let domain = time_domain(delay);
        if delay.count > 1 {
            if let Some(index) = (0..delay.count as usize)
                .find(|&i| hit(tap_remove_rect(&delay.taps[i], domain), x, y))
            {
                if self.arp_ui.pending_tap_remove == Some(index) {
                    self.params.arpeggiator.edit(|s| {
                        let d = &mut s.midi_fx.delay;
                        // These are independent times, not gaps: copy each survivor intact.
                        d.taps.copy_within(index + 1..d.count as usize, index);
                        d.count -= 1;
                    });
                    self.arp_ui.selected_tap = if selected > index {
                        selected - 1
                    } else {
                        selected.min(delay.count as usize - 2)
                    };
                    self.arp_ui.pending_tap_remove = None;
                } else {
                    self.arp_ui.pending_tap_remove = Some(index);
                }
                cx.needs_redraw();
                return true;
            }
        }
        self.arp_ui.pending_tap_remove = None;
        if hit(delay_sync_rect(), x, y) {
            let beat_ms = 60_000.0 / self.snapshot.tempo.max(1.0);
            self.params.arpeggiator.edit(|s| {
                let d = &mut s.midi_fx.delay;
                d.sync = !delay.sync;
                for t in &mut d.taps {
                    t.time = if d.sync {
                        t.time / beat_ms
                    } else {
                        t.time * beat_ms
                    }
                }
            });
        } else if hit(tap_lane_rect(), x, y) {
            self.arp_ui.dropdown = Some(Dropdown::TapLane)
        } else if hit(delay_time_rect(), x, y) {
            self.start_arp_drag(cx, DragKind::BasicTime, selected, tap.time, (x, y), reset);
            if reset {
                self.apply_arp_drag(cx, x, y)
            }
        } else if hit(delay_gain_rect(), x, y) {
            self.start_arp_drag(
                cx,
                DragKind::BasicGain,
                selected,
                tap.velocity,
                (x, y),
                reset,
            );
            if reset {
                self.apply_arp_drag(cx, x, y)
            }
        } else if hit(delay_feedback_rect(), x, y) {
            self.start_arp_drag(
                cx,
                DragKind::BasicFeedback,
                selected,
                tap.feedback,
                (x, y),
                reset,
            );
            if reset {
                self.apply_arp_drag(cx, x, y)
            }
        } else if hit(delay_leading_rect(), x, y) {
            self.params
                .arpeggiator
                .edit(|s| s.midi_fx.delay.voice_leading = !delay.voice_leading)
        } else if hit(delay_follow_rect(), x, y) {
            self.params
                .arpeggiator
                .edit(|s| s.midi_fx.delay.follow_chords = !delay.follow_chords)
        } else if hit(ping_rect(), x, y) {
            self.params
                .arpeggiator
                .edit(|s| s.midi_fx.delay.ping_pong = !delay.ping_pong)
        } else if (delay.count as usize) < TAPS && hit(add_rect(delay, domain), x, y) {
            if (delay.count as usize) < TAPS {
                let index = delay.count as usize;
                let t = DelayTap {
                    time: next_tap_time(delay),
                    ..Default::default()
                };
                self.params.arpeggiator.edit(|s| {
                    let d = &mut s.midi_fx.delay;
                    d.taps[index] = t;
                    d.count += 1;
                });
                self.arp_ui.selected_tap = index
            }
        } else {
            let g = tap_graph();
            // Numbered caps are separate horizontal handles on every adjustment lane.
            if let Some(nearest) = tap_handle_at(delay, domain, x, y) {
                self.arp_ui.selected_tap = nearest;
                self.start_arp_drag(
                    cx,
                    DragKind::TapTime,
                    nearest,
                    delay.taps[nearest].time,
                    (x, y),
                    reset,
                );
                if let Some(d) = &mut self.arp_ui.drag {
                    d.last_value = domain;
                }
                if reset {
                    self.apply_arp_drag(cx, x, y)
                }
            } else if let Some(nearest) = nearest_tap(delay, domain, x).filter(|_| hit(g, x, y)) {
                self.arp_ui.selected_tap = nearest;
                let lane = self.arp_ui.tap_lane;
                self.start_arp_drag(
                    cx,
                    DragKind::Tap(lane),
                    nearest,
                    lane.get(&delay.taps[nearest]),
                    (x, y),
                    reset,
                );
                if reset {
                    self.apply_arp_drag(cx, x, y)
                }
            }
        }
        cx.needs_redraw();
        true
    }

    pub(in crate::ui) fn draw_delay_heading(&self, d: &mut Draw, enabled: bool) {
        let s = DELAY_SURFACE;
        d.with_label_font(|d| {
            let x = s.0 + 48.0;
            let title = "MIDI Delay";
            d.text(x, s.1 + 29.0, title, 16.0, TEXT);
            if !enabled {
                let status_x = x + d.text_width(title, 16.0) + 12.0;
                d.text(status_x, s.1 + 29.0, "Bypassed", READABLE_LABEL_SIZE, MUTED);
            }
        });
        let power = delay_power_rect();
        self.draw_power_switch(d, power, enabled, TEAL, self.fx_delay_screen_rect(power));
    }

    pub(in crate::ui) fn draw_delay_strings(&self, d: &mut Draw, delay: &DelaySettings) {
        let s = DELAY_SURFACE;
        let selected = self.arp_ui.selected_tap.min(delay.count as usize - 1);
        let tap = &delay.taps[selected];
        let u = &self.arp_ui;
        let spotlight = self
            .hover_pointer()
            .and_then(|(x, y)| self.delay_spotlight(delay, x, y));
        self.draw_delay_heading(d, delay.enabled);
        d.with_label_font(|d| {
            d.text(
                s.0 + 252.0,
                s.1 + 28.0,
                &format!("Tap {} of {}", selected + 1, delay.count),
                READABLE_LABEL_SIZE,
                MUTED,
            )
        });
        d.line(
            s.0 + 12.0,
            s.1 + 43.0,
            super::super::midi_fx::source_rect(0).0 - 24.0,
            s.1 + 43.0,
            LINE,
            1.0,
        );
        draw_delay_icon(
            d,
            delay_sync_rect(),
            delay.sync,
            if delay.sync {
                style::Icon::Host
            } else {
                style::Icon::Glyph(style::GLYPH_CLOCK)
            },
        );
        draw_delay_value(
            d,
            delay_time_rect(),
            "Time",
            &time_label(tap.time, delay.sync),
            tap.time / time_range(delay.sync).1,
        );
        draw_delay_value(
            d,
            delay_gain_rect(),
            "Velocity",
            &format!("{:.0}%", tap.velocity * 100.0),
            tap.velocity,
        );
        draw_delay_value(
            d,
            delay_feedback_rect(),
            "Feedback",
            &format!("{:.0}%", tap.feedback * 100.0),
            tap.feedback,
        );
        d.button_tinted(
            tap_lane_rect(),
            &format!("{} ▾", u.tap_lane.label()),
            u.dropdown == Some(Dropdown::TapLane),
            GOLD,
        );
        draw_delay_icon(
            d,
            ping_rect(),
            delay.ping_pong,
            style::Icon::Glyph(style::GLYPH_SHUFFLE),
        );
        draw_delay_icon(
            d,
            delay_leading_rect(),
            delay.voice_leading,
            style::Icon::Glyph(style::GLYPH_ROUTE),
        );
        draw_delay_icon(
            d,
            delay_follow_rect(),
            delay.follow_chords,
            style::Icon::Glyph(style::GLYPH_REPLACE),
        );
        self.draw_delay_pattern(d, delay, spotlight);
        if self.arp_ui.dropdown == Some(Dropdown::TapLane) {
            self.draw_arp_dropdown(d, Dropdown::TapLane);
        }
    }

    fn draw_delay_pattern(&self, d: &mut Draw, delay: &DelaySettings, spotlight: Option<usize>) {
        let u = &self.arp_ui;
        let selected = u.selected_tap.min(delay.count as usize - 1);
        let g = tap_graph();
        let domain =
            if let Some(drag) = u.drag.filter(|drag| matches!(drag.kind, DragKind::TapTime)) {
                drag.last_value
            } else {
                time_domain(delay)
            };
        let range = u.tap_lane.range();
        let baseline = value_y(if range.0 < 0.0 { 0.0 } else { range.0 }, range, g);
        for i in 0..=4 {
            let x = g.0 + i as f32 * g.2 / 4.0;
            dotted(d, x, g.1, g.3, alpha(LINE, 0.55));
            let label = time_label(domain * i as f32 / 4.0, delay.sync);
            let y = g.1 + g.3 + 14.0;
            match i {
                0 => d.text(x, y, &label, READABLE_AXIS_SIZE, MUTED),
                4 => d.text_right(x, y, &label, READABLE_AXIS_SIZE, MUTED),
                _ => d.text_centered(x, y, &label, READABLE_AXIS_SIZE, MUTED),
            }
        }
        d.line(g.0, baseline, g.0 + g.2, baseline, LINE, 1.0);
        // Paint the focused train above the other trains and behind the tap handles.
        for i in (0..delay.count as usize)
            .filter(|&i| Some(i) != spotlight)
            .chain(spotlight)
        {
            let t = &delay.taps[i];
            let focused = spotlight == Some(i);
            let color = if focused || i == selected { GOLD } else { TEAL };
            let y = value_y(u.tap_lane.get(t), range, g);
            let mut last_marker = f32::NEG_INFINITY;
            for (time, decay) in echo_repeats(t, domain) {
                let x = g.0 + time / domain * g.2;
                // Only velocity falls with feedback; pitch, pan and channel
                // retain their lane values while the lines fade in opacity.
                let echo_y = if matches!(u.tap_lane, TapLane::Velocity) {
                    baseline + (y - baseline) * decay
                } else {
                    y
                };
                let opacity = if focused {
                    0.70 + 0.30 * decay
                } else if spotlight.is_some() {
                    0.24 + 0.24 * decay
                } else {
                    0.65 * decay
                };
                d.line(
                    x,
                    baseline,
                    x,
                    echo_y,
                    alpha(color, opacity),
                    if focused { 2.5 } else { 2.0 },
                );
                if focused && x - last_marker >= 6.0 {
                    d.circle(x, echo_y, 2.5 + decay, color, true);
                    if d.light {
                        d.circle(x, echo_y, 2.5 + decay, MUTED, false);
                    }
                    last_marker = x;
                }
            }
        }
        for i in 0..delay.count as usize {
            let t = &delay.taps[i];
            let x = tap_x(t, domain);
            let y = value_y(u.tap_lane.get(t), range, g);
            let color = if i == selected { GOLD } else { TEAL };
            d.line(x, g.1, x, g.1 + g.3, alpha(color, 0.3), 1.0);
            d.line(
                x,
                baseline,
                x,
                y,
                color,
                if i == selected { 8.0 } else { 6.0 },
            );
            d.circle(x, y, 6.0, color, true);
            d.rounded_rect(x - 20.0, g.1 - 18.0, 40.0, 16.0, 4.0, alpha(color, 0.18));
            d.text_centered(
                x,
                g.1 - 6.0,
                &format!("{} ↔", i + 1),
                READABLE_LABEL_SIZE,
                TEXT,
            );
            if i == selected {
                d.line(x - 14.0, g.1 - 2.0, x + 14.0, g.1 - 2.0, GOLD, 2.0);
            }
            self.string_readout(
                d,
                x,
                y,
                g,
                &u.tap_lane.format(u.tap_lane.get(t)),
                if i == selected { TEXT } else { MUTED },
            );
            if delay.count > 1 {
                let r = tap_remove_rect(t, domain);
                let confirming = u.pending_tap_remove == Some(i);
                d.rect(r.0, r.1, r.2, r.3, LINE);
                self.button_surface_at(d, r, self.fx_delay_screen_rect(r), confirming, color);
                if confirming {
                    self.close_icon(d, r, color, true);
                } else {
                    let (cx, cy) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
                    d.line(cx - 3.5, cy - 3.5, cx + 3.5, cy + 3.5, color, 1.5);
                    d.line(cx - 3.5, cy + 3.5, cx + 3.5, cy - 3.5, color, 1.5);
                }
            }
        }
        if let Some(i) = spotlight {
            let x = tap_x(&delay.taps[i], domain);
            d.outline_rounded(
                x - 20.0,
                g.1 - 18.0,
                40.0,
                16.0,
                4.0,
                if d.light { MUTED } else { GOLD },
                1.5,
            );
        }
        if (delay.count as usize) < TAPS {
            let r = add_rect(delay, domain);
            self.button_surface_at(d, r, self.fx_delay_screen_rect(r), false, TEAL);
            let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
            d.line(x - 6.0, y, x + 6.0, y, TEAL, 2.0);
            d.line(x, y - 6.0, x, y + 6.0, TEAL, 2.0);
        }
    }
}
