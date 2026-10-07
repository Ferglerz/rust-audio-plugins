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
        )
        .with_hover(self.hover_pointer());
        d.rounded_rect(0.0, 0.0, W, H, 0.0, BG);
        d.rounded_rect(0.0, 0.0, W, 76.0, 0.0, PANEL);
        d.line(0.0, 75.0, W, 75.0, LINE, 1.0);
        d.font = self.font.get();
        d.text(32.0, 44.0, "CHORDBOARD", 24.0, GOLD);
        d.text(202.0, 44.0, "HARMONY & PERFORMANCE", 12.0, MUTED);
        d.appearance_button(APPEARANCE, prefs().label());
        d.font = self.ui_font.get();
        self.draw_tempo_header(&mut d);
        self.draw_page_tabs(&mut d);
        let expand = self.expand_t();
        let chords = shrink_width(CHORDS_SURFACE, expand);
        let modulation = MOD_SURFACE;
        let performance = perf_surface_rect(expand);
        if chords.2 > 1.0 {
            self.surface(&mut d, chords);
        }
        if modulation.2 > 1.0 {
            self.surface(&mut d, modulation);
        }
        self.surface(&mut d, performance);
        if expand < 1.0 {
            d.scissor(chords.0, chords.1, chords.2.max(0.0), CHORDS_SURFACE.3);
            d.offset_x = -self.route_progress * CHORDS_SURFACE.2;
            if self.route_progress < 1.0 {
                let route_offset = d.offset_x;
                for page in [1, 2] {
                    let offset = (page as f32 - self.page_position) * CHORDS_SURFACE.2;
                    if offset.abs() >= CHORDS_SURFACE.2 {
                        continue;
                    }
                    d.offset_x = route_offset + offset;
                    if page == 1 {
                        self.draw_chord_page_title(&mut d, "ARPEGGIATOR");
                        self.draw_arp(&mut d, if self.mode() == 3 { 3 } else { 1 });
                        for (c, r) in self.base_controls() {
                            if hit(ARP_PAD, r.0, r.1) && !matches!(c.id, "gate" | "swing") {
                                self.draw_control(&mut d, &c, r, GOLD);
                            }
                        }
                    } else {
                        self.draw_chords(&mut d);
                    }
                }
                d.offset_x = route_offset;
            }
            if self.route_progress > 0.0 {
                d.offset_x = (1.0 - self.route_progress) * CHORDS_SURFACE.2;
                self.draw_routes(&mut d);
                self.close_icon(&mut d, self.panel_close_rect(Panel::Routes), TEAL, false);
                if let Some(c) = self
                    .control("route_enabled")
                    .filter(|_| self.has_selected_route())
                {
                    self.draw_control(&mut d, &c, ROUTE_ENABLED, TEAL);
                }
            }
            d.offset_x = 0.0;
            d.reset_scissor();
        }
        self.draw_meters(&mut d);
        let pad = self.pad();
        d.scissor(pad.0, pad.1, pad.2, pad.3);
        d.font = self.font.get();
        d.disabled = !self.params.strum_enabled.value();
        self.draw_pad(&mut d, 2);
        d.font = self.ui_font.get();
        if expand == 0.0 {
            for (c, r) in self.base_controls() {
                if r.0 >= PAD.0 && r.1 >= PAD.1 && r.1 < PAD.1 + PAD.3 {
                    self.draw_control(&mut d, &c, r, GOLD);
                }
            }
        }
        d.offset_x = 0.0;
        d.reset_scissor();
        d.disabled = false;
        let base_controls = self.base_controls();
        let panel_controls = self.panel_controls();
        for (c, r) in &base_controls {
            if c.id == "root_on_select" {
                continue;
            }
            if hit(ARP_PAD, r.0, r.1) {
                continue;
            }
            if r.1 >= PIANO_SURFACE.1 {
                continue;
            }
            if expand > 0.0 && r.1 >= HEADER_H {
                continue;
            }
            if r.0 >= PAD.0 && r.1 >= PAD.1 && r.1 < PAD.1 + PAD.3 {
                continue;
            }
            self.draw_control(&mut d, c, *r, GOLD);
        }
        self.draw_strumfield_header(&mut d);
        d.scissor(
            PIANO_SURFACE.0,
            PIANO_SURFACE.1,
            PIANO_SURFACE.2,
            PIANO_SURFACE.3,
        );
        d.offset_x = -self.sequencer_ui.progress * PIANO_SURFACE.2;
        self.draw_piano(&mut d);
        d.offset_x = 0.0;
        d.reset_scissor();
        self.draw_sequencer(&mut d);
        if self.panel == Some(Panel::Mapping) {
            self.draw_panel(&mut d);
            for (c, r) in &panel_controls {
                self.draw_control(&mut d, c, *r, TEAL);
            }
        }
        self.draw_memory_drag(&mut d);
        self.draw_route_drag(&mut d);
        self.draw_route_chips(&mut d);
        if let Some(edit) = &self.edit {
            d.font = self.font.get();
            d.value_edit(edit, GOLD);
            d.font = self.ui_font.get();
        }
        let hint = if self.sequencer_open()
            && self
                .hover_pointer()
                .is_some_and(|(_, y)| y >= PIANO_SURFACE.1)
        {
            Some("Click a step to select it · Click again to toggle hit/rest · Right-click selects without toggling · One step stays selected".into())
        } else {
            self.hover_hint(if self.panel.is_some() {
                &panel_controls
            } else {
                &base_controls
            })
        };
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
            for (label, heading) in menu.group_heading_rects(r) {
                d.text(heading.0, heading.1 + 14.0, label, 10.0, GOLD);
            }
            let menu_color = match menu {
                Menu::Key | Menu::Scale => GOLD,
                Menu::KeyboardParam(id) if id.starts_with("bass") => GOLD,
                Menu::KeyboardParam("output_channel") => COLORS[1],
                _ => TEAL,
            };
            for (i, label) in menu.items().iter().enumerate() {
                let r = self.menu_option_rect(menu, i);
                let allowed = self.route_menu_allowed(menu, i);
                self.button(
                    &mut d,
                    r,
                    label,
                    allowed && i == self.menu_selection(menu),
                    if allowed { menu_color } else { MUTED },
                );
                if !allowed {
                    d.rect(r.0, r.1, r.2, r.3, alpha(PANEL, 0.65));
                }
                if i == self.menu_cursor {
                    d.outline_rounded(
                        r.0 - 1.0,
                        r.1 - 1.0,
                        r.2 + 2.0,
                        r.3 + 2.0,
                        7.0,
                        alpha(menu_color, 0.6),
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
        let (x, y) = self.hover_pointer()?;
        if self.menu.is_some() {
            return None;
        }
        if self.panel == Some(Panel::Routes) && hit(self.panel_rect(Panel::Routes), x, y) {
            if hit(ROUTE_GRAPH, x, y) {
                return Some("Drag endpoints for the output range · Drag the center node vertically for curve · Linear resets curve".into());
            }
            for (i, r) in self.editor_route_chips() {
                if hit(r, x, y) {
                    let route = self.params.routes[i].route();
                    return Some(format!(
                        "{} → {}",
                        crate::engine::routing::SOURCES[route.source as usize],
                        crate::engine::routing::TARGETS[route.target as usize].name
                    ));
                }
            }
            return None;
        }
        if let Some(id) = self
            .route_targets()
            .into_iter()
            .find(|(_, r)| hit(*r, x, y))
            .map(|(i, _)| crate::engine::routing::TARGETS[i].id)
        {
            if let Some(hint) = self.modulation_hint(id) {
                return Some(hint);
            }
        }
        if let Some(hint) = self.piano_hint(x, y) {
            return Some(hint);
        }
        if hit(strum_bypass_rect(self.expand_t()), x, y) {
            return Some("Enable or bypass the Strumfield".into());
        }
        if hit(HEADER_PLAY_TAB, x, y) {
            return Some("Keyboard and chords".into());
        }
        if hit(HEADER_SEQ_TAB, x, y) {
            return Some("Sequencer replaces the keyboard · Chords area shows the arpeggiator for the selected step".into());
        }
        if hit(HEADER_SEQ_BYPASS, x, y) {
            return Some("Enable or bypass sequencer playback".into());
        }
        if self.expand_t() > 0.0 && y >= HEADER_H {
            if self.can_expand_strum() && hit(self.expand_button(), x, y) {
                return Some("Collapse the strum field".into());
            }
            if hit(strum_latch_rect(self.expand_t()), x, y) && self.manual_playing() {
                return Some(
                    "Hover the strum field to play without clicking · Click the field to drag as usual"
                        .into(),
                );
            }
            if self.manual_playing() {
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
        if (0..KEY_COUNT)
            .any(|i| self.snapshot.ignored & (1 << i) != 0 && hit(self.keyboard_key_rect(i), x, y))
        {
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
        if (0..5).any(|i| hit(transpose_control_rect(self.params.mpe_enabled(), i), x, y)) {
            return Some(format!(
                "Transpose: {:+} semitones · Reset returns to 0",
                self.params.transpose.value()
            ));
        }
        if hit(self.control_octave_bounds(), x, y) && piano_note_at(x, y).is_none() {
            return Some(if self.learning() == 4 {
                "Play C to set the control octave…".into()
            } else {
                "Learn C · C major, Db b9, D sus2, Eb minor, E major, F sus4, F# dim, G power, Ab aug, A 6, Bb 7, B maj7".into()
            });
        }
        if hit(output_protocol_rect(self.params.mpe_enabled(), 0), x, y) {
            return Some(
                if !self.params.mpe_enabled() && self.params.detected_mpe.load(Ordering::Relaxed) {
                    "MPE input detected · Click MPE off to enable MPE output".into()
                } else {
                    "Click to turn MPE output on or off".into()
                },
            );
        }

        if hit(LATCH, x, y) {
            return Some(
                "Click or tap Shift: toggle latch · Hold Shift: invert latch until release".into(),
            );
        }
        if hit(strum_latch_rect(self.expand_t()), x, y) && self.manual_playing() {
            return Some(
                "Hover the strum field to play without clicking · Click the field to drag as usual"
                    .into(),
            );
        }
        if self.manual_playing() && self.panel.is_none() {
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
                "{} is routed · Teal shows live; the original marker shows base · Editing changes base",
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
        if hit(strum_selection_rect(self.expand_t()), x, y) {
            return Some(
                "Play the root immediately on chord selection, including Manual Strum".into(),
            );
        }
        for i in 0..crate::engine::routing::SOURCE_COUNT {
            if hit(meter_rect(i), x, y) {
                return Some(format!(
                    "{} input: {:.0}% · Drag to a control to link · Click to edit routes; click again to close",
                    crate::engine::routing::SOURCES[i + 1],
                    self.snapshot.sources[i] * 100.0
                ));
            }
        }
        if hit(chord_readout_rect(), x, y) {
            return SavedChord::decode(self.snapshot.captured).map(harmony::chord_name);
        }
        if hit(transpose_control_rect(self.params.mpe_enabled(), 2), x, y) {
            return Some("Transpose in semitones · Click to reset to zero".into());
        }
        if (0..2).any(|i| hit(mapping_summary_rect(i), x, y)) {
            return Some("Edit this controller’s source and range here".into());
        }
        None
    }
    pub(super) fn draw_modulation_badge(&self, d: &mut Draw, id: &str, r: Rect) {
        if let Some(slot) = self.modulation_route(id) {
            let route = self.params.routes[slot].route();
            let label = format!(
                "↗ {} controls",
                crate::engine::routing::SOURCE_SHORT[route.source as usize - 1]
            );
            d.text(r.0, r.1 + 12.0, &label, 9.0, TEAL);
        }
    }
    pub(super) fn draw_control(&self, d: &mut Draw, c: &Control, rect: Rect, color: Color) {
        if c.id == "root_on_select" {
            let mode = self.selection_mode();
            self.button(
                d,
                rect,
                ["Strum Only", "Root on Select", "Chord on Select"][mode],
                mode > 0,
                if mode == 2 { COLORS[1] } else { color },
            );
            return;
        }
        let base = c;
        let routed = self.routed_control(c);
        let c = routed.as_ref().unwrap_or(c);
        let is_length_held =
            c.id == "length_ms" && self.playback_mode() == 1 && self.params.strum_hold.value();
        let color = if is_length_held {
            MUTED
        } else if routed.is_some() {
            TEAL
        } else {
            color
        };
        if c.id == "strum_ms" {
            // Free rate uses the same rail in Once and Loop.
            let x = rect.0 + 70.0;
            let y = rect.1 + rect.3 * 0.5 - 1.5;
            let width = rect.2 - 162.0;
            d.text(rect.0 + 12.0, y + 4.0, "Rate", TEXT_SMALL, MUTED);
            d.rounded_rect(x, y, width, 3.0, 1.5, LINE);
            d.rounded_rect(x, y, width * c.norm, 3.0, 1.5, color);
            d.circle(
                x + width * c.norm,
                y + 1.5,
                3.0 + self.hover_amount(rect),
                color,
                true,
            );
            if routed.is_some() {
                d.line(
                    x + width * base.norm,
                    y - 4.0,
                    x + width * base.norm,
                    y + 7.0,
                    GOLD,
                    2.0,
                );
            }
            d.text_right(
                rect.0 + rect.2 - 12.0,
                y + 4.0,
                &self.display_value(c),
                TEXT_SMALL,
                color,
            );
            return;
        }
        let name = match c.id {
            "velocity" => "Velocity",
            "length_ms" => "Length",
            "strings_played" => "Played",
            "output_channel" => "MIDI channel",
            "upper" => "Upper zone",
            "members" => "Members",
            "bend_range" => "Member bend",
            "master_range" => "Master bend",
            "y" => "Expression",
            "strum_ms" => "Rate",
            "contour" => "Velocity contour",
            "humanize" => "Humanize",
            "filter" => "Note filter",
            _ => &c.name,
        };
        if c.id == "voice_leading" {
            self.button(
                d,
                rect,
                ["Nearest", "Furthest dominant", "Off"][self
                    .routed_plain("voice_leading")
                    .unwrap_or(self.params.voice_leading.value() as f32)
                    as usize],
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
                base.norm >= 0.5,
                color,
            );
        } else {
            let hover = if is_length_held {
                0.0
            } else {
                self.hover_amount(rect)
            };
            d.rounded_rect(
                rect.0,
                rect.1,
                rect.2,
                rect.3,
                6.0,
                alpha(color, hover * 0.035),
            );
            d.font = self.ui_font.get();
            if c.id != "tempo" {
                d.text(rect.0 + 12.0, rect.1 + 15.0, name, 11.0, MUTED);
            }
            d.font = self.font.get();
            let value = if is_length_held {
                "Held".to_string()
            } else {
                self.display_value(c)
            };
            d.text_right(
                rect.0 + rect.2 - 12.0,
                rect.1 + if c.id == "tempo" { 16.0 } else { 15.0 },
                &value,
                11.0,
                color,
            );
            let y = rect.1
                + if c.id == "tempo" {
                    11.0
                } else {
                    25.0 + pleasant_ui::value_edit::SLIDER_SPACING_EXTRA
                };
            let x = rect.0 + 12.0;
            let width = rect.2 - if c.id == "tempo" { 92.0 } else { 24.0 };
            d.rounded_rect(x, y, width, 3.0, 1.5, LINE);
            if is_length_held {
                d.rounded_rect(x, y, width, 3.0, 1.5, alpha(LINE, 0.4));
            } else {
                d.rounded_rect(x, y, width * c.norm, 3.0, 1.5, color);
            }
            if routed.is_some() {
                // The saved position remains visible while the teal marker moves.
                let base_x = x + width * base.norm;
                d.line(base_x, y - 4.0, base_x, y + 7.0, GOLD, 2.0);
            }
            if !is_length_held {
                d.circle(x + width * c.norm, y + 1.5, 3.0 + hover, color, true);
            }
            d.font = self.ui_font.get();
        }
        if rect.3 >= 42.0 {
            self.draw_modulation_badge(
                d,
                c.id,
                (rect.0 + 12.0, rect.1 + rect.3 - 14.0, rect.2 - 24.0, 14.0),
            );
        }
        if routed.is_some() && (c.toggle || matches!(c.id, "quality" | "voice_leading" | "spread"))
        {
            self.live_highlight(d, rect);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arp_controls_fit_the_chords_page_without_overlapping() {
        let mut rects = (0..7).map(pattern_rect).collect::<Vec<_>>();
        rects.extend((0..8).map(rate_rect));
        rects.extend((0..4).map(octave_rect));
        rects.extend(arp_controls().map(|(_, r)| r));
        rects.push(RATE_HEADER);
        rects.push(RATE_SYNC);
        rects.push(ARP_STRINGS);
        rects.extend(output_controls(1).into_iter().map(|(_, r)| r));
        for (i, r) in rects.iter().enumerate() {
            assert!(
                r.0 >= ARP_PAD.0
                    && r.1 >= ARP_PAD.1
                    && r.0 + r.2 <= ARP_PAD.0 + ARP_PAD.2
                    && r.1 + r.3 <= ARP_PAD.1 + ARP_PAD.3
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
    fn performance_rows_fill_width_and_share_direction_sizes() {
        let first = pattern_rect(0);
        let last = pattern_rect(6);
        assert_eq!(first.0, ARP_PAD.0 + 8.0);
        assert!((last.0 + last.2 - (ARP_PAD.0 + ARP_MAIN_WIDTH - 8.0)).abs() < 0.001);
        let [(_, humanize), (_, gate), (_, swing)] = arp_controls();
        assert!(humanize.1 + humanize.3 < first.1);
        assert_eq!(gate.1, swing.1);
        assert!(gate.0 > swing.0 + swing.2);
        assert!((gate.2 + swing.2 + 8.0 - (ARP_MAIN_WIDTH - 16.0)).abs() < 0.001);
        assert_eq!(swing.2, gate.2);
        assert_eq!(ROOT_ON_SELECT.1, STRUM_LATCH.1);
        assert_eq!(ROOT_ON_SELECT.0 + ROOT_ON_SELECT.2 + 8.0, STRUM_LATCH.0);
    }
    #[test]
    fn keyboard_is_staggered_and_hit_boxes_do_not_overlap() {
        assert_eq!(key_rect(KEY_COLUMNS).0 - key_rect(0).0, KEY_PITCH * 0.25);
        assert_eq!(
            key_rect(KEY_COLUMNS * 2).0 - key_rect(0).0,
            KEY_PITCH * 0.75
        );
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
