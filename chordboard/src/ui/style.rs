use super::*;
use nih_plug_vizia::vizia::vg::Paint;

pub(super) struct ControlMotion {
    rect: Rect,
    hover: f32,
    selection: f32,
}

impl ChordboardView {
    fn motion_targets(&self) -> Vec<(Rect, bool)> {
        if let Some(menu) = self.menu {
            let mut targets = vec![(self.menu_close_rect(menu), false)];
            targets.extend((0..menu.items().len()).map(|i| {
                (
                    self.menu_option_rect(menu, i),
                    i == self.menu_selection(menu),
                )
            }));
            return targets;
        }
        let mut targets = vec![(OUTPUT, self.panel == Some(Panel::Output)), (MPE, false)];
        targets.extend((0..2).map(|i| {
            (
                mapping_summary_rect(i),
                self.panel == Some(Panel::Mapping) && self.mapping_axis == i,
            )
        }));
        if let Some(panel) = self.panel {
            targets.push((self.panel_close_rect(panel), false));
            targets.extend(
                self.panel_controls()
                    .iter()
                    .map(|(c, r)| (*r, c.toggle && c.norm >= 0.5)),
            );
            if panel == Panel::Mapping {
                targets.push((
                    mapping_learn_rect(self.mapping_axis),
                    self.learning() == self.mapping_axis as u8 + 1,
                ));
                targets.push((self.menu_trigger_rect(Menu::MappingKind), false));
                if self.mapping().kind == 3 {
                    targets.push((self.menu_trigger_rect(Menu::MappingChannel(true)), false));
                }
                if matches!(self.mapping().kind, 1 | 2) {
                    targets.push((
                        self.menu_trigger_rect(Menu::MappingCc(self.mapping().kind == 2)),
                        false,
                    ));
                }
                if self.mapping_axis == 1 {
                    targets.push((self.menu_trigger_rect(Menu::YTarget), false));
                }
            }
            return targets;
        }
        targets.extend([
            (APPEARANCE, false),
            (LATCH, self.params.latch.value()),
            (QWERTY, self.params.keyboard.value()),
            (ORDER, self.params.fifths.value()),
            (LEARN_OCTAVE, self.learning() == 4),
            (TEMPO_SYNC, self.params.tempo_sync.value()),
            (Menu::Key.trigger_rect(), false),
            (Menu::Scale.trigger_rect(), false),
        ]);
        if self.can_capture() || self.memory_ui.armed {
            targets.push((SAVE_MEMORY, self.memory_ui.armed));
        }
        targets.extend((0..KEY_COUNT).map(|i| (key_rect(i), false)));
        targets.extend((0..8).map(|i| (memory_rect(i), self.memory_ui.armed)));
        targets.extend((0..2).map(|i| (inversion_rect(i), false)));
        targets.extend((0..5).map(|i| (transpose_rect(i), false)));
        targets.extend((0..3).map(|i| {
            (
                mode_rect(i),
                self.params.mode.value().max(1) == i as i32 + 1,
            )
        }));
        targets.extend(
            self.base_controls()
                .iter()
                .map(|(c, r)| (*r, c.toggle && c.norm >= 0.5)),
        );
        match self.params.mode.value() {
            1 => {
                targets.push((STRUM_SYNC, self.params.strum_sync.value()));
                if self.params.strum_sync.value() {
                    targets.push((Menu::StrumRate.trigger_rect(), false));
                }
                targets.extend(
                    (0..3).map(|i| (direction_rect(i), self.params.direction.value() == i as i32)),
                );
            }
            2 => {
                targets.push((STRUM_LATCH, self.params.strum_latch.value()));
            }
            3 => {
                targets.extend(
                    (0..5).map(|i| (pattern_rect(i), self.params.arp_pattern.value() == i as i32)),
                );
                targets.extend(ARP_RATES.iter().enumerate().map(|(i, (_, beats))| {
                    (
                        rate_rect(i),
                        (self.params.rate.value() - beats).abs() < 0.0001,
                    )
                }));
                targets.extend(
                    (0..4).map(|i| (octave_rect(i), self.params.octaves.value() == i as i32 + 1)),
                );
            }
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
            let hovered = self.pointer.is_some_and(|(x, y)| hit(rect, x, y));
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

    pub(super) fn hover_amount(&self, r: Rect) -> f32 {
        self.motion
            .iter()
            .find(|m| m.rect == r)
            .map_or(0.0, |m| m.hover)
    }

    pub(super) fn surface(&self, d: &mut Draw, r: Rect) {
        d.rounded_rect(r.0, r.1 + 3.0, r.2, r.3, 12.0, alpha(BG, 0.55));
        d.rounded_rect(r.0, r.1, r.2, r.3, 12.0, PANEL);
        d.outline_rounded(r.0, r.1, r.2, r.3, 12.0, alpha(LINE, 0.7), 1.0);
    }

    pub(super) fn button(&self, d: &mut Draw, r: Rect, label: &str, on: bool, color: Color) {
        self.paint_button(d, r, label, on, color, false);
    }

    pub(super) fn trackpad_latch_button(&self, d: &mut Draw, r: Rect, on: bool, color: Color) {
        self.paint_button(d, r, "Latch", on, color, true);
    }

    fn paint_button(
        &self,
        d: &mut Draw,
        r: Rect,
        label: &str,
        on: bool,
        color: Color,
        trackpad: bool,
    ) {
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
        d.rounded_rect(r.0, r.1, r.2, r.3, 6.0, BG);
        d.rounded_rect(
            r.0,
            r.1,
            r.2,
            r.3,
            6.0,
            alpha(color, selected * 0.12 + hover * 0.05 + flash * 0.07),
        );
        d.outline_rounded(r.0, r.1, r.2, r.3, 6.0, LINE, 1.0);
        d.outline_rounded(
            r.0,
            r.1,
            r.2,
            r.3,
            6.0,
            alpha(color, (selected * 0.65 + hover * 0.35).min(1.0)),
            1.0,
        );
        let old = d.font;
        d.font = self.ui_font.get();
        let label_color = if on || color == MUTED { color } else { TEXT };
        let padding = if r.2 < 64.0 { 6.0 } else { 12.0 };
        let text = self.fit_text(d, label, r.2 - padding, 12.0);
        if trackpad {
            const ICON_WIDTH: f32 = 14.0;
            const GAP: f32 = 5.0;
            let text_w = self.text_width(d, &text, 12.0);
            let left = r.0 + (r.2 - ICON_WIDTH - GAP - text_w) * 0.5;
            let center_y = r.1 + r.3 * 0.5;
            d.trackpad_icon(left + ICON_WIDTH * 0.5, center_y, label_color);
            d.text_middle(left + ICON_WIDTH + GAP, center_y, &text, 12.0, label_color);
        } else {
            d.text_centered(
                r.0 + r.2 / 2.0,
                r.1 + r.3 / 2.0 + 4.0,
                &text,
                12.0,
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
