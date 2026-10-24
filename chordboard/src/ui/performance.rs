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
        pleasant_ui::tempo::Header { source: TEMPO_SYNC, bpm: TEMPO_CONTROL }.draw(
            d, synced,
            if synced { self.snapshot.tempo as f64 } else { self.params.tempo.value() as f64 },
            self.params.tempo.unmodulated_normalized_value(), self.hover_amount(TEMPO_SYNC),
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

    pub(super) fn string_count(&self) -> usize {
        let octaves = self.maximum_routed_plain("octaves", self.params.octaves.value() as f32);
        harmony::string_count(self.snapshot.notes.len, octaves as u8)
    }

    // Shared by manual gestures and the expanded pad.
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

    pub(super) fn draw_pad(&self, d: &mut Draw) {
        let pad = self.pad();
        d.rounded_rect(pad.0, pad.1, pad.2, pad.3, 8.0, BG);
        let field = self.play_pad();
        let (min, max) = strum_bounds(self.params.x_min.value(), self.params.x_max.value());
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
            self.params.x_reverse.value(),
        );
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
        let labels = strum_bound_labels(pad, field, (min, max), (y_min, y_max));
        let show_bounds = self.hover_pointer().is_some_and(|(x, y)| hit(pad, x, y))
            || matches!(self.drag, Some(Drag::StrumBound(..)));
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
                if !show_bounds {
                    continue;
                }
                let dragging = matches!(self.drag, Some(Drag::StrumBound(axis, end)) if axis == y_axis && end == maximum);
                let label = if dragging {
                    format!("{value:.2}")
                } else if maximum {
                    "MAX".into()
                } else {
                    "MIN".into()
                };
                let r = labels[y_axis as usize * 2 + maximum as usize];
                if dragging {
                    d.rounded_rect(r.0, r.1, r.2, r.3, 3.0, alpha(color, 0.16));
                    d.circle(ax, ay, 4.0, color, false);
                }
                d.text_centered(r.0 + r.2 * 0.5, r.1 + r.3 * 0.5 + TEXT_SMALL * 0.35, &label, TEXT_SMALL, color);
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
        for (i, label) in SOURCE_SHORT.iter().enumerate() {
            let r = meter_rect(i);
            let value = self.snapshot.sources[i];
            let bypassed = self.params.modulators[i].bypass.value();
            let accent = if bypassed { MUTED } else if matches!(i,9|10) { GOLD } else { TEAL };
            let hover = self.hover_amount(r);
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
            for dot in 0..route_count {
                d.circle(r.0 + 8.0 + dot as f32 * 6.0, r.1 + r.3 - 17.0, 1.5, accent, true);
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
            } else {
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
            }
            let x = r.0 + 8.0;
            let y = r.1 + r.3 - 12.0;
            let width = r.2 - 16.0;
            d.rounded_rect(x, y, width, 6.0, 2.0, BG);
            let (start, end) = meter_fill(i, value);
            if end > start {
                d.rounded_rect(x + width * start, y, width * (end - start), 6.0, 2.0, accent);
            }
            if i == 0 || matches!(i,9|10) {
                let center = x + width * 0.5;
                d.line(center, y - 2.0, center, y + 8.0, MUTED, 1.0);
            }
            let color = if matches!(i,9|10) || linked || dragging || hover > 0.3 { accent } else { MUTED };
            style::MODULATOR_ICONS[i].draw(d, x + 8.0, r.1 + 14.0, color);
            d.text_right(modulator_bypass_rect(i).0 - 5.0, r.1 + 14.0 + TEXT_SMALL * 0.35, label, TEXT_SMALL, color);
            let power = modulator_bypass_rect(i);
            let power_hover = self.hover_amount(power);
            let flash = self.action_flash
                .filter(|(rect, _)| *rect == power)
                .map_or(0.0, |(_, amount)| amount);
            let center = (power.0 + power.2 * 0.5, power.1 + power.3 * 0.5);
            let pulse = if power_hover > 0.0 {
                let seconds = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() % 100_000;
                let seconds = seconds as f32 * 0.001;
                (seconds * 4.5).sin() * 0.5 + 0.5
            } else { 0.0 };
            let power_color = if bypassed && power_hover > 0.01 { TEXT } else { accent };
            if power_hover > 0.0 || flash > 0.0 {
                d.circle(center.0, center.1, 8.0 + pulse + flash * 2.0,
                    alpha(power_color, power_hover * (0.04 + 0.06 * pulse) + flash * 0.2), true);
            }
            d.power_icon_scaled(center.0, center.1, power_color,
                0.7 * (1.0 + flash * 0.15), 1.1 + flash * 0.3);
        }
        d.font = self.ui_font.get();
    }
}

// Pitch uses a normalized center; dedicated step modulators use signed offsets.
pub(super) fn meter_fill(source: usize, value: f32) -> (f32, f32) {
    let value = if matches!(source,9|10) { (value.clamp(-1.0, 1.0) + 1.0) * 0.5 } else { value.clamp(0.0, 1.0) };
    if source == 0 || matches!(source,9|10) {
        (value.min(0.5), value.max(0.5))
    } else {
        (0.0, value)
    }
}
