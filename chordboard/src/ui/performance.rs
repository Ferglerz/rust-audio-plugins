use super::*;

impl ChordboardView {
    pub(super) fn draw_tempo_header(&self, d: &mut Draw) {
        let synced = self.params.tempo_sync.value();
        self.button_tinted(
            d,
            TEMPO_SYNC,
            if synced { "Host tempo" } else { "Manual tempo" },
            true,
            if synced { TEAL } else { GOLD },
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

    pub(super) fn draw_performance_header(&self, d: &mut Draw) {
        d.font = self.font.get();
        d.text(
            PERF_SURFACE.0 + 10.0,
            module_title_y(PERF_SURFACE.1, MODULE_TITLE_SIZE),
            "PERFORMANCE",
            MODULE_TITLE_SIZE,
            TEXT,
        );
        d.font = self.ui_font.get();
        for (i, label) in MODE_LABELS.iter().enumerate() {
            self.button(
                d,
                mode_rect(i),
                label,
                if i == 0 {
                    self.params.mode.value() != 2
                } else {
                    self.params.mode.value() == 2
                },
                TEAL,
            );
            self.live_choice(d, "mode", i as f32 + 1.0, mode_rect(i));
        }
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

    pub(super) fn arp_string_count(&self, _mode: i32) -> usize {
        let octaves = self
            .routed_plain("octaves")
            .unwrap_or(self.params.octaves.value() as f32) as usize;
        self.snapshot.notes.len * octaves
    }

    pub(super) fn arp_cycle_length(&self) -> usize {
        let notes = self.snapshot.notes.len;
        if notes == 0 {
            return 0;
        }
        let octaves = self
            .routed_plain("octaves")
            .unwrap_or(self.params.octaves.value() as f32) as usize;
        let total = (notes * octaves).max(1);
        let loop_start = self.params.loop_start.value().clamp(1, 32) as usize;
        let loop_end = self.params.loop_end.value().clamp(0, 32) as usize;
        let start_idx = loop_start.saturating_sub(1);
        let end_idx = if loop_end > 0 {
            loop_end.saturating_sub(1)
        } else {
            total.saturating_sub(1)
        };
        let (min_idx, max_idx) = if start_idx <= end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };
        let mut unskipped_len = 0;
        for s in min_idx..=max_idx {
            if !self.params.is_string_skipped(s) {
                unskipped_len += 1;
            }
        }
        let pattern = self.params.arp_pattern.value() as u8;
        crate::engine::Engine::pattern_length(pattern, unskipped_len).min(32)
    }

    fn draw_arp_strings(&self, d: &mut Draw, mode: i32) {
        let r = ARP_STRINGS;
        d.rounded_rect(r.0, r.1, r.2, r.3, 6.0, alpha(TEXT, 0.025));
        if self.snapshot.notes.len == 0 {
            d.text_centered(
                r.0 + r.2 * 0.5,
                r.1 + r.3 * 0.5 + 4.0,
                "Play a chord to see its strings",
                TEXT_SMALL,
                MUTED,
            );
            return;
        }
        let base_count = self.arp_string_count(mode).max(1);
        let loop_start_val = self.params.loop_start.value() as usize;
        let loop_end_val = self.params.loop_end.value() as usize;
        let count = if mode == 3 {
            base_count.max(loop_end_val).max(loop_start_val)
        } else {
            base_count
        };
        let effective_end = if loop_end_val > 0 { loop_end_val } else { base_count };
        let start_idx = loop_start_val.saturating_sub(1).min(count - 1);
        let end_idx = effective_end.saturating_sub(1).min(count - 1);
        let (min_idx, max_idx) = if start_idx <= end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };

        if mode == 3 {
            let x_start = arp_string_pos(r, start_idx, count).0;
            let x_end = arp_string_pos(r, end_idx, count).0;
            let (bx1, bx2) = (x_start.min(x_end), x_start.max(x_end));
            d.rounded_rect(
                bx1 - 8.0,
                r.1 + 2.0,
                (bx2 - bx1 + 16.0).max(16.0),
                r.3 - 4.0,
                4.0,
                alpha(GOLD, 0.04),
            );
            d.line(bx1, r.1 + 3.0, bx2, r.1 + 3.0, alpha(GOLD, 0.6), 2.0);
        }

        for i in 0..count {
            let Some(note) = self.snapshot.notes.string(i) else {
                continue;
            };
            let (x, y, bottom) = arp_string_pos(r, i, count);
            let in_loop = mode != 3 || (i >= min_idx && i <= max_idx);
            let pulse = self.note_anim[note as usize];
            let sounding = self.snapshot.sounding_notes[note as usize];
            let color = if sounding { GOLD } else { TEAL };

            if in_loop {
                if mode == 3 {
                    let is_skipped = self.params.is_string_skipped(i);
                    let is_muted = self.params.is_string_muted(i);
                    let vol = self.params.string_volume(i);
                    let vol_y = bottom - vol * (bottom - y);

                    if is_skipped {
                        d.line(x, y, x, bottom, alpha(MUTED, 0.2), 1.0);
                        d.rounded_rect(x - 13.0, bottom + 2.0, 26.0, 14.0, 3.0, alpha(MUTED, 0.25));
                        d.text_centered(x, bottom + 12.0, "SKIP", 8.0, alpha(TEXT, 0.45));
                    } else if is_muted {
                        d.line(x, y, x, bottom, alpha(GOLD, 0.2), 1.0);
                        d.rounded_rect(x - 3.5, vol_y - 1.5, 7.0, 3.0, 1.5, alpha(GOLD, 0.6));
                        d.rounded_rect(x - 13.0, bottom + 2.0, 26.0, 14.0, 3.0, alpha(GOLD, 0.25));
                        d.text_centered(x, bottom + 12.0, "MUTE", 8.0, GOLD);
                    } else {
                        if sounding || pulse > 0.01 {
                            d.rounded_rect(
                                x - 6.0,
                                vol_y,
                                12.0,
                                bottom - vol_y,
                                3.0,
                                alpha(color, 0.08 + pulse * 0.12),
                            );
                        }
                        let displacement = (pulse * 24.0).sin() * pulse * 3.0;
                        let mid_y = (vol_y + bottom) * 0.5;
                        d.poly(
                            &[(x, vol_y), (x + displacement, mid_y), (x, bottom)],
                            alpha(color, if sounding { 0.95 } else { 0.35 + pulse * 0.65 }),
                            1.0 + pulse,
                        );
                        if vol < 0.98 {
                            d.line(x, y, x, vol_y, alpha(color, 0.15), 1.0);
                        }
                        d.rounded_rect(
                            x - 4.0,
                            vol_y - 1.5,
                            8.0,
                            3.0,
                            1.5,
                            if sounding { GOLD } else { TEAL },
                        );
                        d.text_centered(
                            x,
                            bottom + 13.0,
                            &self.note_name(note),
                            TEXT_SMALL,
                            if sounding || pulse > 0.1 {
                                color
                            } else {
                                MUTED
                            },
                        );
                    }
                } else {
                    if sounding || pulse > 0.01 {
                        d.rounded_rect(
                            x - 6.0,
                            y,
                            12.0,
                            bottom - y,
                            3.0,
                            alpha(color, 0.08 + pulse * 0.12),
                        );
                    }
                    let displacement = (pulse * 24.0).sin() * pulse * 3.0;
                    d.poly(
                        &[(x, y), (x + displacement, (y + bottom) * 0.5), (x, bottom)],
                        alpha(color, if sounding { 0.95 } else { 0.25 + pulse * 0.65 }),
                        1.0 + pulse,
                    );
                    d.text_centered(
                        x,
                        bottom + 13.0,
                        &self.note_name(note),
                        TEXT_SMALL,
                        if sounding || pulse > 0.1 {
                            color
                        } else {
                            MUTED
                        },
                    );
                }
            } else {
                d.line(x, y, x, bottom, alpha(MUTED, 0.15), 1.0);
                d.text_centered(
                    x,
                    bottom + 13.0,
                    &self.note_name(note),
                    TEXT_SMALL,
                    alpha(MUTED, 0.35),
                );
            }
        }

        if mode == 3 {
            let x_start = arp_string_pos(r, start_idx, count).0;
            let x_end = arp_string_pos(r, end_idx, count).0;
            let tag_w = 16.0;
            let tag_h = 11.0;
            let tag_y = r.1 + 2.0;

            let (xs, xe) = if (x_start - x_end).abs() < 12.0 {
                (x_start - 7.0, x_end + 7.0)
            } else {
                (x_start, x_end)
            };
            let (bx1, bx2) = (xs.min(xe), xs.max(xe));
            d.line(bx1, tag_y + tag_h * 0.5, bx2, tag_y + tag_h * 0.5, alpha(GOLD, 0.4), 1.5);

            let end_str = if loop_end_val == 0 {
                "Auto".to_string()
            } else {
                loop_end_val.to_string()
            };
            if self.drag.is_none() {
                if (bx2 - bx1) >= 80.0 {
                    let loop_len = effective_end.saturating_sub(loop_start_val).saturating_add(1);
                    d.text_centered(
                        (bx1 + bx2) * 0.5,
                        tag_y + 8.5,
                        &format!("{loop_start_val} → {end_str} ({loop_len} notes)"),
                        9.0,
                        alpha(TEXT, 0.7),
                    );
                } else if (bx2 - bx1) >= 44.0 {
                    d.text_centered(
                        (bx1 + bx2) * 0.5,
                        tag_y + 8.5,
                        &format!("{loop_start_val} → {end_str}"),
                        9.0,
                        alpha(TEXT, 0.7),
                    );
                }
            }

            d.rounded_rect(xs - tag_w * 0.5, tag_y, tag_w, tag_h, 3.0, TEAL);
            d.fill_poly(
                &[
                    (xs - 3.5, tag_y + tag_h),
                    (xs + 3.5, tag_y + tag_h),
                    (xs, tag_y + tag_h + 3.0),
                ],
                TEAL,
            );
            d.text_centered(xs, tag_y + 8.5, "S", 8.5, BG);

            d.rounded_rect(xe - tag_w * 0.5, tag_y, tag_w, tag_h, 3.0, GOLD);
            d.fill_poly(
                &[
                    (xe - 3.5, tag_y + tag_h),
                    (xe + 3.5, tag_y + tag_h),
                    (xe, tag_y + tag_h + 3.0),
                ],
                GOLD,
            );
            d.text_centered(xe, tag_y + 8.5, "E", 8.5, BG);
        }

        // Draw Adaptive Step Accent Lane
        let cycle = self.arp_cycle_length();
        if cycle > 0 {
            let active_step = self.snapshot.arp_cycle_step % cycle;
            for s in 0..cycle {
                let (px, py) = arp_accent_step_pos(r, s, cycle);
                let accent = self.params.step_accent(s);
                let is_sounding = self.snapshot.notes.len > 0 && active_step == s;

                // Playhead highlight
                if is_sounding {
                    let halo_color = if accent == 1 { GOLD } else { TEAL };
                    d.rounded_rect(px - 5.0, py - 6.0, 10.0, 12.0, 3.0, alpha(halo_color, 0.25));
                }

                match accent {
                    1 => {
                        // Accent: Tall gold peg
                        d.rounded_rect(px - 2.5, py - 5.0, 5.0, 10.0, 1.5, GOLD);
                        d.circle(px, py - 5.0, 2.0, GOLD, true);
                    }
                    2 => {
                        // Ghost: Muted staccato pip with subtle horizontal bracket
                        d.circle(px, py, 2.2, alpha(TEXT, 0.55), true);
                        d.line(px - 3.5, py, px + 3.5, py, alpha(TEXT, 0.45), 1.0);
                    }
                    _ => {
                        // Normal: Subtle hollow dot
                        d.circle(px, py, 2.0, alpha(MUTED, 0.4), false);
                    }
                }
            }
        }
    }

    pub(super) fn draw_arp(&self, d: &mut Draw, mode: i32) {
        let looping = mode == 3;
        let pad = self.pad();
        d.rounded_rect(pad.0, pad.1, pad.2, pad.3, 8.0, BG);
        d.font = self.ui_font.get();
        for loop_choice in [false, true] {
            self.button(
                d,
                repeat_rect(loop_choice),
                if loop_choice { "Loop" } else { "Once" },
                looping == loop_choice,
                GOLD,
            );
        }
        self.draw_arp_strings(d, mode);
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
        d.text(OCTAVE_LABEL.0, OCTAVE_LABEL.1, "Octaves", TEXT_SMALL, MUTED);
        for i in 0..4 {
            self.button(
                d,
                octave_rect(i),
                &(i + 1).to_string(),
                self.params.octaves.value() == i as i32 + 1,
                GOLD,
            );
            self.live_choice(d, "octaves", i as f32 + 1.0, octave_rect(i));
        }
        self.button(
            d,
            RATE_SYNC,
            if self.sweep_synced() {
                "Tempo sync"
            } else {
                "Free · ms"
            },
            self.sweep_synced(),
            TEAL,
        );
        self.live_choice(d, "strum_sync", 1.0, RATE_SYNC);
        if self.sweep_synced() {
            let header = RATE_HEADER;
            d.text(header.0, header.1 + 15.0, "Rate", TEXT_SMALL, MUTED);
            self.draw_modulation_badge(d, "rate", (header.0 + 46.0, header.1, 194.0, 20.0));
            let beats = self
                .routed_plain("rate")
                .unwrap_or(self.params.rate.value());
            d.text_right(
                header.0 + header.2,
                header.1 + 15.0,
                &format!("{beats:.3} beats"),
                TEXT_SMALL,
                if self.routed_plain("rate").is_some() {
                    TEAL
                } else {
                    MUTED
                },
            );
            for (i, (label, beats)) in ARP_RATES.iter().enumerate() {
                self.button(
                    d,
                    rate_rect(i),
                    label,
                    (self.params.rate.value() - beats).abs() < 0.0001,
                    GOLD,
                );
                self.live_choice(d, "rate", *beats, rate_rect(i));
            }
        }
        if !looping {
            self.button(
                d,
                STRUM_HOLD,
                "Hold notes",
                self.params.strum_hold.value(),
                TEAL,
            );
            self.live_choice(d, "strum_hold", 1.0, STRUM_HOLD);
        }
        for (id, r) in arp_controls()
            .into_iter()
            .skip(1)
            .filter(|(id, _)| looping || *id != "gate")
        {
            let gate = self
                .routed_plain("gate")
                .unwrap_or(self.params.gate.value());
            let swing = self
                .routed_plain("swing")
                .unwrap_or(self.params.swing.value());
            let is_gate = id == "gate";
            d.rounded_rect(r.0, r.1, r.2, r.3, 6.0, alpha(TEXT, 0.025));
            self.draw_modulation_badge(d, id, (r.0 + 66.0, r.1, (r.2 - 148.0).max(0.0), 20.0));
            d.text(
                r.0 + 12.0,
                r.1 + 17.0,
                if is_gate { "Gate" } else { "Swing" },
                TEXT_SMALL,
                MUTED,
            );
            d.text_right(
                r.0 + r.2 - 12.0,
                r.1 + 17.0,
                &format!("{:.0}%", if is_gate { gate * 100.0 } else { swing * 100.0 }),
                TEXT_SMALL,
                if self.routed_plain(id).is_some() {
                    TEAL
                } else {
                    GOLD
                },
            );
            let step = (r.2 - 24.0) / 8.0;
            for i in 0..8 {
                let start = if i % 2 == 0 {
                    i as f32
                } else {
                    i as f32 + swing
                };
                let duration = if i % 2 == 0 { 1.0 + swing } else { 1.0 - swing };
                let x = r.0 + 12.0 + start * step;
                d.line(
                    r.0 + 12.0 + i as f32 * step,
                    r.1 + 26.0,
                    r.0 + 12.0 + i as f32 * step,
                    r.1 + 46.0,
                    LINE,
                    1.0,
                );
                d.rounded_rect(
                    x,
                    r.1 + 29.0,
                    (step * duration * if is_gate { gate } else { 0.8 }).max(2.0),
                    13.0,
                    3.0,
                    if i % 2 == 0 { GOLD } else { alpha(GOLD, 0.5) },
                );
            }
        }

        if looping {
            self.button(d, loop_reset_rect(), "Reset", false, GOLD);

            if let Some(Drag::ArpLoopBound(is_end)) = self.drag {
                let r = ARP_STRINGS;
                let base_count = self.arp_string_count(3).max(1);
                let loop_start_val = self.params.loop_start.value() as usize;
                let loop_end_val = self.params.loop_end.value() as usize;
                let count = base_count.max(loop_end_val).max(loop_start_val);
                let effective_end = if loop_end_val > 0 { loop_end_val } else { base_count };
                let target_idx = if is_end {
                    effective_end.saturating_sub(1).min(count - 1)
                } else {
                    loop_start_val.saturating_sub(1).min(count - 1)
                };
                let x = arp_string_pos(r, target_idx, count).0;
                let text = if is_end {
                    if loop_end_val == 0 {
                        format!("End: Auto ({effective_end})")
                    } else {
                        format!("End: {loop_end_val}")
                    }
                } else {
                    format!("Begin: {loop_start_val}")
                };
                let badge_w = 78.0;
                let badge_h = 16.0;
                let badge_x = (x - badge_w * 0.5).clamp(r.0 + 4.0, r.0 + r.2 - badge_w - 4.0);
                let badge_y = r.1 + 17.0;
                d.rounded_rect(badge_x, badge_y, badge_w, badge_h, 3.0, BG);
                d.outline_rounded(
                    badge_x,
                    badge_y,
                    badge_w,
                    badge_h,
                    3.0,
                    if is_end { GOLD } else { TEAL },
                    1.0,
                );
                d.text_centered(
                    badge_x + badge_w * 0.5,
                    badge_y + 11.5,
                    &text,
                    9.5,
                    if is_end { GOLD } else { TEAL },
                );
            } else if let Some(Drag::ArpVolumeSweep {
                origin_idx,
                origin_vol,
                last_pos,
                is_ramp,
                ..
            }) = self.drag
            {
                let r = ARP_STRINGS;
                let base_count = self.arp_string_count(3).max(1);
                let loop_start_val = self.params.loop_start.value() as usize;
                let loop_end_val = self.params.loop_end.value() as usize;
                let count = base_count.max(loop_end_val).max(loop_start_val);

                let cur_idx = arp_string_index_at(r, last_pos.0, last_pos.1, count);
                let cur_vol = self.params.string_volume(cur_idx);

                let tip = if is_ramp {
                    format!(
                        "Ramp #{}-#{} ({:.0}% → {:.0}%)",
                        origin_idx + 1,
                        cur_idx + 1,
                        origin_vol * 100.0,
                        cur_vol * 100.0
                    )
                } else {
                    format!("#{}: {:.0}%", cur_idx + 1, cur_vol * 100.0)
                };
                let tip_w = if is_ramp { 150.0 } else { 62.0 };
                let tip_h = 16.0;
                let tip_x = (last_pos.0 - tip_w * 0.5).clamp(r.0 + 4.0, r.0 + r.2 - tip_w - 4.0);
                let tip_y = (last_pos.1 - 22.0).clamp(r.1 + 4.0, r.1 + r.3 - tip_h - 4.0);
                d.rounded_rect(tip_x, tip_y, tip_w, tip_h, 3.0, BG);
                d.outline_rounded(tip_x, tip_y, tip_w, tip_h, 3.0, GOLD, 1.0);
                d.text_centered(tip_x + tip_w * 0.5, tip_y + 11.5, &tip, 9.0, GOLD);
            }
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
        let n = self
            .routed_plain("strings")
            .unwrap_or(self.params.strings.value() as f32) as usize;
        for i in 0..n {
            let x = field.0 + (min + i as f32 * (max - min) / (n - 1) as f32) * field.2;
            let string = if manual && self.params.x_reverse.value() {
                n - 1 - i
            } else {
                i
            };
            let pulse = self.string_anim[string];
            let color = if self.snapshot.notes.string(string).is_some() {
                TEAL
            } else {
                MUTED
            };
            if pulse > 0.01 {
                d.rect(x - 5.0, field.1, 10.0, field.3, alpha(color, pulse * 0.10));
            }
            let displacement = (pulse * 24.0).sin() * pulse * 3.0;
            d.poly(
                &[
                    (x, field.1),
                    (x + displacement, field.1 + field.3 / 2.0),
                    (x, field.1 + field.3),
                ],
                alpha(color, 0.15 + pulse * 0.7),
                1.0 + pulse,
            );
            if let Some(note) = self.snapshot.notes.string(string) {
                d.text_centered(
                    x,
                    field.1 + field.3 + 14.0,
                    &self.note_name(note),
                    TEXT_SMALL,
                    if pulse > 0.1 { color } else { MUTED },
                );
            }
        }
        d.text(
            pad.0 + 10.0,
            module_title_y(pad.1, MODULE_TITLE_SIZE),
            if manual { "STRUM FIELD" } else { "AUTO STRUM" },
            MODULE_TITLE_SIZE,
            TEXT,
        );
        if manual {
            let r = self.expand_button();
            d.graph_zoom_button(
                r,
                self.expand_target < 0.5,
                self.hover_amount(r) > 0.0,
                true,
            );
        }
        if manual {
            self.touch_latch_button(
                d,
                strum_latch_rect(self.expand_t()),
                self.params.strum_latch.value(),
                TEAL,
            );
        } else {
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
                "HOLD",
                self.params.strum_hold.value(),
                TEAL,
            );
            if let Some(live) = self.routed_plain("strum_sync") {
                let r = strum_sync_rect(self.expand_t());
                self.live_highlight(d, r);
                d.text_right(
                    r.0 + r.2 - 4.0,
                    r.1 + r.3 - 3.0,
                    if live >= 0.5 { "ON" } else { "OFF" },
                    8.0,
                    TEAL,
                );
            }
            if let Some(live) = self.routed_plain("strum_hold") {
                let r = strum_hold_rect(self.expand_t());
                self.live_highlight(d, r);
                d.text_right(
                    r.0 + r.2 - 4.0,
                    r.1 + r.3 - 3.0,
                    if live >= 0.5 { "ON" } else { "OFF" },
                    8.0,
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
                    d.text_centered(ax - 22.0, ay + 22.0, &label, TEXT_SMALL, color);
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
            harmony::voice(
                (self.params.keyboard_octave.value() + self.params.key.value()).clamp(0, 127) as u8,
                self.params.quality.value() as u8,
                None,
                self.params.transpose.value() as i8,
                self.params.inversion.value() as u8,
                self.params.spread.value() as u8,
            )
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
        d.text(
            MOD_SURFACE.0 + 132.0,
            module_title_y(MOD_SURFACE.1, 11.0),
            "Drag to route",
            11.0,
            MUTED,
        );
        for (i, label) in SOURCE_SHORT.iter().enumerate() {
            let r = meter_rect(i);
            let value = self.snapshot.sources[i];
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
                alpha(TEAL, 0.025 + emphasis * 0.12),
            );
            d.outline_rounded(
                r.0,
                r.1,
                r.2,
                r.3,
                4.0,
                alpha(
                    if linked || emphasis > 0.0 { TEAL } else { LINE },
                    0.55 + emphasis * 0.4,
                ),
                1.0,
            );
            for dot in 0..route_count {
                d.circle(r.0 + 8.0 + dot as f32 * 6.0, r.1 + 7.0, 1.5, TEAL, true);
            }
            if selected {
                d.outline_rounded(
                    r.0 - 1.0,
                    r.1 - 1.0,
                    r.2 + 2.0,
                    r.3 + 2.0,
                    5.0,
                    alpha(TEAL, emphasis),
                    2.0,
                );
                d.rect(r.0 + 8.0, r.1 + r.3 - 3.0, r.2 - 16.0, 2.0, TEAL);
            }
            let x = r.0 + 8.0;
            let y = r.1 + 20.0;
            let width = r.2 - 16.0;
            d.rounded_rect(x, y, width, 6.0, 2.0, BG);
            let (start, end) = meter_fill(i, value);
            if end > start {
                d.rounded_rect(x + width * start, y, width * (end - start), 6.0, 2.0, TEAL);
            }
            if i == 0 {
                let center = x + width * 0.5;
                d.line(center, y - 2.0, center, y + 8.0, MUTED, 1.0);
            }
            d.text_centered(
                r.0 + r.2 * 0.5,
                r.1 + 14.0,
                label,
                TEXT_SMALL,
                if linked || dragging || hover > 0.3 {
                    TEAL
                } else {
                    MUTED
                },
            );
        }
        d.font = self.ui_font.get();
    }
}

// Pitch is bipolar: center is neutral, while other sources fill from zero.
pub(super) fn meter_fill(source: usize, value: f32) -> (f32, f32) {
    let value = value.clamp(0.0, 1.0);
    if source == 0 {
        (value.min(0.5), value.max(0.5))
    } else {
        (0.0, value)
    }
}
