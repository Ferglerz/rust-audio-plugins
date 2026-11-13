use super::*;

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
pub(super) fn delay_feedback_rect() -> Rect {
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

pub(super) fn tap_graph() -> Rect {
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
pub(super) fn tap_x(tap: &DelayTap, domain: f32) -> f32 {
    let g = tap_graph();
    g.0 + (tap.time / domain).clamp(0.0, 1.0) * g.2
}

impl ChordboardView {
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
            let domain = time_domain(delay);
            // Numbered caps are separate horizontal handles on every adjustment lane.
            let nearest = (0..delay.count as usize)
                .min_by(|a, b| {
                    (tap_x(&delay.taps[*a], domain) - x)
                        .abs()
                        .total_cmp(&(tap_x(&delay.taps[*b], domain) - x).abs())
                })
                .unwrap_or(0);
            let tx = tap_x(&delay.taps[nearest], domain);
            if (x - tx).abs() < 22.0 && y >= g.1 - 18.0 && y < g.1 {
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
            } else if hit(g, x, y) {
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

    pub(in crate::ui) fn draw_delay_strings(&self, d: &mut Draw, delay: &DelaySettings) {
        let s = DELAY_SURFACE;
        let selected = self.arp_ui.selected_tap.min(delay.count as usize - 1);
        let tap = &delay.taps[selected];
        let u = &self.arp_ui;
        d.text(s.0 + 48.0, s.1 + 29.0, "MIDI DELAY", 16.0, TEXT);
        d.text(
            s.0 + 210.0,
            s.1 + 28.0,
            &format!("Tap {} of {}", selected + 1, delay.count),
            13.0,
            MUTED,
        );
        let power = delay_power_rect();
        self.draw_power_switch(
            d,
            power,
            delay.enabled,
            TEAL,
            self.fx_delay_screen_rect(power),
        );
        d.line(
            s.0 + 12.0,
            s.1 + 43.0,
            super::super::midi_fx::source_rect(0).0 - 24.0,
            s.1 + 43.0,
            LINE,
            1.0,
        );
        self.arp_icon_button(
            d,
            delay_sync_rect(),
            "",
            delay.sync,
            TEAL,
            if delay.sync {
                style::Icon::Host
            } else {
                style::Icon::Glyph(style::GLYPH_CLOCK)
            },
        );
        self.arp_value(
            d,
            delay_time_rect(),
            &format!("Time: {}", time_label(tap.time, delay.sync)),
            tap.time / time_range(delay.sync).1,
        );
        self.arp_value(
            d,
            delay_gain_rect(),
            &format!("Velocity: {:.0}%", tap.velocity * 100.0),
            tap.velocity,
        );
        self.arp_value(
            d,
            delay_feedback_rect(),
            &format!("Feedback: {:.0}%", tap.feedback * 100.0),
            tap.feedback,
        );
        self.arp_button(
            d,
            tap_lane_rect(),
            &format!("{} ▾", u.tap_lane.label()),
            u.dropdown == Some(Dropdown::TapLane),
            GOLD,
        );
        self.arp_icon_button(
            d,
            ping_rect(),
            "",
            delay.ping_pong,
            TEAL,
            style::Icon::Glyph(style::GLYPH_SHUFFLE),
        );
        self.arp_icon_button(
            d,
            delay_leading_rect(),
            "",
            delay.voice_leading,
            TEAL,
            style::Icon::Glyph(style::GLYPH_ROUTE),
        );
        self.arp_icon_button(
            d,
            delay_follow_rect(),
            "",
            delay.follow_chords,
            TEAL,
            style::Icon::Glyph(style::GLYPH_REPLACE),
        );
        self.draw_delay_pattern(d, delay);
        if self.arp_ui.dropdown == Some(Dropdown::TapLane) {
            self.draw_arp_dropdown(d, Dropdown::TapLane);
        }
    }

    fn draw_delay_pattern(&self, d: &mut Draw, delay: &DelaySettings) {
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
                0 => d.text(x, y, &label, TEXT_LABEL, MUTED),
                4 => d.text_right(x, y, &label, TEXT_LABEL, MUTED),
                _ => d.text_centered(x, y, &label, TEXT_LABEL, MUTED),
            }
        }
        d.line(g.0, baseline, g.0 + g.2, baseline, LINE, 1.0);
        // Paint echo trains first so their faint lines never cover tap handles.
        for (i, t) in delay.taps[..delay.count as usize].iter().enumerate() {
            if !t.enabled || t.velocity < 1.0 / 127.0 || t.time <= 0.0 {
                continue;
            }
            let color = if i == selected { GOLD } else { TEAL };
            let y = value_y(u.tap_lane.get(t), range, g);
            let mut decay = 1.0;
            for generation in 2..=64 {
                decay *= t.feedback;
                let time = t.time * generation as f32;
                if time > domain || t.velocity * decay < 1.0 / 127.0 {
                    break;
                }
                let x = g.0 + time / domain * g.2;
                // Only velocity falls with feedback; pitch, pan and channel
                // retain their lane values while the lines fade in opacity.
                let echo_y = if matches!(u.tap_lane, TapLane::Velocity) {
                    baseline + (y - baseline) * decay
                } else {
                    y
                };
                d.line(x, baseline, x, echo_y, alpha(color, decay * 0.65), 2.0);
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
            d.text_centered(x, g.1 - 6.0, &format!("{} ↔", i + 1), 13.0, color);
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
                self.button_surface(d, r, confirming, color);
                if confirming {
                    self.close_icon(d, r, color, true);
                } else {
                    let (cx, cy) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
                    d.line(cx - 3.5, cy - 3.5, cx + 3.5, cy + 3.5, color, 1.5);
                    d.line(cx - 3.5, cy + 3.5, cx + 3.5, cy - 3.5, color, 1.5);
                }
            }
        }
        if (delay.count as usize) < TAPS {
            let r = add_rect(delay, domain);
            self.button_surface(d, r, false, TEAL);
            let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
            d.line(x - 6.0, y, x + 6.0, y, TEAL, 2.0);
            d.line(x, y - 6.0, x, y + 6.0, TEAL, 2.0);
        }
    }
}
