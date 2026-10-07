use super::*;
use nih_plug_vizia::vizia::vg::Paint;

pub(super) struct ControlMotion {
    rect: Rect,
    hover: f32,
    selection: f32,
}

impl ChordboardView {
    pub(super) fn close_icon(&self, d: &mut Draw, r: Rect, color: Color, confirm: bool) {
        if self.pointer.is_some_and(|(x, y)| hit(r, x, y)) {
            d.rounded_rect(r.0, r.1, r.2, r.3, 4.0, alpha(color, 0.18));
        }
        let (x, y) = (r.0 + r.2 * 0.5, r.1 + r.3 * 0.5);
        let size = (r.3 * 0.30).min(9.0);
        if confirm {
            d.poly(
                &[
                    (x - size, y),
                    (x - size * 0.2, y + size * 0.65),
                    (x + size, y - size * 0.75),
                ],
                color,
                2.5,
            );
        } else {
            d.line(x - size, y - size, x + size, y + size, color, 2.5);
            d.line(x - size, y + size, x + size, y - size, color, 2.5);
        }
    }

    fn motion_targets(&self) -> Vec<(Rect, bool)> {
        if let Some(menu) = self.menu {
            return (0..menu.items().len())
                .map(|i| {
                    (
                        self.menu_option_rect(menu, i),
                        i == self.menu_selection(menu),
                    )
                })
                .collect();
        }
        let mut targets = Vec::new();
        targets.extend(self.route_chips().into_iter().map(|(_, r)| (r, false)));
        if self.expand_t() == 0.0 {
            targets
                .extend((0..crate::engine::routing::SOURCE_COUNT).map(|i| (meter_rect(i), false)));
        }
        if let Some(panel) = self.panel {
            targets.push((self.panel_close_rect(panel), false));
            targets.extend(
                self.panel_controls()
                    .iter()
                    .map(|(c, r)| (*r, c.toggle && c.norm >= 0.5)),
            );
            if panel == Panel::Routes {
                targets.extend(
                    self.editor_route_chips()
                        .into_iter()
                        .map(|(slot, r)| (r, slot == self.route_slot)),
                );
                if let Some(r) = self.add_route_rect() {
                    targets.push((r, false));
                }
                targets.extend([
                    (ROUTE_SOURCE, false),
                    (ROUTE_TARGET, false),
                    (ROUTE_CLEAR, false),
                ]);
            }
            if panel == Panel::Mapping {
                targets.push((self.menu_trigger_rect(Menu::MappingKind), false));
                if self.mapping().kind == 3 {
                    targets.push((self.menu_trigger_rect(Menu::MappingChannel(true)), false));
                }
                if matches!(self.mapping().kind, 1 | 2) {
                    targets.push((self.menu_trigger_rect(Menu::MappingCc(false)), false));
                }
                if self.mapping_axis == 1 {
                    targets.push((self.menu_trigger_rect(Menu::YTarget), false));
                }
            }
            if panel == Panel::Mapping {
                return targets;
            }
        }
        targets.extend([
            (ALWAYS_BASS, self.params.always_bass.value()),
            (
                key_split_rect(self.params.mpe_enabled()),
                self.params.key_split.value(),
            ),
            (
                melody_split_rect(self.params.mpe_enabled()),
                self.params.key_split.value(),
            ),
            (APPEARANCE, false),
            (LATCH, self.params.latch.value()),
            (ORDER, true),
            (self.control_octave_bounds(), self.learning() == 4),
            (TEMPO_SYNC, true),
            (Menu::Key.trigger_rect(), true),
            (Menu::Scale.trigger_rect(), true),
        ]);
        if self.can_expand_strum() {
            targets.push((self.expand_button(), self.expand_target > 0.5));
        }
        targets.extend((0..KEY_COUNT).map(|i| (self.keyboard_key_rect(i), false)));
        targets.extend((0..8).map(|i| (memory_rect(i), self.memory_ui.armed)));
        targets.push((BASS_BYPASS, !self.params.bass_enabled.value()));
        targets.push((INTERLOCK, self.params.interlock.value() != 0));
        targets.push((PIANO_LEADING, self.params.voice_leading.value() != 2));
        targets.push((self.split_marker(), false));
        if let Some(r) = self.bass_marker() {
            targets.push((r, false));
        }
        targets.push((
            affect_chords_rect(self.params.mpe_enabled()),
            self.params.affect_chords.value(),
        ));
        targets.push((
            output_protocol_rect(self.params.mpe_enabled(), 0),
            self.params.mpe_enabled(),
        ));
        targets
            .extend((0..2).map(|i| (inversion_control_rect(self.params.mpe_enabled(), i), false)));
        targets
            .extend((0..5).map(|i| (transpose_control_rect(self.params.mpe_enabled(), i), false)));
        targets.push((HEADER_PLAY_TAB, !self.sequencer_open()));
        targets.push((HEADER_SEQ_TAB, self.sequencer_open()));
        targets.push((
            strum_bypass_rect(self.expand_t()),
            self.params.strum_enabled.value(),
        ));
        if self.arp_main() {
            targets.extend(
                [false, true].map(|looping| (repeat_rect(looping), (self.mode() == 3) == looping)),
            );
        }
        targets.extend(self.base_controls().iter().map(|(c, r)| {
            (
                *r,
                if c.id == "bass_split" {
                    self.learning() == 5
                } else if c.id == "root_on_select" {
                    self.selection_mode() > 0
                } else {
                    c.toggle && c.norm >= 0.5
                },
            )
        }));
        match self.mode() {
            1 | 3 => {
                targets.push((RATE_SYNC, self.sweep_synced()));
                if self.mode() == 1 {
                    targets.push((STRUM_HOLD, self.params.strum_hold.value()));
                }
                targets.extend(
                    (0..7).map(|i| (pattern_rect(i), self.params.arp_pattern.value() == i as i32)),
                );
                if self.sweep_synced() {
                    targets.extend(ARP_RATES.iter().enumerate().map(|(i, (_, beats))| {
                        (
                            rate_rect(i),
                            (self.params.rate.value() - beats).abs() < 0.0001,
                        )
                    }));
                }
                targets.extend(
                    (0..4).map(|i| (octave_rect(i), self.params.octaves.value() == i as i32 + 1)),
                );
            }
            2 | 4 => targets.push((
                strum_latch_rect(self.expand_t()),
                self.params.strum_latch.value(),
            )),
            _ => {}
        }
        targets
    }

    pub(super) fn tick_motion(&mut self, dt: f32) -> bool {
        let targets = self.motion_targets();
        let mut changed = false;
        self.motion
            .retain(|m| targets.iter().any(|(r, _)| *r == m.rect));
        for (rect, selected) in targets {
            if !self.motion.iter().any(|m| m.rect == rect) {
                self.motion.push(ControlMotion {
                    rect,
                    hover: 0.0,
                    selection: selected as u8 as f32,
                });
            }
            let hovered = self.hover_pointer().is_some_and(|(x, y)| hit(rect, x, y));
            if let Some(m) = self.motion.iter_mut().find(|m| m.rect == rect) {
                for (value, target, duration) in [
                    (&mut m.hover, hovered, 0.10),
                    (&mut m.selection, selected, 0.18),
                ] {
                    let next = *value
                        + ((target as u8 as f32) - *value).clamp(-dt / duration, dt / duration);
                    changed |= next != *value;
                    *value = next;
                }
            }
        }
        if let Some((_, time)) = &mut self.action_flash {
            *time = (*time - dt / 0.18).max(0.0);
            changed = true;
            if *time == 0.0 {
                self.action_flash = None;
            }
        }
        changed
    }

    pub(super) fn flash_press(&mut self, x: f32, y: f32) {
        if let Some((r, _)) = self
            .motion_targets()
            .into_iter()
            .find(|(r, _)| hit(*r, x, y))
        {
            self.action_flash = Some((r, 1.0));
        }
    }

    pub(super) fn hover_pointer(&self) -> Option<(f32, f32)> {
        pleasant_ui::idle_hover(self.pointer, self.drag.is_some())
    }

    pub(super) fn hover_amount(&self, r: Rect) -> f32 {
        if self.drag.is_some() {
            return 0.0;
        }
        self.motion
            .iter()
            .find(|m| m.rect == r)
            .map_or(0.0, |m| m.hover)
    }

    pub(super) fn surface(&self, d: &mut Draw, r: Rect) {
        d.rect(r.0, r.1, r.2, r.3, PANEL);
        d.outline(r, LINE);
    }

    pub(super) fn button(&self, d: &mut Draw, r: Rect, label: &str, on: bool, color: Color) {
        self.paint_button(d, r, label, on, color, false);
    }

    pub(super) fn button_tinted(&self, d: &mut Draw, r: Rect, label: &str, on: bool, color: Color) {
        self.paint_button(d, r, label, on, color, false);
    }

    pub(super) fn touch_latch_button(&self, d: &mut Draw, r: Rect, on: bool, color: Color) {
        self.paint_button(d, r, "Latch", on, color, true);
    }

    pub(super) fn button_surface(&self, d: &mut Draw, r: Rect, on: bool, color: Color) {
        let hover = self.hover_amount(r);
        let selected = self
            .motion
            .iter()
            .find(|m| m.rect == r)
            .map_or(on as u8 as f32, |m| m.selection);
        let flash = self
            .action_flash
            .filter(|(rect, _)| *rect == r)
            .map_or(0.0, |(_, v)| v);
        d.button_surface(r, color, selected, hover, flash);
    }

    fn paint_button(
        &self,
        d: &mut Draw,
        r: Rect,
        label: &str,
        on: bool,
        color: Color,
        touch: bool,
    ) {
        self.button_surface(d, r, on, color);
        let old = d.font;
        d.font = self.font.get();
        let label_color = if on { color } else { MUTED };
        let padding = if r.2 < 90.0 { 6.0 } else { 12.0 };
        let text = self.fit_text(d, label, r.2 - padding, 11.0);
        if touch {
            const ICON_WIDTH: f32 = 18.0;
            const GAP: f32 = 5.0;
            let text_w = self.text_width(d, &text, 11.0);
            let left = r.0 + (r.2 - ICON_WIDTH - GAP - text_w) * 0.5;
            let center_y = r.1 + r.3 * 0.5;
            d.touch_icon(left + ICON_WIDTH * 0.5, center_y, label_color);
            d.text_middle(left + ICON_WIDTH + GAP, center_y, &text, 11.0, label_color);
        } else {
            d.text_centered(
                r.0 + r.2 / 2.0,
                r.1 + r.3 / 2.0 + 4.0,
                &text,
                11.0,
                label_color,
            );
        }
        d.font = old;
    }

    pub(super) fn text_width(&self, d: &Draw, text: &str, size: f32) -> f32 {
        let mut paint = Paint::color(TEXT);
        if let Some(font) = d.font {
            paint.set_font(&[font]);
        }
        paint.set_font_size(size * d.s);
        d.c.measure_text(0.0, 0.0, text, &paint)
            .map_or(text.chars().count() as f32 * size * 0.6, |m| {
                m.width() / d.s
            })
    }

    pub(super) fn fit_text(&self, d: &Draw, text: &str, width: f32, size: f32) -> String {
        if self.text_width(d, text, size) <= width {
            return text.to_string();
        }
        let mut fitted = text.to_string();
        while !fitted.is_empty() {
            fitted.pop();
            let result = format!("{fitted}…");
            if self.text_width(d, &result, size) <= width {
                return result;
            }
        }
        "…".into()
    }

    pub(super) fn display_value(&self, c: &Control) -> String {
        if let Some(routed) = self.routed_control(c) {
            return routed.value;
        }
        if matches!(c.id, "route_min" | "route_max") {
            return crate::engine::routing::TARGETS
                [self.params.routes[self.route_slot].target.value() as usize]
                .label(c.norm);
        }
        match c.id {
            "velocity" => format!("{:.0}%", self.params.velocity.value() * 100.0),
            "humanize" => format!("{:.0}%", self.params.humanize.value() * 100.0),
            "contour" => format!("{:+.0}%", self.params.contour.value() * 100.0),
            "y" => format!("{:.0}%", self.params.y.value() * 100.0),
            "length_ms" => format!("{:.0} ms", self.params.length_ms.value()),
            "strum_ms" => format!("{:.0} ms", self.params.strum_ms.value()),
            "bend_range" => format!("{:.0} st", self.params.bend_range.value()),
            "master_range" => format!("{:.0} st", self.params.master_range.value()),
            _ => c.value.clone(),
        }
    }

    pub(super) fn parse_display_value(&self, c: &Control, text: &str) -> Option<f32> {
        let text = text.trim();
        if matches!(c.id, "route_min" | "route_max") {
            let target = &crate::engine::routing::TARGETS
                [self.params.routes[self.route_slot].target.value() as usize];
            // Enumerated labels accept their displayed names; numbers use destination units.
            if target.discrete {
                for step in target.min as i32..=target.max as i32 {
                    let norm = (step as f32 - target.min) / (target.max - target.min);
                    if target.label(norm).eq_ignore_ascii_case(text) {
                        return Some(norm);
                    }
                }
            }
            let v = text
                .trim_end_matches("ms")
                .trim_end_matches("st")
                .trim_end_matches('%')
                .trim()
                .parse::<f32>()
                .ok()?;
            let v = if matches!(
                target.id,
                "velocity" | "humanize" | "gate" | "contour" | "swing"
            ) {
                v / 100.0
            } else if matches!(
                target.id,
                "output_channel" | "bass_channel" | "upper_channel"
            ) {
                v - 1.0
            } else {
                v
            };
            return v
                .is_finite()
                .then(|| ((v - target.min) / (target.max - target.min)).clamp(0.0, 1.0));
        }
        let percent = || {
            text.trim_end_matches('%')
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|n| n.is_finite())
                .map(|n| n / 100.0)
        };
        match c.id {
            "velocity" => percent().map(|v| self.params.velocity.preview_normalized(v)),
            "humanize" => percent().map(|v| self.params.humanize.preview_normalized(v)),
            "contour" => percent().map(|v| self.params.contour.preview_normalized(v)),
            "y" => percent().map(|v| self.params.y.preview_normalized(v)),
            _ => {
                let text = match c.id {
                    "length_ms" | "strum_ms" => text.trim_end_matches("ms").trim(),
                    "bend_range" | "master_range" => text.trim_end_matches("st").trim(),
                    _ => text,
                };
                // This view's Arc<ChordboardParams> owns the parameter pointer.
                unsafe { c.ptr.string_to_normalized_value(text) }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_underline_stays_visible_for_root_and_chord_modes() {
        for (root, chord, expected) in [(false, false, 0.0), (true, false, 1.0), (false, true, 1.0)]
        {
            let mut view = ChordboardView::new(
                Arc::new(ChordboardParams {
                    mode: IntParam::new("Mode", 2, IntRange::Linear { min: 0, max: 3 }),
                    root_on_select: BoolParam::new("Root", root),
                    always_chord: BoolParam::new("Chord", chord),
                    ..ChordboardParams::default()
                }),
                Arc::new(Bridge::default()),
                Arc::new(PreviewContext),
            );
            view.tick_motion(1.0);
            let button = view
                .motion
                .iter()
                .find(|m| m.rect == ROOT_ON_SELECT)
                .unwrap();
            assert_eq!(button.selection, expected);
        }
    }
}
