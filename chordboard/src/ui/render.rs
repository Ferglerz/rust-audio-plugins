use super::*;
impl ChordboardView {
    pub(super) fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
            self.ui_font.set(
                canvas
                    .add_font_mem(nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR)
                    .ok(),
            );
            self.bold_font.set(
                canvas
                    .add_font_mem(nih_plug_vizia::assets::fonts::NOTO_SANS_BOLD)
                    .ok(),
            );
        }
        let mut d = Draw::new(
            canvas,
            prefs().light(),
            bounds.w / W,
            bounds.x,
            bounds.y,
            self.ui_font.get(),
        );
        d.rounded_rect(0.0, 0.0, W, H, 0.0, BG);
        d.rounded_rect(0.0, 0.0, W, 76.0, 0.0, PANEL);
        d.line(0.0, 75.0, W, 75.0, LINE, 1.0);
        d.font = self.bold_font.get();
        d.text(32.0, 43.0, "Chordboard", 27.0, TEXT);
        d.font = self.ui_font.get();
        d.text(32.0, 62.0, "HARMONY & PERFORMANCE", 10.0, MUTED);
        d.rounded_rect(219.0, 27.0, 3.0, 28.0, 1.5, TEAL);
        d.text(234.0, 43.0, "Choose. Shape. Play.", 12.0, MUTED);
        self.button(
            &mut d,
            APPEARANCE,
            &format!(
                "Theme: {}",
                match prefs().label() {
                    "LIGHT" => "Light",
                    "DARK" => "Dark",
                    _ => "Auto",
                }
            ),
            false,
            TEAL,
        );
        let expand = self.expand_t();
        let chords = shrink_width(CHORDS_SURFACE, expand);
        let voicing = shrink_width(VOICING_SURFACE, expand);
        let performance = perf_surface_rect(expand);
        if chords.2 > 1.0 {
            self.surface(&mut d, chords);
        }
        if voicing.2 > 1.0 {
            self.surface(&mut d, voicing);
        }
        if expand == 0.0 {
            self.surface(&mut d, METERS_SURFACE);
        }
        self.surface(&mut d, performance);
        if expand < 1.0 {
            d.scissor(
                chords.0,
                chords.1,
                chords.2.max(0.0),
                CHORDS_SURFACE.3 + VOICING_SURFACE.3 + 20.0,
            );
            self.draw_chords(&mut d);
            d.reset_scissor();
        }
        if expand == 0.0 {
            self.draw_performance_header(&mut d);
        }
        let pad = self.pad();
        d.scissor(pad.0, pad.1, pad.2, pad.3.max(360.0));
        for mode in 1..4 {
            let offset = (mode as f32 - self.page_position) * pad.2;
            if offset.abs() >= pad.2 {
                continue;
            }
            d.offset_x = offset;
            if mode == 3 {
                self.draw_arp(&mut d);
            } else {
                // The complete strum renderer, its font, field and timing remain unchanged.
                d.font = self.font.get();
                self.draw_pad(&mut d, mode);
                d.font = self.ui_font.get();
                if mode == 1 && expand == 0.0 {
                    self.draw_direction(&mut d);
                }
            }
            if expand == 0.0 {
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
        }
        d.offset_x = 0.0;
        d.reset_scissor();
        if expand == 0.0 {
            self.draw_meters(&mut d);
        }
        if expand == 0.0 {
            d.font = self.ui_font.get();
            d.text(
                600.0,
                616.0,
                if self.snapshot.voices > 0 {
                    "Playing your harmony"
                } else {
                    "Ready to play"
                },
                13.0,
                TEXT,
            );
            d.text(
                600.0,
                638.0,
                match self.params.mode.value() {
                    2 => "Hold a chord, then sweep the strings · Latch plays on hover.",
                    3 => "Hold a chord to hear the pattern.",
                    _ => "Click a chord or play your MIDI keyboard.",
                },
                11.0,
                MUTED,
            );
            d.text(
                600.0,
                660.0,
                "MIDI effect · route output to an instrument",
                11.0,
                MUTED,
            );
            for axis in 0..2 {
                let mapping = crate::engine::Mapping::decode(
                    self.params.mapping(axis).load(Ordering::Relaxed),
                );
                let source = match mapping.kind {
                    1 => format!("CC {}", mapping.number),
                    2 => format!("CC {} / {}", mapping.number, mapping.number + 32),
                    3 => "Pitch bend".into(),
                    _ => "Unassigned".into(),
                };
                let channel = if mapping.kind == 0 {
                    String::new()
                } else if mapping.channel == 16 {
                    " · Any channel".into()
                } else {
                    format!(" · Ch {}", mapping.channel + 1)
                };
                let r = mapping_summary_rect(axis);
                self.button(
                    &mut d,
                    r,
                    "",
                    self.panel == Some(Panel::Mapping) && self.mapping_axis == axis,
                    TEAL,
                );
                let label = if axis == 0 {
                    "X · Strum".into()
                } else {
                    format!(
                        "Y · {}",
                        Menu::YTarget.items()[self.params.y_target.value() as usize]
                    )
                };
                d.text(r.0 + 12.0, r.1 + 17.0, &label, 12.0, TEAL);
                d.text_right(r.0 + r.2 - 12.0, r.1 + 17.0, "Edit ▾", 11.0, MUTED);
                d.text(
                    r.0 + 12.0,
                    r.1 + 35.0,
                    &format!("{source}{channel}"),
                    11.0,
                    MUTED,
                );
            }
        }
        let base_controls = self.base_controls();
        let panel_controls = self.panel_controls();
        for (c, r) in &base_controls {
            if expand > 0.0 && r.1 >= HEADER_H {
                continue;
            }
            if r.0 >= PAD.0 && r.1 >= PAD.1 && r.1 < 518.0 {
                continue;
            }
            self.draw_control(&mut d, c, *r, GOLD);
        }
        if self.panel.is_some() {
            self.draw_panel(&mut d);
            for (c, r) in &panel_controls {
                self.draw_control(&mut d, c, *r, TEAL);
            }
        }
        self.draw_memory_drag(&mut d);
        if let Some(edit) = &self.edit {
            d.font = self.font.get();
            d.value_edit(edit, GOLD);
            d.font = self.ui_font.get();
        }
        let hint = self.hover_hint(if self.panel.is_some() {
            &panel_controls
        } else {
            &base_controls
        });
        let feedback =
            self.memory_ui.flash.iter().any(|v| *v > 0.0) || self.status.starts_with("Saving");
        let footer = if feedback {
            self.status.as_str()
        } else if let Some(hint) = &hint {
            hint
        } else if !self.status.is_empty() {
            &self.status
        } else {
            "Play a chord, then add a second note to colour it.  ↑ / ↓ changes inversion."
        };
        let footer = self.fit_text(&d, footer, W - 64.0, 11.0);
        d.text(
            32.0,
            694.0,
            &footer,
            11.0,
            if feedback { GOLD } else { MUTED },
        );
        if let Some(menu) = self.menu {
            self.surface(&mut d, self.menu_bounds(menu));
            let r = self.menu_bounds(menu);
            d.text(r.0 + 12.0, r.1 + 24.0, menu.title(), 12.0, MUTED);
            self.button(&mut d, self.menu_close_rect(menu), "×", false, TEAL);
            for (i, label) in menu.items().iter().enumerate() {
                let r = self.menu_option_rect(menu, i);
                self.button(&mut d, r, label, i == self.menu_selection(menu), TEAL);
                if i == self.menu_cursor {
                    d.outline_rounded(
                        r.0 - 1.0,
                        r.1 - 1.0,
                        r.2 + 2.0,
                        r.3 + 2.0,
                        7.0,
                        alpha(TEAL, 0.6),
                        1.0,
                    );
                }
            }
        }
    }

    fn hover_hint(&self, controls: &[(Control, Rect)]) -> Option<String> {
        let (x, y) = self.pointer?;
        if self.expand_t() > 0.0 && y >= HEADER_H {
            if hit(self.expand_button(), x, y) {
                return Some("Collapse the strum field".into());
            }
            if hit(strum_latch_rect(self.expand_t()), x, y) && self.params.mode.value() == 2 {
                return Some(
                    "Hover the strum field to play without clicking · Click the field to drag as usual"
                        .into(),
                );
            }
            if self.params.mode.value() == 2 {
                let play = self.play_pad();
                for y_axis in [false, true] {
                    let (min, max) = if y_axis {
                        expression_bounds(self.params.y_min.value(), self.params.y_max.value())
                    } else {
                        strum_bounds(self.params.x_min.value(), self.params.x_max.value())
                    };
                    if [min, max].iter().any(|&value| {
                        let (ax, ay, pointer) = strum_bound_anchor_in(play, y_axis, value);
                        pleasant_ui::tag_contains(ax, ay, pointer, x, y)
                    }) {
                        return Some(if y_axis {
                            "Drag MIN / MAX vertically to set the expression range".into()
                        } else {
                            "Drag MIN / MAX to stretch the strings · Minimum span 25%".into()
                        });
                    }
                }
                if hit(play, x, y) {
                    return Some(if self.params.strum_latch.value() {
                        "Latch on · move across the strings to strum without clicking".into()
                    } else {
                        "Click and drag to strum · Latch plays the field on hover".into()
                    });
                }
            }
            return None;
        }
        if hit(self.expand_button(), x, y) && self.can_expand_strum() {
            return Some("Expand the strum field over the plugin body".into());
        }
        if let Some(slot) = (0..8).find(|&i| hit(memory_rect(i), x, y)) {
            let label = SavedChord::decode(self.params.slot(slot).load(Ordering::Relaxed))
                .map_or_else(|| "Empty memory".into(), harmony::chord_name);
            return Some(format!(
                "{label} · {} to recall · Shift-click or Shift+{} to save · Drag to move or swap",
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
                "Click, then play your keyboard’s lowest key to assign twelve chord-quality selectors".into()
            });
        }
        if hit(MPE, x, y) {
            return Some(format!(
                "Output: {} · Choose Auto, MPE or Regular MIDI",
                if self.params.mpe_enabled() {
                    "MPE"
                } else {
                    "regular MIDI"
                }
            ));
        }
        if hit(QWERTY, x, y) {
            return Some(
                "Computer keys play only when enabled and this window is focused · Click a chord key to focus".into(),
            );
        }
        if hit(strum_latch_rect(self.expand_t()), x, y) && self.params.mode.value() == 2 {
            return Some(
                "Hover the strum field to play without clicking · Click the field to drag as usual"
                    .into(),
            );
        }
        if self.params.mode.value() == 2 && self.panel.is_none() {
            let play = self.play_pad();
            for y_axis in [false, true] {
                let (min, max) = if y_axis {
                    expression_bounds(self.params.y_min.value(), self.params.y_max.value())
                } else {
                    strum_bounds(self.params.x_min.value(), self.params.x_max.value())
                };
                if [min, max].iter().any(|&value| {
                    let (ax, ay, pointer) = strum_bound_anchor_in(play, y_axis, value);
                    pleasant_ui::tag_contains(ax, ay, pointer, x, y)
                }) {
                    return Some(if y_axis {
                        "Drag MIN / MAX vertically to set the expression range".into()
                    } else {
                        "Drag MIN / MAX to stretch the strings · Minimum span 25%".into()
                    });
                }
            }
            if hit(play, x, y) {
                return Some(if self.params.strum_latch.value() {
                    "Latch on · move across the strings to strum without clicking".into()
                } else {
                    "Click and drag to strum · Latch plays the field on hover".into()
                });
            }
        }
        if controls
            .iter()
            .any(|(c, r)| c.id == "spread" && hit(*r, x, y))
        {
            return Some("Click to cycle Close / Open / Wide · Keys show the full voicing".into());
        }
        if controls
            .iter()
            .any(|(c, r)| c.id == "quality" && hit(*r, x, y))
        {
            return Some(
                "Choose a chord quality · Chord keys also select their row’s quality".into(),
            );
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
        if hit(SAVE_MEMORY, x, y) {
            return Some(
                if self.memory_ui.armed {
                    "Choose a slot to save, or click Cancel"
                } else if self.can_capture() {
                    "Save the current chord to any memory · Shift-click a slot also saves"
                } else {
                    "Play a chord before saving it"
                }
                .into(),
            );
        }
        if hit((32.0, 120.0, 284.0, 58.0), x, y) {
            return SavedChord::decode(self.snapshot.captured).map(harmony::chord_name);
        }
        if (0..2).any(|i| hit(mapping_summary_rect(i), x, y)) {
            return Some("Edit this controller’s source and range here".into());
        }
        None
    }
    fn draw_control(&self, d: &mut Draw, c: &Control, rect: Rect, color: Color) {
        let name = match c.id {
            "length_ms" => "Note length",
            "output_channel" => "MIDI channel",
            "upper" => "Upper zone",
            "members" => "Members",
            "bend_range" => "Member bend",
            "master_range" => "Master bend",
            "split_channels" => "Split bass / upper",
            "y" => "Expression",
            "strum_ms" => "Sweep",
            "contour" => "Velocity contour",
            "humanize" => "Humanize",
            "filter" => "Note filter",
            _ => &c.name,
        };
        if c.id == "quality" {
            self.button(
                d,
                rect,
                &format!(
                    "{} ▾",
                    Menu::Quality.items()[self.params.quality.value() as usize]
                ),
                self.menu == Some(Menu::Quality),
                GOLD,
            );
        } else if c.id == "spread" {
            self.draw_spread_keyboard(d, rect, &c.value);
        } else if c.toggle {
            self.button(
                d,
                rect,
                &format!("{name}: {}", if c.norm >= 0.5 { "On" } else { "Off" }),
                c.norm >= 0.5,
                color,
            );
        } else {
            let hover = self.hover_amount(rect);
            d.rounded_rect(
                rect.0,
                rect.1,
                rect.2,
                rect.3,
                6.0,
                alpha(color, hover * 0.035),
            );
            d.font = self.ui_font.get();
            d.text(rect.0 + 12.0, rect.1 + 15.0, name, 11.0, MUTED);
            d.font = self.font.get();
            let value = self.display_value(c);
            d.text_right(rect.0 + rect.2 - 12.0, rect.1 + 15.0, &value, 11.0, color);
            let y = rect.1 + 25.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA;
            let x = rect.0 + 12.0;
            let width = rect.2 - 24.0;
            d.rounded_rect(x, y, width, 3.0, 1.5, LINE);
            if c.id == "contour" {
                let centre = x + width / 2.0;
                let end = x + width * c.norm;
                d.rounded_rect(centre.min(end), y, (end - centre).abs(), 3.0, 1.5, color);
                d.line(centre, y - 3.0, centre, y + 6.0, MUTED, 1.0);
            } else {
                d.rounded_rect(x, y, width * c.norm, 3.0, 1.5, color);
            }
            d.circle(x + width * c.norm, y + 1.5, 3.0 + hover, color, true);
            d.font = self.ui_font.get();
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
        assert_eq!(key_rect(KEY_COLUMNS).0 - key_rect(0).0, 10.125);
        assert_eq!(key_rect(KEY_COLUMNS * 2).0 - key_rect(0).0, 30.375);
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
