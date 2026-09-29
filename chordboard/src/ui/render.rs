use super::*;
impl ChordboardView {
    pub(super) fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
        }
        let mut d = Draw::new(
            canvas,
            prefs().light(),
            bounds.w / W,
            bounds.x,
            bounds.y,
            self.font.get(),
        );
        d.rounded_rect(0.0, 0.0, W, H, 0.0, BG);
        d.rounded_rect(0.0, 0.0, W, 76.0, 0.0, PANEL);
        d.line(0.0, 75.0, W, 75.0, LINE, 1.0);
        d.text(32.0, 44.0, "CHORDBOARD", 23.0, GOLD);
        d.text(240.0, 43.0, "HARMONY / PERFORMANCE", TEXT_SMALL, MUTED);
        d.appearance_button(APPEARANCE, prefs().label());
        d.rounded_rect(16.0, 92.0, 564.0, 406.0, 10.0, PANEL);
        d.rounded_rect(16.0, 502.0, 564.0, 48.0, 10.0, PANEL);
        d.rounded_rect(584.0, 92.0, 520.0, 454.0, 10.0, PANEL);
        self.draw_chords(&mut d);
        self.draw_performance_header(&mut d);
        d.scissor(PAD.0, PAD.1, PAD.2, 360.0);
        for mode in 1..4 {
            let offset = (mode as f32 - self.page_position) * PAD.2;
            if offset.abs() >= PAD.2 {
                continue;
            }
            d.offset_x = offset;
            if mode == 3 {
                self.draw_arp(&mut d);
            } else {
                self.draw_pad(&mut d, mode);
            }
            for (c, r) in self.controls_for_mode(mode) {
                if r.0 >= PAD.0
                    && r.1 >= PAD.1
                    && r.1 < 518.0
                    && !(mode == 3 && matches!(c.id, "gate" | "swing"))
                {
                    self.draw_control(&mut d, &c, r, GOLD);
                }
            }
        }
        d.offset_x = 0.0;
        d.reset_scissor();
        self.draw_meters(&mut d);
        for axis in 0..2 {
            let m =
                crate::engine::Mapping::decode(self.params.mapping(axis).load(Ordering::Relaxed));
            let source = match m.kind {
                1 => format!("CC{}", m.number),
                2 => format!("CC{}/{}", m.number, m.number + 32),
                3 => "BEND".into(),
                _ => "OFF".into(),
            };
            let channel = if m.kind == 0 {
                String::new()
            } else if m.channel == 16 {
                " · ANY".into()
            } else {
                format!(" · CH{}", m.channel + 1)
            };
            let r = mapping_summary_rect(axis);
            d.button(
                r,
                &format!("{}: {source}{channel} ▾", ["X", "Y"][axis]),
                self.panel == Some(Panel::Mapping) && self.mapping_axis == axis,
                TEAL,
            );
        }
        d.rounded_rect(584.0, 590.0, 304.0, 84.0, 10.0, PANEL);
        d.text(600.0, 612.0, "PLAYBACK VOICING", TEXT_SMALL, MUTED);
        let base_controls = self.base_controls();
        let panel_controls = self.panel_controls();
        for (c, rect) in &base_controls {
            if rect.0 >= PAD.0 && rect.1 >= PAD.1 && rect.1 < 518.0 {
                continue;
            }
            self.draw_control(&mut d, c, *rect, GOLD);
        }
        if self.panel.is_some() {
            self.draw_panel(&mut d);
            for (c, rect) in &panel_controls {
                self.draw_control(&mut d, c, *rect, TEAL);
            }
        }
        self.draw_memory_drag(&mut d);
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        }
        let hint = self.hover_hint(if self.panel.is_some() {
            &panel_controls
        } else {
            &base_controls
        });
        let footer = if let Some(hint) = &hint {
            hint
        } else if self.status.is_empty() {
            "↑ / ↓ Inversion"
        } else {
            &self.status
        };
        d.text(
            32.0,
            692.0,
            &footer.chars().take(145).collect::<String>(),
            TEXT_SMALL,
            MUTED,
        );
        if let Some(menu) = self.menu {
            let first = menu.option_rect(0);
            let mut right = first.0 + first.2;
            let mut bottom = first.1 + first.3;
            for i in 1..menu.items().len() {
                let rect = menu.option_rect(i);
                right = right.max(rect.0 + rect.2);
                bottom = bottom.max(rect.1 + rect.3);
            }
            let bounds = (
                first.0 - 3.0,
                first.1 - 3.0,
                right - first.0 + 6.0,
                bottom - first.1 + 6.0,
            );
            d.rect(bounds.0, bounds.1, bounds.2, bounds.3, BG);
            d.outline(bounds, TEAL);
            for (i, label) in menu.items().iter().enumerate() {
                d.button(menu.option_rect(i), label, i == self.menu_cursor, TEAL);
            }
        }
    }

    fn hover_hint(&self, controls: &[(Control, Rect)]) -> Option<String> {
        let (x, y) = self.pointer?;
        if let Some(slot) = (0..8).find(|&i| hit(memory_rect(i), x, y)) {
            let label = SavedChord::decode(self.params.slot(slot).load(Ordering::Relaxed))
                .map_or_else(|| "Empty memory".into(), harmony::chord_name);
            return Some(format!(
                "{label} · {} to recall · Shift-click or Shift+{} to capture · Drag to move or swap",
                MEMORY_HINTS[slot], MEMORY_HINTS[slot]
            ));
        }
        if (0..5).any(|i| hit(transpose_rect(i), x, y)) {
            return Some(format!(
                "Transpose: {:+} semitones · Reset returns to 0",
                self.params.transpose.value()
            ));
        }
        if hit(LEARN_OCTAVE, x, y) {
            return Some(if self.learning() == 4 {
                "Play your keyboard's lowest key…".into()
            } else {
                "Learn twelve control keys for chord qualities".into()
            });
        }
        if hit(MPE, x, y) {
            return Some(format!(
                "Output: {} · Click to cycle AUTO / MPE / REG",
                if self.params.mpe_enabled() {
                    "MPE"
                } else {
                    "regular MIDI"
                }
            ));
        }
        if hit(QWERTY, x, y) {
            return Some(
                "Computer-keyboard playing is active only while the editor is focused".into(),
            );
        }
        if self.params.mode.value() == 2 && self.panel.is_none() {
            for y_axis in [false, true] {
                let (min, max) = if y_axis {
                    expression_bounds(self.params.y_min.value(), self.params.y_max.value())
                } else {
                    strum_bounds(self.params.x_min.value(), self.params.x_max.value())
                };
                if [min, max].iter().any(|&value| {
                    let (ax, ay, pointer) = strum_bound_anchor(y_axis, value);
                    pleasant_ui::tag_contains(ax, ay, pointer, x, y)
                }) {
                    return Some(if y_axis {
                        "Drag MIN / MAX vertically to set the expression range".into()
                    } else {
                        "Drag MIN / MAX to stretch the strings · Minimum span 25%".into()
                    });
                }
            }
        }
        if controls
            .iter()
            .any(|(c, r)| c.id == "spread" && hit(*r, x, y))
        {
            return Some("Click to cycle Close / Open / Wide · Keys show the full voicing".into());
        }
        if controls.iter().any(|(c, r)| !c.toggle && hit(*r, x, y)) {
            return Some(
                "Drag anywhere to adjust · Click value to type · Shift for fine adjustment".into(),
            );
        }
        for (i, label) in ["Pressure", "Timbre", "Bend"].iter().enumerate() {
            if hit(meter_rect(i), x, y) {
                let value = [
                    self.snapshot.pressure,
                    self.snapshot.timbre,
                    self.snapshot.bend,
                ][i];
                return Some(if i == 0 {
                    format!("{label}: {:.0}%", value * 100.0)
                } else {
                    format!("{label}: {:+.0}%", (value - 0.5) * 200.0)
                });
            }
        }
        if (0..2).any(|i| hit(mapping_summary_rect(i), x, y)) {
            return Some("Edit this controller’s source and range here".into());
        }
        None
    }
    fn draw_control(&self, d: &mut Draw, c: &Control, rect: Rect, color: Color) {
        let name = match c.id {
            "length_ms" => "Length ms",
            "quality" => "Quality",
            "output_channel" => "MIDI channel",
            "upper" => "Upper zone",
            "members" => "Members",
            "bend_range" => "Member semitones",
            "master_range" => "Master semitones",
            "split_channels" => "Split bass / upper",
            "y_target" => "Y destination",
            "strum_ms" => "Sweep ms",
            "direction" => "Direction",
            "contour" => "Velocity contour",
            _ => &c.name,
        };
        if c.id == "spread" {
            self.draw_spread_keyboard(d, rect, &c.value);
        } else if c.toggle {
            d.button(rect, &format!("{name}: {}", c.value), c.norm >= 0.5, color);
        } else {
            d.text(rect.0 + 12.0, rect.1 + 15.0, name, TEXT_SMALL, MUTED);
            d.text_right(
                rect.0 + rect.2 - 12.0,
                rect.1 + 15.0,
                &c.value,
                TEXT_SMALL,
                color,
            );
            d.rect(
                rect.0 + 12.0,
                rect.1 + 25.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
                rect.2 - 24.0,
                4.0,
                LINE,
            );
            d.rect(
                rect.0 + 12.0,
                rect.1 + 25.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA,
                (rect.2 - 24.0) * c.norm,
                4.0,
                color,
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arp_controls_fit_the_performance_surface_without_overlapping() {
        let mut rects = (0..5).map(pattern_rect).collect::<Vec<_>>();
        rects.extend((0..8).map(rate_rect));
        rects.extend((0..4).map(octave_rect));
        rects.extend(arp_controls().map(|(_, r)| r));
        rects.push(RATE_HEADER);
        for (i, r) in rects.iter().enumerate() {
            assert!(
                r.0 >= PAD.0
                    && r.1 >= PAD.1
                    && r.0 + r.2 <= PAD.0 + PAD.2
                    && r.1 + r.3 <= PAD.1 + PAD.3
            );
            for other in rects.iter().skip(i + 1) {
                let overlap = r.0 < other.0 + other.2
                    && other.0 < r.0 + r.2
                    && r.1 < other.1 + other.3
                    && other.1 < r.1 + r.3;
                assert!(!overlap, "overlapping arp targets: {r:?} and {other:?}");
            }
        }
    }
    #[test]
    fn keyboard_is_staggered_and_hit_boxes_do_not_overlap() {
        assert_eq!(key_rect(KEY_COLUMNS).0 - key_rect(0).0, 10.5);
        assert_eq!(key_rect(KEY_COLUMNS * 2).0 - key_rect(0).0, 31.5);
        for i in 0..KEY_COUNT {
            let r = key_rect(i);
            assert!(hit(r, r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
            for j in 0..KEY_COUNT {
                if i != j {
                    assert!(!hit(key_rect(j), r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
                }
            }
        }
    }
}
