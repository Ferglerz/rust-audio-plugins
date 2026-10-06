use super::*;
use crate::engine::Lane;
use crate::params::WURLI_PRESETS;
use crate::sound::LaneEngineType;

pub(super) const SURFACE_TAB_CHORDS: Rect = (420.0, 23.0, 96.0, 30.0);
pub(super) const SURFACE_TAB_SOUND: Rect = (524.0, 23.0, 96.0, 30.0);
pub(super) const SURFACE_TAB_ROUTES: Rect = (628.0, 23.0, 96.0, 30.0);
pub(super) const SURFACE_TAB_LIVE: Rect = (732.0, 23.0, 130.0, 30.0);

pub(super) const SOUND_AUDITION: Rect = (25.0, 474.0, 140.0, 28.0);
pub(super) const SOUND_LATCH: Rect = (173.0, 474.0, 72.0, 28.0);
pub(super) const SOUND_RESET_ALL: Rect =
    (CHORDS_SURFACE.0 + CHORDS_SURFACE.2 - 116.0, 474.0, 100.0, 28.0);

pub(super) fn lane_strip_rect(lane: usize) -> Rect {
    let sx = 25.0 + lane as f32 * 228.0;
    (sx, 134.0, 222.0, 330.0)
}

pub(super) fn lane_engine_pill_rect(lane: usize) -> Rect {
    let r = lane_strip_rect(lane);
    (r.0 + 8.0, r.1 + 24.0, r.2 - 16.0, 20.0)
}

pub(super) fn lane_preset_prev_rect(lane: usize) -> Rect {
    let r = lane_strip_rect(lane);
    (r.0 + 8.0, r.1 + 48.0, 20.0, 18.0)
}

pub(super) fn lane_preset_name_rect(lane: usize) -> Rect {
    let r = lane_strip_rect(lane);
    (r.0 + 32.0, r.1 + 48.0, r.2 - 64.0, 18.0)
}

pub(super) fn lane_preset_next_rect(lane: usize) -> Rect {
    let r = lane_strip_rect(lane);
    (r.0 + r.2 - 28.0, r.1 + 48.0, 20.0, 18.0)
}

pub(super) fn lane_macro_rect(lane: usize, macro_idx: usize) -> Rect {
    let r = lane_strip_rect(lane);
    let col = macro_idx % 3;
    let row = macro_idx / 3;
    let x = r.0 + 8.0 + col as f32 * 71.0;
    let y = r.1 + 72.0 + row as f32 * 56.0;
    (x, y, 64.0, 52.0)
}

pub(super) fn lane_opt_a_rect(lane: usize) -> Rect {
    let r = lane_strip_rect(lane);
    (r.0 + 8.0, r.1 + 186.0, (r.2 - 20.0) * 0.5, 20.0)
}

pub(super) fn lane_opt_b_rect(lane: usize) -> Rect {
    let r = lane_strip_rect(lane);
    let w = (r.2 - 20.0) * 0.5;
    (r.0 + 12.0 + w, r.1 + 186.0, w, 20.0)
}

pub(super) fn lane_level_rect(lane: usize) -> Rect {
    let r = lane_strip_rect(lane);
    (r.0 + 8.0, r.1 + 216.0, r.2 - 44.0, 26.0)
}

pub(super) fn lane_pan_rect(lane: usize) -> Rect {
    let r = lane_strip_rect(lane);
    (r.0 + 8.0, r.1 + 248.0, r.2 - 44.0, 26.0)
}

pub(super) fn lane_mute_rect(lane: usize) -> Rect {
    let r = lane_strip_rect(lane);
    (r.0 + 8.0, r.1 + 282.0, 56.0, 24.0)
}

pub(super) fn lane_meter_rect(lane: usize) -> Rect {
    let r = lane_strip_rect(lane);
    (r.0 + r.2 - 28.0, r.1 + 216.0, 20.0, 90.0)
}

impl ChordboardView {
    pub(super) fn lane_color(&self, lane: Lane) -> Color {
        match lane {
            Lane::Bass => GOLD,
            Lane::Comp => TEAL,
            Lane::Arp => Color::rgb(230, 160, 80),
            Lane::Lead => Color::rgb(130, 205, 245),
        }
    }

    pub(super) fn draw_surface_tabs(&self, d: &mut Draw) {
        let active_tab = match self.panel {
            Some(Panel::Sound) => 1,
            Some(Panel::Routes) => 2,
            _ => 0,
        };

        for (i, &(r, label)) in [
            (SURFACE_TAB_CHORDS, "CHORDS"),
            (SURFACE_TAB_SOUND, "SOUND"),
            (SURFACE_TAB_ROUTES, "ROUTES"),
        ]
        .iter()
        .enumerate()
        {
            self.button(d, r, label, i == active_tab, TEAL);
        }

        let is_live = self.params.console_view.value() == 1;
        self.button(d, SURFACE_TAB_LIVE, "🎛 LIVE CONSOLE", is_live, GOLD);
    }

    pub(super) fn draw_sound(&self, d: &mut Draw) {
        d.font = self.font.get();
        d.text(
            CHORDS_SURFACE.0 + 16.0,
            module_title_y(CHORDS_SURFACE.1, MODULE_TITLE_SIZE),
            "SOUND INSTRUMENT",
            MODULE_TITLE_SIZE,
            TEXT,
        );
        d.font = self.ui_font.get();
        self.close_icon(d, self.panel_close_rect(Panel::Sound), TEAL, false);

        // Update meter decay
        for lane in 0..4 {
            let peak = self.bridge.lane_peak(lane);
            let prev = self.lane_meters[lane].get();
            let current = if peak > prev { peak } else { prev * 0.88 };
            self.lane_meters[lane].set(current);
        }

        // Render 4 Lane Strips
        for &lane in &Lane::ALL {
            let i = lane.index();
            let sr = lane_strip_rect(i);
            let lp = self.params.lane_params(lane);
            let color = self.lane_color(lane);
            let engine_type = LaneEngineType::from_index(lp.engine.value());

            // Strip background container
            d.rounded_rect(sr.0, sr.1, sr.2, sr.3, 6.0, alpha(BG, 0.6));
            d.outline_rounded(
                sr.0,
                sr.1,
                sr.2,
                sr.3,
                6.0,
                alpha(if engine_type == LaneEngineType::Off { MUTED } else { color }, 0.35),
                1.0,
            );

            // Strip Title Bar
            let title_color = if engine_type == LaneEngineType::Off { MUTED } else { color };
            d.rounded_rect(sr.0 + 8.0, sr.1 + 6.0, 6.0, 14.0, 2.0, title_color);
            d.font = self.bold_font.get();
            d.text(sr.0 + 20.0, sr.1 + 18.0, lane.name(), 13.0, TEXT);

            // Engine Selector Button
            let eng_r = lane_engine_pill_rect(i);
            let eng_hover = self.hover_amount(eng_r);
            d.rounded_rect(
                eng_r.0,
                eng_r.1,
                eng_r.2,
                eng_r.3,
                4.0,
                alpha(color, if engine_type == LaneEngineType::Off { 0.08 } else { 0.16 + eng_hover * 0.08 }),
            );
            d.outline_rounded(
                eng_r.0,
                eng_r.1,
                eng_r.2,
                eng_r.3,
                4.0,
                alpha(color, if engine_type == LaneEngineType::Off { 0.3 } else { 0.8 }),
                1.0,
            );
            d.font = self.ui_font.get();
            d.text_centered(
                eng_r.0 + eng_r.2 * 0.5 - 6.0,
                eng_r.1 + eng_r.3 * 0.5 + 4.0,
                engine_type.name(),
                11.0,
                if engine_type == LaneEngineType::Off { MUTED } else { TEXT },
            );
            d.text_centered(
                eng_r.0 + eng_r.2 - 14.0,
                eng_r.1 + eng_r.3 * 0.5 + 4.0,
                "▾",
                10.0,
                alpha(MUTED, 0.7),
            );

            // Preset Selector (if engine != Off)
            if engine_type != LaneEngineType::Off {
                let pr_idx = lp.preset.value() as usize;
                let preset_name = match engine_type {
                    LaneEngineType::OpenWurli => {
                        WURLI_PRESETS.get(pr_idx).map_or("Custom", |p| p.0)
                    }
                    LaneEngineType::Off => "—",
                };

                let prev_r = lane_preset_prev_rect(i);
                let next_r = lane_preset_next_rect(i);
                let name_r = lane_preset_name_rect(i);

                d.rounded_rect(
                    prev_r.0,
                    prev_r.1,
                    prev_r.2,
                    prev_r.3,
                    3.0,
                    alpha(LINE, 0.3 + self.hover_amount(prev_r) * 0.2),
                );
                d.text_centered(prev_r.0 + prev_r.2 * 0.5, prev_r.1 + prev_r.3 * 0.5 + 3.0, "‹", 13.0, TEXT);

                d.rounded_rect(name_r.0, name_r.1, name_r.2, name_r.3, 3.0, alpha(BG, 0.5));
                d.text_centered(name_r.0 + name_r.2 * 0.5, name_r.1 + name_r.3 * 0.5 + 3.0, preset_name, 10.0, TEXT);

                d.rounded_rect(
                    next_r.0,
                    next_r.1,
                    next_r.2,
                    next_r.3,
                    3.0,
                    alpha(LINE, 0.3 + self.hover_amount(next_r) * 0.2),
                );
                d.text_centered(next_r.0 + next_r.2 * 0.5, next_r.1 + next_r.3 * 0.5 + 3.0, "›", 13.0, TEXT);

                // Macro Knobs (2 rows of 3)
                let macro_labels: [&str; 6] = match engine_type {
                    LaneEngineType::OpenWurli => ["Drive", "Tremolo", "Trem Rate", "Speaker", "Reed", "Hammer"],
                    LaneEngineType::Off => ["", "", "", "", "", ""],
                };

                let macro_controls = [
                    self.control(&format!("lane_{}_m1", lane.name().to_lowercase())),
                    self.control(&format!("lane_{}_m2", lane.name().to_lowercase())),
                    self.control(&format!("lane_{}_m3", lane.name().to_lowercase())),
                    self.control(&format!("lane_{}_m4", lane.name().to_lowercase())),
                    self.control(&format!("lane_{}_m5", lane.name().to_lowercase())),
                    self.control(&format!("lane_{}_m6", lane.name().to_lowercase())),
                ];

                for (m_idx, (label, ctrl)) in macro_labels.iter().zip(macro_controls.iter()).enumerate() {
                    let mr = lane_macro_rect(i, m_idx);
                    let norm = ctrl.as_ref().map_or(0.5, |c| c.norm);
                    let val_str = ctrl.as_ref().map_or(String::new(), |c| c.value.clone());
                    d.knob(mr, label, &val_str, norm, color, false);
                }

                // Option Toggles (Opt A & Opt B)
                let (opt_a_label, opt_b_label) = match engine_type {
                    LaneEngineType::OpenWurli => ("Heavy Ckt", "Rail Sag"),
                    LaneEngineType::Off => ("", ""),
                };

                let opt_a_r = lane_opt_a_rect(i);
                let opt_b_r = lane_opt_b_rect(i);
                let opt_a_on = lp.opt_a.value();
                let opt_b_on = lp.opt_b.value();

                self.button(d, opt_a_r, opt_a_label, opt_a_on, color);
                self.button(d, opt_b_r, opt_b_label, opt_b_on, color);
            } else {
                // Engine is Off: Bypassed Banner
                let off_box = (sr.0 + 16.0, sr.1 + 80.0, sr.2 - 32.0, 120.0);
                d.rounded_rect(off_box.0, off_box.1, off_box.2, off_box.3, 6.0, alpha(LINE, 0.2));
                d.font = self.ui_font.get();
                d.text_centered(
                    off_box.0 + off_box.2 * 0.5,
                    off_box.1 + 44.0,
                    "MIDI Output Only",
                    12.0,
                    MUTED,
                );
                d.text_centered(
                    off_box.0 + off_box.2 * 0.5,
                    off_box.1 + 68.0,
                    "(Sound Engine Bypassed)",
                    10.0,
                    alpha(MUTED, 0.7),
                );
            }

            // Separator
            d.line(sr.0 + 8.0, sr.1 + 210.0, sr.0 + sr.2 - 8.0, sr.1 + 210.0, alpha(LINE, 0.4), 1.0);

            // Mixer Controls
            let lvl_r = lane_level_rect(i);
            let pan_r = lane_pan_rect(i);
            let mut_r = lane_mute_rect(i);
            let met_r = lane_meter_rect(i);

            let lvl_ctrl = self.control(&format!("lane_{}_level", lane.name().to_lowercase()));
            let pan_ctrl = self.control(&format!("lane_{}_pan", lane.name().to_lowercase()));
            let mut_ctrl = self.control(&format!("lane_{}_mute", lane.name().to_lowercase()));

            if let Some(c) = lvl_ctrl {
                self.draw_control(d, &c, lvl_r, color);
            }
            if let Some(c) = pan_ctrl {
                self.draw_control(d, &c, pan_r, color);
            }
            if let Some(c) = mut_ctrl {
                self.draw_control(d, &c, mut_r, Color::rgb(220, 70, 70));
            }

            // Live VU Peak Meter
            let peak_norm = self.lane_meters[i].get().clamp(0.0, 1.2) / 1.2;
            d.rounded_rect(met_r.0, met_r.1, met_r.2, met_r.3, 3.0, alpha(BG, 0.8));
            d.outline_rounded(met_r.0, met_r.1, met_r.2, met_r.3, 3.0, alpha(LINE, 0.5), 1.0);

            let meter_fill_h = met_r.3 * peak_norm;
            if meter_fill_h > 1.0 {
                let fill_y = met_r.1 + met_r.3 - meter_fill_h;
                let meter_color = if peak_norm > 0.85 {
                    Color::rgb(240, 80, 80)
                } else if peak_norm > 0.65 {
                    Color::rgb(240, 180, 50)
                } else {
                    color
                };
                d.rounded_rect(met_r.0 + 2.0, fill_y + 1.0, met_r.2 - 4.0, meter_fill_h - 2.0, 2.0, meter_color);
            }
        }

        // Bottom Bar (Audition Chord & Latch)
        let aud_hover = self.hover_amount(SOUND_AUDITION);
        let aud_active = self.auditioning.get();
        d.rounded_rect(
            SOUND_AUDITION.0,
            SOUND_AUDITION.1,
            SOUND_AUDITION.2,
            SOUND_AUDITION.3,
            4.0,
            alpha(GOLD, if aud_active { 0.3 } else { 0.12 + aud_hover * 0.08 }),
        );
        d.outline_rounded(
            SOUND_AUDITION.0,
            SOUND_AUDITION.1,
            SOUND_AUDITION.2,
            SOUND_AUDITION.3,
            4.0,
            GOLD,
            1.2,
        );
        d.font = self.bold_font.get();
        d.text_centered(
            SOUND_AUDITION.0 + SOUND_AUDITION.2 * 0.5,
            SOUND_AUDITION.1 + SOUND_AUDITION.3 * 0.5 + 4.0,
            if aud_active { "■ Stop Audition" } else { "▶ Audition Chord" },
            12.0,
            TEXT,
        );

        let latch_hover = self.hover_amount(SOUND_LATCH);
        let latch_active = self.audition_latch.get();
        d.rounded_rect(
            SOUND_LATCH.0,
            SOUND_LATCH.1,
            SOUND_LATCH.2,
            SOUND_LATCH.3,
            4.0,
            alpha(TEAL, if latch_active { 0.28 } else { 0.08 + latch_hover * 0.08 }),
        );
        d.outline_rounded(
            SOUND_LATCH.0,
            SOUND_LATCH.1,
            SOUND_LATCH.2,
            SOUND_LATCH.3,
            4.0,
            alpha(TEAL, if latch_active { 1.0 } else { 0.5 }),
            1.0,
        );
        d.font = self.ui_font.get();
        d.text_centered(
            SOUND_LATCH.0 + SOUND_LATCH.2 * 0.5,
            SOUND_LATCH.1 + SOUND_LATCH.3 * 0.5 + 4.0,
            "Latch",
            11.0,
            if latch_active { TEAL } else { MUTED },
        );

        // Reset All Defaults Button
        let rst_hover = self.hover_amount(SOUND_RESET_ALL);
        d.rounded_rect(
            SOUND_RESET_ALL.0,
            SOUND_RESET_ALL.1,
            SOUND_RESET_ALL.2,
            SOUND_RESET_ALL.3,
            4.0,
            alpha(LINE, 0.2 + rst_hover * 0.15),
        );
        d.outline_rounded(
            SOUND_RESET_ALL.0,
            SOUND_RESET_ALL.1,
            SOUND_RESET_ALL.2,
            SOUND_RESET_ALL.3,
            4.0,
            alpha(MUTED, 0.5),
            1.0,
        );
        d.text_centered(
            SOUND_RESET_ALL.0 + SOUND_RESET_ALL.2 * 0.5,
            SOUND_RESET_ALL.1 + SOUND_RESET_ALL.3 * 0.5 + 4.0,
            "Reset Engines",
            10.0,
            MUTED,
        );
    }

    pub(super) fn apply_lane_preset(
        &self,
        cx: &mut EventContext,
        lane: Lane,
        engine_type: LaneEngineType,
        preset_idx: usize,
    ) {
        let lp = self.params.lane_params(lane);
        Self::emit(cx, lp.preset.as_ptr(), lp.preset.preview_normalized(preset_idx as i32));

        match engine_type {
            LaneEngineType::OpenWurli => {
                if let Some(p) = WURLI_PRESETS.get(preset_idx) {
                    Self::emit(cx, lp.m1.as_ptr(), p.1[0]);
                    Self::emit(cx, lp.m2.as_ptr(), p.1[1]);
                    Self::emit(cx, lp.m3.as_ptr(), p.1[2]);
                    Self::emit(cx, lp.m4.as_ptr(), p.1[3]);
                    Self::emit(cx, lp.m5.as_ptr(), p.1[4]);
                    Self::emit(cx, lp.m6.as_ptr(), p.1[5]);
                    Self::emit(cx, lp.opt_a.as_ptr(), if p.2 { 1.0 } else { 0.0 });
                    Self::emit(cx, lp.opt_b.as_ptr(), if p.3 { 1.0 } else { 0.0 });
                }
            }
            LaneEngineType::Off => {}
        }
    }

    pub(super) fn cycle_lane_engine(&self, cx: &mut EventContext, lane: Lane) {
        let lp = self.params.lane_params(lane);
        let current = lp.engine.value();
        // Toggle: Off (0) <-> OpenWurli (1)
        let next = if current == 0 { 1 } else { 0 };
        Self::emit(cx, lp.engine.as_ptr(), lp.engine.preview_normalized(next));
        let next_engine = LaneEngineType::from_index(next);
        self.apply_lane_preset(cx, lane, next_engine, 0);
    }

    pub(super) fn step_lane_preset(&self, cx: &mut EventContext, lane: Lane, delta: i32) {
        let lp = self.params.lane_params(lane);
        let engine_type = LaneEngineType::from_index(lp.engine.value());
        let total = match engine_type {
            LaneEngineType::OpenWurli => WURLI_PRESETS.len() as i32,
            LaneEngineType::Off => 1,
        };
        let current = lp.preset.value();
        let next = (current + delta).rem_euclid(total);
        self.apply_lane_preset(cx, lane, engine_type, next as usize);
    }

    pub(super) fn press_sound(&mut self, cx: &mut EventContext, x: f32, y: f32) -> bool {
        // Audition button
        if hit(SOUND_AUDITION, x, y) {
            if self.auditioning.get() {
                self.auditioning.set(false);
                self.bridge.send(Command::ReleaseKeyboard);
            } else {
                self.auditioning.set(true);
                // Audition with current keyboard chord or middle C
                let key = self.current_chord_key().unwrap_or(0);
                if let Some(chord) = self.keyboard_chord(key) {
                    let octave = self.params.keyboard_octave.value();
                    self.bridge.send(Command::KeyDown(
                        key as u8 + POINTER_KEY_OFFSET,
                        (octave + chord.root as i32) as u8,
                        chord.quality,
                    ));
                }
            }
            return true;
        }

        // Latch audition button
        if hit(SOUND_LATCH, x, y) {
            let next_latch = !self.audition_latch.get();
            self.audition_latch.set(next_latch);
            if !next_latch && self.auditioning.get() {
                self.auditioning.set(false);
                self.bridge.send(Command::ReleaseKeyboard);
            }
            return true;
        }

        // Reset all engines to default presets
        if hit(SOUND_RESET_ALL, x, y) {
            for &lane in &Lane::ALL {
                let default_engine = LaneEngineType::OpenWurli;
                let lp = self.params.lane_params(lane);
                Self::emit(cx, lp.engine.as_ptr(), lp.engine.preview_normalized(default_engine.to_index()));
                Self::emit(cx, lp.level.as_ptr(), lp.level.preview_normalized(0.8));
                Self::emit(cx, lp.pan.as_ptr(), lp.pan.preview_normalized(0.0));
                Self::emit(cx, lp.mute.as_ptr(), 0.0);
                let preset_idx = match lane {
                    Lane::Bass => 3,
                    Lane::Comp => 0,
                    Lane::Arp => 2,
                    Lane::Lead => 5,
                };
                self.apply_lane_preset(cx, lane, default_engine, preset_idx);
            }
            return true;
        }

        // Per-lane buttons
        for &lane in &Lane::ALL {
            let i = lane.index();

            // Engine cycler
            if hit(lane_engine_pill_rect(i), x, y) {
                self.cycle_lane_engine(cx, lane);
                return true;
            }

            // Preset prev / next
            let lp = self.params.lane_params(lane);
            if lp.engine.value() != 0 {
                if hit(lane_preset_prev_rect(i), x, y) {
                    self.step_lane_preset(cx, lane, -1);
                    return true;
                }
                if hit(lane_preset_next_rect(i), x, y) {
                    self.step_lane_preset(cx, lane, 1);
                    return true;
                }
            }
        }

        false
    }
}
