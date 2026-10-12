use super::*;

impl ChordboardView {
    pub(super) fn meter(&self) -> (u8, u8) {
        if self.params.tempo_sync.value() && self.snapshot.time_sig_host {
            (self.snapshot.time_sig_num, self.snapshot.time_sig_den)
        } else {
            (self.params.meter_num.value() as u8, 1 << self.params.meter_den.value())
        }
    }
    pub(super) fn draw_tempo_header(&self, d: &mut Draw) {
        let synced = self.params.tempo_sync.value();
        let (num, den) = self.meter();
        let host_meter = synced && self.snapshot.time_sig_host;
        if host_meter {
            d.text(METER_NUM.0 + 8.0, 43.5, &format!("{num}/{den}"), 13.0, TEXT);
        } else {
            self.button(d, METER_NUM, &num.to_string(), false, GOLD);
            d.text(METER_NUM.0 + 49.0, 43.5, "/", 13.0, MUTED);
            self.button(d, METER_DEN, &den.to_string(), false, GOLD);
        }
        self.icon_button(
            d,
            TEMPO_SYNC,
            if synced { "Host tempo" } else { "Manual tempo" },
            true,
            if synced { TEAL } else { GOLD },
            if synced { style::Icon::Host } else { style::Icon::Manual },
        );
        if synced {
            d.text(
                TEMPO_CONTROL.0 + 16.0,
                TEMPO_CONTROL.1 + TEMPO_CONTROL.3 * 0.5 + 13.0 * 0.35,
                &format!("{:.1} bpm", self.snapshot.tempo),
                13.0,
                TEXT,
            );
        }
    }

    pub(super) fn draw_page_tabs(&self, d: &mut Draw) {
        let open = self.sequencer_open();
        self.button(d, HEADER_PLAY_TAB, "Play", !open, TEAL);
        self.button(d, HEADER_SEQ_TAB, "", open, TEAL);
        d.text(
            HEADER_SEQ_BYPASS.0 + HEADER_SEQ_BYPASS.2 + 10.0,
            HEADER_SEQ_TAB.1 + HEADER_SEQ_TAB.3 * 0.5 + 13.0 * 0.35,
            "Sequencer", 13.0, if open { TEAL } else { TEXT },
        );
        d.power_icon(
            HEADER_SEQ_BYPASS.0 + HEADER_SEQ_BYPASS.2 * 0.5,
            HEADER_SEQ_BYPASS.1 + HEADER_SEQ_BYPASS.3 * 0.5,
            if self.params.seq_enabled.value() { TEAL } else { MUTED },
        );
    }

    pub(super) fn draw_strumfield_header(&self, d: &mut Draw) {
        let surface = perf_surface_rect(self.expand_t());
        let bypass = strum_bypass_rect(self.expand_t());
        d.bypass_button(
            bypass,
            !self.params.strum_enabled.value(),
            TEAL,
            self.hover_amount(bypass) > 0.01,
            0.0,
        );
        let disabled = d.disabled;
        d.disabled = !self.params.strum_enabled.value();
        d.font = self.font.get();
        d.text(
            surface.0 + 48.0,
            module_title_y(PERF_SURFACE.1, MODULE_TITLE_SIZE),
            "STRUMFIELD",
            MODULE_TITLE_SIZE,
            TEXT,
        );
        d.font = self.ui_font.get();
        let expand = self.expand_t();
        let first = octave_button_rect(expand, 0);
        d.text_right(
            first.0 - 8.0,
            first.1 + first.3 * 0.5 + TEXT_SMALL * 0.35,
            "Octaves",
            TEXT_SMALL,
            MUTED,
        );
        let octaves = self.params.octaves.value();
        for i in 0..4 {
            let r = octave_button_rect(expand, i);
            self.button(d, r, &(i + 1).to_string(), octaves == i as i32 + 1, GOLD);
            self.live_choice(d, "octaves", i as f32 + 1.0, r);
        }
        self.touch_latch_button(
            d,
            strum_latch_rect(self.expand_t()),
            self.params.strum_latch.value(),
            TEAL,
        );
        let r = self.expand_button();
        d.graph_zoom_button(
            r,
            self.expand_target < 0.5,
            self.hover_amount(r) > 0.0,
            true,
        );
        d.disabled = disabled;
    }

    pub(super) fn draw_chord_page_title(&self, d: &mut Draw, title: &str) {
        let font = d.font;
        d.font = self.font.get();
        d.text(
            CHORDS_SURFACE.0 + 16.0,
            module_title_y(CHORDS_SURFACE.1, MODULE_TITLE_SIZE),
            title,
            MODULE_TITLE_SIZE,
            TEXT,
        );
        d.font = font;
    }

    pub(super) fn draw_contour_button(
        &self,
        d: &mut Draw,
        r: Rect,
        label: &str,
        contour: &[u8],
        selected: bool,
        color: Color,
        param_id: &'static str,
        choice_val: f32,
    ) {
        let text_color = if selected { color } else { MUTED };
        self.button_surface(d, r, selected, color);
        self.live_choice(d, param_id, choice_val, r);
        let n_points = contour.len();
        let points: Vec<_> = contour
            .iter()
            .enumerate()
            .map(|(j, n)| {
                (
                    r.0 + 6.0 + j as f32 * (r.2 - 12.0) / (n_points - 1).max(1) as f32,
                    r.1 + r.3 * 0.53 - *n as f32 * (r.3 * 0.075),
                )
            })
            .collect();
        d.poly(&points, text_color, 1.2);
        for &(x, y) in &points {
            d.circle(x, y, 1.6, text_color, true);
        }
        d.text_centered(
            r.0 + r.2 / 2.0,
            r.1 + r.3 - 10.0,
            label,
            TEXT_SMALL,
            text_color,
        );
    }

    pub(super) fn string_count(&self) -> usize {
        let octaves = self
            .routed_plain("octaves")
            .unwrap_or(self.params.octaves.value() as f32);
        harmony::string_count(self.snapshot.notes.len, octaves as u8)
    }

    // Shared by Play and Sequencer pages, manual and auto fields, and the expanded pad.
    fn draw_strings(&self, d: &mut Draw, field: Rect, span: (f32, f32), reverse: bool) {
        if self.snapshot.notes.len == 0 {
            d.text_centered(
                field.0 + field.2 * 0.5,
                field.1 + field.3 * 0.5 + 4.0,
                "Play a chord to see its strings",
                TEXT_SMALL,
                MUTED,
            );
            return;
        }
        let (min, max) = span;
        let n = self.string_count();
        let (top, bottom) = (field.1, field.1 + field.3);
        for i in 0..n {
            let string = if reverse { n - 1 - i } else { i };
            let Some(note) = self.snapshot.notes.string(string) else {
                continue;
            };
            let x = field.0 + (min + i as f32 * (max - min) / (n - 1).max(1) as f32) * field.2;
            let pulse = self.note_anim[note as usize].max(self.string_anim[string]);
            let sounding = self.snapshot.sounding_notes[note as usize];
            let color = if sounding { GOLD } else { TEAL };
            if sounding || pulse > 0.01 {
                d.rounded_rect(
                    x - 6.0,
                    top,
                    12.0,
                    bottom - top,
                    3.0,
                    alpha(color, 0.08 + pulse * 0.12),
                );
            }
            let displacement = (pulse * 24.0).sin() * pulse * 3.0;
            d.poly(
                &[
                    (x, top),
                    (x + displacement, (top + bottom) * 0.5),
                    (x, bottom),
                ],
                alpha(color, if sounding { 0.95 } else { 0.25 + pulse * 0.65 }),
                1.0 + pulse,
            );
            d.text_centered(
                x,
                bottom + 14.0,
                &music::label(&self.note_name(note)),
                TEXT_SMALL,
                if sounding || pulse > 0.1 {
                    color
                } else {
                    MUTED
                },
            );
        }
    }

    pub(super) fn draw_arp(&self, d: &mut Draw, mode: i32) {
        let looping = mode == 3;
        let pad = ARP_PAD;
        d.rounded_rect(pad.0, pad.1, pad.2, pad.3, 8.0, BG);
        d.font = self.ui_font.get();
        for loop_choice in [false, true] {
            self.icon_button(
                d,
                repeat_rect(loop_choice),
                if loop_choice { "Loop" } else { "Once" },
                looping == loop_choice,
                GOLD,
                if loop_choice { style::Icon::Loop } else { style::Icon::Once },
            );
        }
        // Glyphs illustrate each ordering rule; they are not a playback position.
        let contours = [
            [0, 1, 2, 3, 4],
            [4, 3, 2, 1, 0],
            [0, 2, 4, 2, 0],
            [0, 2, 1, 3, 2],
            [4, 2, 3, 1, 2],
            [0, 3, 1, 2, 4],
            [2, 4, 0, 3, 1],
        ];
        let labels = [
            "Up",
            "Down",
            "Up / Down",
            "Up 3rds",
            "Dn 3rds",
            "As played",
            "Random",
        ];
        for i in 0..7 {
            let r = pattern_rect(i);
            let selected = self.params.arp_pattern.value() == i as i32;
            self.draw_contour_button(
                d,
                r,
                labels[i],
                &contours[i],
                selected,
                GOLD,
                "arp_pattern",
                i as f32,
            );
        }

    }
    pub(super) fn draw_pad(&self, d: &mut Draw, mode: i32) {
        let pad = self.pad();
        d.rounded_rect(pad.0, pad.1, pad.2, pad.3, 8.0, BG);
        let manual = mode == 2;
        let field = self.field(manual);
        let (min, max) = if manual {
            strum_bounds(self.params.x_min.value(), self.params.x_max.value())
        } else {
            (0.0, 1.0)
        };
        let (y_min, y_max) =
            expression_bounds(self.params.y_min.value(), self.params.y_max.value());
        let expression_position = |value: f32| {
            y_min
                + if self.params.y_reverse.value() {
                    1.0 - value
                } else {
                    value
                } * (y_max - y_min)
        };
        self.draw_strings(
            d,
            field,
            (min, max),
            manual && self.params.x_reverse.value(),
        );
        if !manual {
            self.button(
                d,
                strum_sync_rect(self.expand_t()),
                "SWEEP SYNC",
                self.params.strum_sync.value(),
                TEAL,
            );
            self.button(
                d,
                strum_hold_rect(self.expand_t()),
                if self.routed_plain("strum_hold").is_some() { "" } else { "HOLD" },
                self.params.strum_hold.value(),
                TEAL,
            );
            if let Some(live) = self.routed_plain("strum_sync") {
                let r = strum_sync_rect(self.expand_t());
                self.live_highlight(d, r);
                d.text(
                    r.0 + r.2 + 6.0,
                    r.1 + r.3 * 0.5 + 4.0,
                    if live >= 0.5 { "ON" } else { "OFF" },
                    11.0,
                    TEAL,
                );
            }
            if let Some(live) = self.routed_plain("strum_hold") {
                let r = strum_hold_rect(self.expand_t());
                self.live_highlight(d, r);
                d.text_centered(r.0 + r.2 * 0.5, r.1 + 14.0, "HOLD", 11.0, MUTED);
                d.text_right(
                    r.0 + r.2 - 4.0,
                    r.1 + r.3 - 3.0,
                    if live >= 0.5 { "ON" } else { "OFF" },
                    11.0,
                    TEAL,
                );
            }
            if self
                .routed_plain("strum_sync")
                .map_or(self.params.strum_sync.value(), |v| v >= 0.5)
            {
                let beats = self
                    .routed_plain("strum_beats")
                    .unwrap_or(self.params.strum_beats.value());
                let label = ARP_RATES
                    .iter()
                    .find(|(_, beats)| {
                        (*beats
                            - self
                                .routed_plain("strum_beats")
                                .unwrap_or(self.params.strum_beats.value()))
                        .abs()
                            < 0.0001
                    })
                    .map_or_else(
                        || format!("{beats:.3} beats"),
                        |(label, _)| label.to_string(),
                    );
                self.button(
                    d,
                    strum_rate_rect(self.expand_t()),
                    &format!("Sweep: {label} ▾"),
                    self.menu == Some(Menu::StrumRate),
                    GOLD,
                );
                if self.routed_plain("strum_beats").is_some() {
                    self.live_highlight(d, strum_rate_rect(self.expand_t()));
                }
            }
        }
        if !manual {
            return;
        }
        {
            for &(x, y, time) in &self.trail {
                let age = time.elapsed().as_secs_f32();
                let strength = (1.0 - age / 0.35).max(0.0);
                d.circle(
                    field.0
                        + (min
                            + if self.params.x_reverse.value() {
                                1.0 - x
                            } else {
                                x
                            } * (max - min))
                            * field.2,
                    field.1 + (1.0 - expression_position(y)) * field.3,
                    3.0 + strength * 3.0,
                    alpha(TEAL, strength * 0.17),
                    true,
                );
            }
        }
        let y_labels = expression_bound_labels(field, y_min, y_max);
        for (y_axis, low, high) in [(false, min, max), (true, y_min, y_max)] {
            for (maximum, value) in [(false, low), (true, high)] {
                let (ax, ay, pointer) = strum_bound_anchor_in(field, y_axis, value);
                if y_axis {
                    d.line(field.0, ay, field.0 + field.2, ay, alpha(GOLD, 0.35), 1.0);
                } else {
                    d.line(ax, field.1, ax, field.1 + field.3, alpha(TEAL, 0.35), 1.0);
                }
                let color = if y_axis { GOLD } else { TEAL };
                d.tag_handle(ax, ay, pointer, color);
                let dragging = matches!(self.drag, Some(Drag::StrumBound(axis, end)) if axis == y_axis && end == maximum);
                let label = if dragging {
                    format!("{value:.2}")
                } else if maximum {
                    "MAX".into()
                } else {
                    "MIN".into()
                };
                if y_axis {
                    let r = y_labels[maximum as usize];
                    let center_y = r.1 + r.3 * 0.5;

                    if dragging {
                        d.rounded_rect(r.0, r.1, r.2, r.3, 3.0, alpha(color, 0.16));
                        d.circle(ax, ay, 4.0, color, false);
                    }
                    d.text_centered(r.0 + r.2 * 0.5, center_y + TEXT_SMALL * 0.35, &label, TEXT_SMALL, color);
                } else if maximum {
                    d.text_right(ax - 14.0, ay - 17.0, &label, TEXT_SMALL, color);
                } else {
                    d.text(ax + 14.0, ay - 17.0, &label, TEXT_SMALL, color);
                }
            }
        }
        let position = if self.params.x_reverse.value() {
            1.0 - self.snapshot.x
        } else {
            self.snapshot.x
        };
        let x = field.0 + (min + position * (max - min)) * field.2;
        let y = field.1 + (1.0 - expression_position(self.snapshot.y)) * field.3;
        d.circle(
            x.clamp(field.0 + 8.0, field.0 + field.2 - 8.0),
            y.clamp(field.1 + 8.0, field.1 + field.3 - 8.0),
            7.0,
            TEAL,
            false,
        );
    }
    pub(super) fn spread_preview_notes(&self) -> harmony::Notes {
        if self.snapshot.full_notes.len > 0 {
            self.snapshot.full_notes
        } else {
            let chord = SavedChord::decode(self.snapshot.captured).unwrap_or(SavedChord {
                root: (self.params.keyboard_octave.value() + self.params.key.value()).clamp(0, 127) as u8,
                quality: self.params.quality.value() as u8,
                second: None,
                additions: 0,
                transpose: self.params.transpose.value() as i8,
                inversion: self.params.inversion.value() as u8,
                spread: self.params.spread.value() as u8,
            });
            harmony::voice_recipe(chord, self.params.spread.value() as u8)
        }
    }

    pub(super) fn draw_meters(&self, d: &mut Draw) {
        use crate::engine::routing::SOURCE_SHORT;
        d.font = self.font.get();
        d.text(
            MOD_SURFACE.0 + 10.0,
            module_title_y(MOD_SURFACE.1, MODULE_TITLE_SIZE),
            "MODULATORS",
            MODULE_TITLE_SIZE,
            TEXT,
        );
        style::Icon::Routing.draw(d, MOD_SURFACE.0 + 18.0, MOD_SURFACE.1 + 46.0, MUTED);
        d.text(
            MOD_SURFACE.0 + 31.0,
            MOD_SURFACE.1 + 50.0,
            "Drag to route",
            11.0,
            MUTED,
        );
        for (i, label) in SOURCE_SHORT.iter().enumerate() {
            let r = meter_rect(i);
            let value = self.snapshot.sources[i];
            let accent = if i >= 9 { GOLD } else { TEAL };
            let hover = self.hover_amount(r);
            let r = (r.0, r.1 - hover * 2.0, r.2, r.3);
            let dragging = matches!(self.drag, Some(Drag::Route(drag)) if drag.source == i);
            let route_count = self
                .params
                .routes
                .iter()
                .filter(|p| p.source.value() as usize == i + 1)
                .count();
            let linked = route_count > 0;
            let selected = self.panel == Some(Panel::Routes) && self.selected_modulator == Some(i);
            let emphasis = if dragging {
                1.0
            } else if selected {
                0.65 + 0.35 * self.modulator_pulse.sin().abs()
            } else {
                hover
            };
            d.rounded_rect(
                r.0,
                r.1,
                r.2,
                r.3,
                4.0,
                alpha(accent, 0.025 + emphasis * 0.12),
            );
            d.outline_rounded(
                r.0,
                r.1,
                r.2,
                r.3,
                4.0,
                alpha(
                    if linked || emphasis > 0.0 { accent } else { LINE },
                    0.55 + emphasis * 0.4,
                ),
                1.0,
            );
            for dot in 0..route_count {
                d.circle(r.0 + 8.0 + dot as f32 * 6.0, r.1 + 7.0, 1.5, accent, true);
            }
            if selected {
                d.outline_rounded(
                    r.0 - 1.0,
                    r.1 - 1.0,
                    r.2 + 2.0,
                    r.3 + 2.0,
                    5.0,
                    alpha(accent, emphasis),
                    2.0,
                );
                d.rect(r.0 + 8.0, r.1 + r.3 - 3.0, r.2 - 16.0, 2.0, accent);
            }
            let x = r.0 + 8.0;
            let y = r.1 + 20.0;
            let width = r.2 - 16.0;
            d.rounded_rect(x, y, width, 6.0, 2.0, BG);
            let (start, end) = meter_fill(i, value);
            if end > start {
                d.rounded_rect(x + width * start, y, width * (end - start), 6.0, 2.0, accent);
            }
            if i == 0 || i >= 9 {
                let center = x + width * 0.5;
                d.line(center, y - 2.0, center, y + 8.0, MUTED, 1.0);
            }
            let color = if i >= 9 || linked || dragging || hover > 0.3 { accent } else { MUTED };
            let label_width = d.text_width(label, TEXT_SMALL);
            let left = x + (width - label_width - 22.0) * 0.5;
            style::MODULATOR_ICONS[i].draw(d, left + 8.0, r.1 + 10.0, color);
            d.text(left + 22.0, r.1 + 14.0, label, TEXT_SMALL, color);
        }
        d.font = self.ui_font.get();
    }
}

// Pitch uses a normalized center; dedicated step modulators use signed offsets.
pub(super) fn meter_fill(source: usize, value: f32) -> (f32, f32) {
    let value = if source >= 9 { (value.clamp(-1.0, 1.0) + 1.0) * 0.5 } else { value.clamp(0.0, 1.0) };
    if source == 0 || source >= 9 {
        (value.min(0.5), value.max(0.5))
    } else {
        (0.0, value)
    }
}
