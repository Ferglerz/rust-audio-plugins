use super::*;
impl ChordboardView {
    pub(super) fn render(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let bounds = cx.bounds();
        if bounds.w <= 0.0 || bounds.h <= 0.0 {
            return;
        }
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
            self.ui_font.set(self.font.get());
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
        d.font = self.font.get();
        d.text(32.0, 44.0, "CHORDBOARD", 24.0, GOLD);
        d.text(202.0, 44.0, "HARMONY & PERFORMANCE", 12.0, MUTED);
        d.appearance_button(APPEARANCE, prefs().label());
        d.font = self.ui_font.get();
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
        self.draw_piano(&mut d);
        if self.panel.is_some() {
            self.draw_panel(&mut d);
            for (c, r) in &panel_controls {
                self.draw_control(&mut d, c, *r, TEAL);
            }
        }
        self.draw_memory_drag(&mut d);
        self.draw_route_drag(&mut d);
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
            ""
        };
        let footer = self.fit_text(&d, footer, W - 64.0, 11.0);
        d.text(
            32.0,
            H - 10.0,
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
        if let Some(hint) = self.route_drag_hint() {
            return Some(hint);
        }
        let (x, y) = self.pointer?;
        if self.menu.is_some() {
            return None;
        }
        if self.panel == Some(Panel::Routes) && hit(self.panel_rect(Panel::Routes), x, y) {
            if hit(ROUTE_GRAPH, x, y) {
                return Some("Drag endpoints for the output range · Drag the center node vertically for curve · Linear resets curve".into());
            }
            for i in 0..crate::engine::routing::ROUTE_COUNT {
                if hit(route_slot_rect(i), x, y) {
                    let r = self.params.routes[i].route();
                    return Some(if r.source == 0 {
                        format!("Route {} · Unassigned", i + 1)
                    } else {
                        format!(
                            "Route {} · {} → {}",
                            i + 1,
                            crate::engine::routing::SOURCES[r.source as usize],
                            crate::engine::routing::TARGETS[r.target as usize].name
                        )
                    });
                }
            }
            return None;
        }
        if let Some(hint) = self.piano_hint(x, y) {
            return Some(hint);
        }
        if self.expand_t() > 0.0 && y >= HEADER_H {
            if self.can_expand_strum() && hit(self.expand_button(), x, y) {
                return Some("Collapse the strum field".into());
            }
            if hit(strum_latch_rect(self.expand_t()), x, y) && self.mode() == 2 {
                return Some(
                    "Hover the strum field to play without clicking · Click the field to drag as usual"
                        .into(),
                );
            }
            if self.mode() == 2 {
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
        if (0..KEY_COUNT).any(|i| self.snapshot.ignored & (1 << i) != 0 && hit(key_rect(i), x, y)) {
            return Some("Extra key pending · The last held key becomes the next root".into());
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
                "Play C to set the control octave…".into()
            } else {
                "Learn C · C major, Db b9, D sus2, Eb minor, E major, F sus4, F# dim, G power, Ab aug, A 6, Bb 7, B maj7".into()
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
        if hit(LATCH, x, y) {
            return Some(
                "Keep the chord and alteration after release · Re-press the root to reset".into(),
            );
        }
        if hit(QWERTY, x, y) {
            return Some(
                "Computer keys play only when enabled and this window is focused · Click a chord key to focus".into(),
            );
        }
        if hit(strum_latch_rect(self.expand_t()), x, y) && self.mode() == 2 {
            return Some(
                "Hover the strum field to play without clicking · Click the field to drag as usual"
                    .into(),
            );
        }
        if self.mode() == 2 && self.panel.is_none() {
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
        if let Some((c, _)) = controls
            .iter()
            .find(|(c, r)| hit(*r, x, y) && self.routed_control(c).is_some())
        {
            return Some(format!(
                "{} is routed · Teal shows its live value; editing changes the saved base value",
                c.name
            ));
        }
        if controls
            .iter()
            .any(|(c, r)| c.id == "voice_leading" && hit(*r, x, y))
        {
            return Some("Click: Nearest resolution / Furthest dominant resolution / Off · Furthest uses larger motion on V–I; otherwise nearest".into());
        }
        if controls
            .iter()
            .any(|(c, r)| c.id == "spread" && hit(*r, x, y))
        {
            return Some("Click to cycle Close / Open / Wide · The full keyboard shows the voicing and voice-leading paths".into());
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
        if hit(ROOT_ON_SELECT, x, y) {
            return Some(
                "Play the root immediately on chord selection, including Manual Strum".into(),
            );
        }
        if hit(ROUTES_BUTTON, x, y) {
            return Some(
                "Link controllers to parameters · Each route has its own range and curve".into(),
            );
        }
        for i in 0..crate::engine::routing::SOURCE_COUNT {
            if hit(meter_rect(i), x, y) {
                return Some(format!(
                    "{} input: {:.0}% · Drag to a control to link · Routes edits range and curve",
                    crate::engine::routing::SOURCES[i + 1],
                    self.snapshot.sources[i] * 100.0
                ));
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
        let routed = self.routed_control(c);
        let c = routed.as_ref().unwrap_or(c);
        let color = if routed.is_some() { TEAL } else { color };
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
                    Menu::Quality.items()[self
                        .routed_plain("quality")
                        .unwrap_or(self.params.quality.value() as f32)
                        as usize]
                ),
                self.menu == Some(Menu::Quality),
                GOLD,
            );
        } else if c.id == "voice_leading" {
            self.button(
                d,
                rect,
                ["Nearest", "Furthest dominant", "Off"][self.params.voice_leading.value() as usize],
                self.params.voice_leading.value() != 2,
                TEAL,
            );
        } else if c.id == "spread" {
            self.button(d, rect, &c.value, false, GOLD);
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
