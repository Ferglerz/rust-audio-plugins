use super::*;
use nih_plug_vizia::vizia::vg::Color;
use pleasant_ui::theme::{rgb, GOLD, LINE, MUTED, PANEL, TEAL, TEXT};
use pleasant_ui::{Draw, KnobLayout};

// Color Palette for Nopia MK1 Hardware Aesthetic fused with Pleasant UI Themes
pub(super) const SAGE: Color = rgb(197, 212, 192);
#[allow(dead_code)]
pub(super) const SAGE_DARK: Color = rgb(176, 193, 171);
pub(super) const SAGE_BORDER: Color = rgb(124, 147, 119);
pub(super) const BIRCH_1: Color = rgb(218, 191, 157);
pub(super) const BIRCH_2: Color = rgb(190, 155, 115);
pub(super) const BIRCH_DARK: Color = rgb(140, 106, 70);
pub(super) const OLED_BG: Color = rgb(10, 14, 11);
pub(super) const OLED_BEZEL: Color = rgb(18, 24, 20);
#[allow(dead_code)]
pub(super) const OLED_CYAN: Color = TEAL;
#[allow(dead_code)]
pub(super) const LED_GOLD: Color = GOLD;
pub(super) const LED_AMBER: Color = rgb(245, 158, 11);
pub(super) const LED_PURPLE: Color = rgb(192, 132, 252);
pub(super) const LED_GREEN: Color = rgb(34, 197, 94);
pub(super) const CHERRY_WHITE: Color = rgb(250, 251, 249);
pub(super) const CHERRY_DARK: Color = rgb(27, 34, 29);
pub(super) const ENGRAVED_TEXT: Color = rgb(59, 77, 61);
pub(super) const KNOB_BODY: Color = rgb(245, 247, 243);

pub(super) const CONSOLE_W: f32 = 1040.0;
pub(super) const CONSOLE_H: f32 = 840.0;

pub(super) fn console_x_offset() -> f32 {
    ((W - CONSOLE_W) * 0.5).max(0.0)
}

// --- Unified Geometry Functions for All Interactive Controls ---

// Top Deck Geometry
pub(super) fn console_view_pill_rect(x0: f32) -> Rect { (x0 + CONSOLE_W - 148.0, 23.0, 116.0, 22.0) }
pub(super) fn fx_drive_rect(x0: f32) -> Rect { (x0 + 44.0, 14.0, 36.0, 44.0) }
pub(super) fn fx_space_rect(x0: f32) -> Rect { (x0 + 88.0, 14.0, 36.0, 44.0) }
pub(super) fn fx_button_rect(x0: f32, i: usize) -> Rect { (x0 + 138.0 + (i as f32 * 46.0), 23.0, 40.0, 23.0) }
pub(super) fn looper_rec_rect(x0: f32) -> Rect { let loop_cx = x0 + 380.0; (loop_cx + 56.0, 23.0, 30.0, 23.0) }
pub(super) fn looper_bar_rect(x0: f32, i: usize) -> Rect { let loop_cx = x0 + 380.0; (loop_cx + 92.0 + (i as f32 * 66.0), 23.0, 62.0, 23.0) }
pub(super) fn strum_knob_rect(x0: f32) -> Rect { let strum_x = x0 + 734.0; (strum_x, 14.0, 36.0, 44.0) }
pub(super) fn strum_plate_rect(x0: f32) -> Rect { let strum_x = x0 + 734.0; (strum_x + 48.0, 22.0, 106.0, 25.0) }

// Column 1 Geometry
pub(super) fn col1_oled_rect(x0: f32) -> Rect { (x0 + 40.0, 80.0, 112.0, 86.0) }
pub(super) fn col1_octave_rect(x0: f32) -> Rect { (x0 + 26.0, 214.0, 36.0, 44.0) }
pub(super) fn static_real_toggle_rect(x0: f32) -> Rect { (x0 + 34.0, 276.0, 20.0, 30.0) }
pub(super) fn sound_knob_rect(x0: f32) -> Rect { (x0 + 26.0, 326.0, 36.0, 44.0) }
pub(super) fn open_reg_toggle_rect(x0: f32) -> Rect { (x0 + 34.0, 390.0, 20.0, 30.0) }
pub(super) fn cherry_white_key_rect(x0: f32, i: usize) -> Rect {
    let kb_x = x0 + 70.0;
    let kb_y = 380.0;
    let key_w = 256.0 / 7.0;
    (kb_x + 2.0 + (i as f32 * key_w), kb_y + 2.0, key_w - 1.0, 136.0)
}
pub(super) fn cherry_black_key_rect(x0: f32, j: usize) -> Rect {
    let kb_x = x0 + 70.0;
    let kb_y = 380.0;
    let key_w = 256.0 / 7.0;
    let offsets = [key_w * 0.75, key_w * 1.75, key_w * 3.75, key_w * 4.75, key_w * 5.75];
    (kb_x + 2.0 + offsets[j] - 10.0, kb_y + 2.0, 20.0, 84.0)
}
pub(super) fn degree_shift_rect(x0: f32) -> Rect {
    let kb_x = x0 + 70.0;
    let kb_y = 380.0;
    (kb_x + 170.0, kb_y + 150.0, 88.0, 24.0)
}

// Column 2 Geometry
pub(super) fn arp_loop_once_rect(x0: f32) -> Rect { let col2_x = x0 + 356.0; (col2_x + 44.0, 89.0, 52.0, 20.0) }
pub(super) fn arp_sync_free_rect(x0: f32) -> Rect { let col2_x = x0 + 356.0; (col2_x + 100.0, 89.0, 52.0, 20.0) }
pub(super) fn arp_octave_rect(x0: f32, oct: usize) -> Rect {
    let rack_x = x0 + 356.0;
    (rack_x + 156.0 + ((oct - 1) as f32 * 14.0), 89.0, 13.0, 20.0)
}
pub(super) fn arp_rate_rect(x0: f32, i: usize) -> Rect {
    let rack_x = x0 + 356.0;
    let r_btn_w = 160.0 / 8.0;
    (rack_x + 6.0 + (i as f32 * r_btn_w), 118.0, r_btn_w - 1.5, 14.0)
}
pub(super) fn arp_pattern_rect(x0: f32, i: usize) -> Rect {
    let rack_x = x0 + 356.0;
    let c_tile_w = 204.0 / 7.0;
    (rack_x + 6.0 + (i as f32 * c_tile_w), 136.0, c_tile_w - 2.0, 28.0)
}
pub(super) fn arp_accent_step_rect(x0: f32, s: usize) -> Rect {
    let rack_x = x0 + 356.0;
    let step_w = 200.0 / 16.0;
    (rack_x + 8.0 + (s as f32 * step_w), 168.0, step_w, 24.0)
}
pub(super) fn swing_knob_rect(x0: f32) -> Rect { let col2_x = x0 + 356.0; (col2_x + 6.0, 192.0, 36.0, 44.0) }
pub(super) fn gate_knob_rect(x0: f32) -> Rect { let col2_x = x0 + 356.0; (col2_x + 46.0, 192.0, 36.0, 44.0) }
pub(super) fn comp_interlock_rect(x0: f32) -> Rect { let col2_x = x0 + 356.0; (col2_x + 160.0, 198.0, 48.0, 20.0) }
pub(super) fn hero_extensions_knob_rect(x0: f32) -> Rect {
    let hero_cx = x0 + 356.0 + 108.0;
    let hero_cy = 320.0;
    (hero_cx - 44.0, hero_cy - 44.0, 88.0, 88.0)
}
pub(super) fn spread_dial_rect(x0: f32) -> Rect { let col2_x = x0 + 356.0; (col2_x + 16.0, 458.0, 44.0, 52.0) }
pub(super) fn voice_leading_dial_rect(x0: f32) -> Rect { let col2_x = x0 + 356.0; (col2_x + 120.0, 458.0, 44.0, 52.0) }

// Column 3 Geometry
pub(super) fn sub_knob_rect(x0: f32) -> Rect { let col3_x = x0 + 590.0; (col3_x + 4.0, 82.0, 36.0, 44.0) }
pub(super) fn root_alt_toggle_rect(x0: f32) -> Rect { let col3_x = x0 + 590.0; (col3_x + 46.0, 88.0, 20.0, 30.0) }
pub(super) fn bass_pad_alt_rect(x0: f32) -> Rect { let col3_x = x0 + 590.0; (col3_x + 84.0, 84.0, 26.0, 18.0) }
pub(super) fn bass_pad_root_rect(x0: f32) -> Rect { let col3_x = x0 + 590.0; (col3_x + 84.0, 106.0, 26.0, 18.0) }
pub(super) fn pad_bank_toggle_rect(x0: f32) -> Rect { let col3_x = x0 + 590.0; (col3_x + 164.0, 88.0, 20.0, 30.0) }
pub(super) fn pad_crossfader_rect(x0: f32) -> Rect { let col3_x = x0 + 590.0; (col3_x + 196.0, 90.0, 96.0, 26.0) }
pub(super) fn roller_wheel_rect(x0: f32) -> Rect { let col3_x = x0 + 590.0; (col3_x + 380.0, 82.0, 24.0, 44.0) }
pub(super) fn white_voice_node(x0: f32, i: usize) -> (f32, f32, f32) {
    let col3_x = x0 + 590.0;
    let white_spacing = 368.0 / 6.0;
    (col3_x + 24.0 + (i as f32 * white_spacing), 239.0, 15.0)
}
pub(super) fn black_voice_node(x0: f32, j: usize) -> (f32, f32, f32) {
    let col3_x = x0 + 590.0;
    let white_spacing = 368.0 / 6.0;
    let offsets = [0.5, 1.5, 3.5, 4.5, 5.5];
    (col3_x + 24.0 + (offsets[j] * white_spacing), 194.0, 14.0)
}
pub(super) fn pitch_bend_rail_rect(x0: f32) -> Rect { let col3_x = x0 + 590.0; (col3_x + 12.0, 260.0, 392.0, 18.0) }

// Bottom Deck Geometry
pub(super) fn bottom_deck_pill_rect(x0: f32) -> Rect { (x0 + 26.0, 594.0, 180.0, 20.0) }
pub(super) fn bottom_loop_rail_rect(x0: f32) -> Rect { let deck_x = x0 + 14.0; (deck_x + 12.0, 633.0, 988.0, 16.0) }
pub(super) fn bottom_4octave_white_key_rect(x0: f32, i: usize) -> Rect {
    let deck_x = x0 + 14.0;
    let wk_w = 984.0 / 28.0;
    (deck_x + 14.0 + (i as f32 * wk_w), 653.0, wk_w - 1.0, 130.0)
}
pub(super) fn bottom_4octave_black_key_rect(x0: f32, j: usize) -> Rect {
    let deck_x = x0 + 14.0;
    let wk_w = 984.0 / 28.0;
    let oct = j / 5;
    let b_idx = j % 5;
    let offsets = [wk_w * 0.7, wk_w * 1.75, wk_w * 3.7, wk_w * 4.7, wk_w * 5.75];
    let bk_w = wk_w * 0.6;
    let kx = deck_x + 14.0 + (oct as f32 * 7.0 * wk_w) + offsets[b_idx] - (bk_w * 0.5);
    (kx, 653.0, bk_w, 82.0)
}
pub(super) fn harp_string_rect(x0: f32, i: usize) -> Rect {
    let deck_x = x0 + 14.0;
    let str_w = 964.0 / 12.0;
    (deck_x + 24.0 + (i as f32 * str_w), 616.0, str_w, 136.0)
}

// Token IDs for Live Console interactive key commands (must be < KEY_TOKEN_COUNT = 128)
const CONSOLE_TOKEN_CHERRY_WHITE: u8 = POINTER_KEY_OFFSET; // 64..70 (7 keys)
const CONSOLE_TOKEN_CHERRY_BLACK: u8 = POINTER_KEY_OFFSET + 7; // 71..75 (5 keys)
const CONSOLE_TOKEN_BASS_ROOT: u8 = POINTER_KEY_OFFSET + 12; // 76
const CONSOLE_TOKEN_BASS_ALT: u8 = POINTER_KEY_OFFSET + 13; // 77
const CONSOLE_TOKEN_VOICE_NODE: u8 = POINTER_KEY_OFFSET + 14; // 78..89 (12 nodes)
const CONSOLE_TOKEN_BOTTOM_KEY: u8 = POINTER_KEY_OFFSET + 26; // 90..127 (38 keys)

impl ChordboardView {
    /// Renders the complete Nopia MK1 Universal Live Performance Console powered by Pleasant UI.
    pub(super) fn draw_live_console(&self, d: &mut Draw) {
        let x0 = console_x_offset();

        // 1. Draw Layered Birch Plywood Cheeks
        self.draw_birch_cheeks(d, x0);

        // 2. Draw Main Pistachio Sage Faceplate
        d.rounded_rect(x0 + 14.0, 0.0, CONSOLE_W - 28.0, CONSOLE_H, 10.0, SAGE);
        d.outline_rounded(x0 + 14.0, 0.0, CONSOLE_W - 28.0, CONSOLE_H, 10.0, SAGE_BORDER, 1.5);

        // 3. Section 1: Top Row Deck (68px)
        self.draw_top_deck(d, x0);

        // 4. Section 2: Main Hardware Performance Surface (520px)
        self.draw_main_surface_columns(d, x0);

        // 5. Section 3: Bottom Interactive Deck (205px)
        self.draw_bottom_deck_live(d, x0);

        // 6. Section 4: Status Footer (47px)
        self.draw_footer_live(d, x0);

        // 7. Dynamic Parameter HUD Overlay (if active)
        self.draw_oled_hud_overlay(d, x0);
    }

    /// Paints the physical layered birch plywood side cheeks.
    fn draw_birch_cheeks(&self, d: &mut Draw, x0: f32) {
        let cheek_w = x0 + 14.0;
        let right_x = x0 + CONSOLE_W - 14.0;
        let right_w = W - right_x;

        // Base plywood slabs
        d.rect(0.0, 0.0, cheek_w, CONSOLE_H, BIRCH_1);
        d.rect(right_x, 0.0, right_w, CONSOLE_H, BIRCH_1);

        // Horizontal veneer layers (3px rhythm)
        let mut y = 0.0;
        let mut layer = 0;
        while y < CONSOLE_H {
            let color = match layer % 3 {
                0 => BIRCH_1,
                1 => BIRCH_2,
                _ => BIRCH_DARK,
            };
            d.rect(0.0, y, cheek_w, 3.0, color);
            d.rect(right_x, y, right_w, 3.0, color);
            y += 3.0;
            layer += 1;
        }

        // Inner shadow dividing cheeks from aluminum faceplate
        d.line(cheek_w, 0.0, cheek_w, CONSOLE_H, alpha(rgb(0, 0, 0), 0.35), 2.0);
        d.line(right_x, 0.0, right_x, CONSOLE_H, alpha(rgb(0, 0, 0), 0.35), 2.0);
    }

    /// Paints Top Row Deck: FX chain, Looper, Strum plate, and [ STUDIO | LIVE CONSOLE ] view switch pill.
    fn draw_top_deck(&self, d: &mut Draw, x0: f32) {
        d.line(x0 + 14.0, 68.0, x0 + CONSOLE_W - 14.0, 68.0, alpha(SAGE_BORDER, 0.4), 1.0);

        // Top Left: FX Section
        d.text(x0 + 26.0, 40.0, "FX", 11.0, ENGRAVED_TEXT);
        let drive_norm = self.params.fx_drive.value();
        let dr_r = fx_drive_rect(x0);
        let dr_val = format!("{:.0}%", drive_norm * 100.0);
        self.draw_mini_knob(d, dr_r.0 + dr_r.2 * 0.5, dr_r.1, "DRIVE", &dr_val, drive_norm, TEAL);

        let space_norm = self.params.fx_space.value();
        let sp_r = fx_space_rect(x0);
        let sp_val = format!("{:.0}%", space_norm * 100.0);
        self.draw_mini_knob(d, sp_r.0 + sp_r.2 * 0.5, sp_r.1, "SPACE", &sp_val, space_norm, TEAL);

        // 3 Square FX Buttons (Tape, Delay, Reverb)
        let fx_states = [self.params.fx_tape.value(), self.params.fx_delay.value(), self.params.fx_reverb.value()];
        for (i, label) in ["TAPE", "DELAY", "REVERB"].iter().enumerate() {
            let br = fx_button_rect(x0, i);
            self.button(d, br, label, fx_states[i], TEAL);
        }

        // Top Center: Looper Section
        let loop_cx = x0 + 380.0;
        d.text(loop_cx, 39.0, "LOOPER", 11.0, ENGRAVED_TEXT);
        // Rec button
        let rec_active = self.params.looper_rec.value();
        let rec_r = looper_rec_rect(x0);
        self.button(d, rec_r, if rec_active { "● REC" } else { "REC" }, rec_active, Color::rgb(239, 68, 68));

        // 3 Loop Memory bars
        let active_slot = self.params.looper_slot.value() as usize;
        for (i, title) in ["1: VERSE", "2: CHORUS", "3: BRIDGE"].iter().enumerate() {
            let lr = looper_bar_rect(x0, i);
            self.button(d, lr, title, i == active_slot, GOLD);
        }

        // Top Right: Capacitive Strum Plate & Dual View Switcher Pill
        let strum_norm = self.params.strum_ms.preview_normalized(self.params.strum_ms.value());
        let st_r = strum_knob_rect(x0);
        let st_val = format!("{:.0}ms", self.params.strum_ms.value());
        self.draw_mini_knob(d, st_r.0 + st_r.2 * 0.5, st_r.1, "STRUM", &st_val, strum_norm, GOLD);

        // Capacitive Strum copper trace
        let sp = strum_plate_rect(x0);
        d.rounded_rect(sp.0, sp.1, sp.2, sp.3, 4.0, PANEL);
        d.outline_rounded(sp.0, sp.1, sp.2, sp.3, 4.0, LINE, 1.0);
        for line_idx in 0..11 {
            let lx = sp.0 + 8.0 + (line_idx as f32 * (sp.2 - 16.0) / 10.0);
            d.line(lx, sp.1 + 4.0, lx, sp.1 + sp.3 - 4.0, alpha(GOLD, 0.45), 1.5);
        }
        d.line(sp.0 + 4.0, sp.1 + sp.3 * 0.5, sp.0 + sp.2 - 4.0, sp.1 + sp.3 * 0.5, GOLD, 1.5);
        d.text(sp.0 + 6.0, sp.1 + 9.0, "STRUM", 5.5, MUTED);
        if matches!(self.drag, Some(Drag::Pad)) || self.snapshot.pressure > 0.05 {
            let touch_x = sp.0 + (self.snapshot.x * sp.2).clamp(6.0, sp.2 - 6.0);
            d.circle(touch_x, sp.1 + sp.3 * 0.5, 6.0, GOLD, true);
            d.circle(touch_x, sp.1 + sp.3 * 0.5, 9.0, alpha(GOLD, 0.4), true);
        }

        // View Mode Pill: [ STUDIO | LIVE CONSOLE ]
        let pill = console_view_pill_rect(x0);
        let is_live = self.params.console_view.value() == 1;
        let half_w = (pill.2 - 2.0) * 0.5;
        let studio_r = (pill.0, pill.1, half_w, pill.3);
        let live_r = (pill.0 + half_w + 2.0, pill.1, half_w, pill.3);
        self.button(d, studio_r, "STUDIO", !is_live, TEAL);
        self.button(d, live_r, "LIVE", is_live, GOLD);
    }

    /// Paints the 3 columns of the main hardware console surface.
    fn draw_main_surface_columns(&self, d: &mut Draw, x0: f32) {
        let col1_x = x0 + 32.0;
        let col2_x = x0 + 356.0;
        let col3_x = x0 + 590.0;

        // Dividing vertical seams
        d.line(col2_x - 12.0, 78.0, col2_x - 12.0, 578.0, alpha(SAGE_BORDER, 0.25), 1.0);
        d.line(col3_x - 12.0, 78.0, col3_x - 12.0, 578.0, alpha(SAGE_BORDER, 0.25), 1.0);

        // Column 1: Left Hand Deck
        self.draw_col1_keys_builder(d, col1_x, x0);

        // Column 2: Center Arp & Hero Extensions Deck
        self.draw_col2_arp_hero(d, col2_x, x0);

        // Column 3: Right Hand Performance & Circular Piano Nodes Deck
        self.draw_col3_performance(d, col3_x, x0);
    }

    /// Column 1: 6-Box Modular OLED Display, mechanical Cherry keybed, degree shift, flank toggles.
    fn draw_col1_keys_builder(&self, d: &mut Draw, x: f32, x0: f32) {
        // 6-Box Modular OLED Display
        self.draw_6box_oled(d, x + 8.0, 80.0);

        // Octave Knob
        let oct_r = col1_octave_rect(x0);
        let oct_val = (self.params.keyboard_octave.value() - 12) as f32 / (96.0 - 12.0);
        let oct_name = match self.params.keyboard_octave.value() {
            12 => "C0",
            24 => "C1",
            36 => "C2",
            48 => "C3",
            60 => "C4",
            72 => "C5",
            84 => "C6",
            96 => "C7",
            _ => "OCT",
        };
        self.draw_mini_knob(d, oct_r.0 + oct_r.2 * 0.5, oct_r.1, "OCT", oct_name, oct_val, GOLD);

        // Flank Toggles & Controls
        let sr_toggle = static_real_toggle_rect(x0);
        self.draw_toggle_switch(d, sr_toggle.0, sr_toggle.1, "STATIC\nREAL", self.params.harmonization_mode.value() != 0);

        let filter_norm = self.params.filter.preview_normalized(self.params.filter.value());
        let snd_r = sound_knob_rect(x0);
        let filter_hz = self.params.filter.value();
        let val_str = if filter_hz >= 1000 { format!("{:.1}k", filter_hz as f32 / 1000.0) } else { format!("{filter_hz}Hz") };
        self.draw_mini_knob(d, snd_r.0 + snd_r.2 * 0.5, snd_r.1, "FILTER", &val_str, filter_norm, TEAL);

        let open_reg = self.params.open_register.value();
        let open_r = open_reg_toggle_rect(x0);
        self.draw_toggle_switch(d, open_r.0, open_r.1, "OPEN\nREG", open_reg);

        // 1-Octave Mechanical Cherry Keybed Chassis
        let kb_x = x0 + 70.0;
        let kb_y = 380.0;
        d.rounded_rect(kb_x, kb_y, 260.0, 140.0, 6.0, PANEL);
        d.outline_rounded(kb_x, kb_y, 260.0, 140.0, 6.0, LINE, 2.0);

        // Degree Shift Keycap
        let shift_active = self.params.degree_shift.value();
        let key_tonic = self.params.key.value() as u8;

        // 7 White Keys
        let white_labels = ["I", "II", "III", "IV", "V", "VI", "VII"];
        let degree_pitches = [0u8, 2, 4, 5, 7, 9, 11];
        for i in 0..7 {
            let kr = cherry_white_key_rect(x0, i);
            let root_pitch = (key_tonic + degree_pitches[i]) % 12;
            let qual = if shift_active { 5 } else { [3, 1, 1, 3, 2, 1, 4][i] };
            let qual_name = match qual {
                1 => "m7",
                2 => "7",
                3 => "maj7",
                4 => "m7b5",
                5 => "sus4",
                _ => "",
            };
            let sub_name = format!("{}{}", self.note_name(root_pitch), qual_name);
            let active = matches!(self.drag, Some(Drag::Key(k)) if k == CONSOLE_TOKEN_CHERRY_WHITE + i as u8)
                || (self.snapshot.root >= 0 && (self.snapshot.root.rem_euclid(12) as u8) == root_pitch);
            d.rounded_rect(kr.0, kr.1, kr.2, kr.3, 3.0, if active { TEXT } else { CHERRY_WHITE });
            d.outline_rounded(kr.0, kr.1, kr.2, kr.3, 3.0, if active { GOLD } else { LINE }, if active { 1.5 } else { 1.0 });
            d.text_centered(kr.0 + kr.2 * 0.5, kr.1 + 112.0, white_labels[i], 8.5, if active { PANEL } else { rgb(75, 97, 78) });
            d.text_centered(kr.0 + kr.2 * 0.5, kr.1 + 126.0, &sub_name, 6.5, if active { GOLD } else { rgb(114, 139, 112) });
        }

        // 5 Black Keys
        let black_labels = ["V/V", "SubV", "viio/V", "bVI", "bVII"];
        let black_pitches = [1u8, 3, 6, 8, 10];
        for j in 0..5 {
            let kr = cherry_black_key_rect(x0, j);
            let root_pitch = (key_tonic + black_pitches[j]) % 12;
            let active = matches!(self.drag, Some(Drag::Key(k)) if k == CONSOLE_TOKEN_CHERRY_BLACK + j as u8)
                || (self.snapshot.root >= 0 && (self.snapshot.root.rem_euclid(12) as u8) == root_pitch);
            d.rounded_rect(kr.0, kr.1, kr.2, kr.3, 2.0, if active { rgb(54, 70, 56) } else { CHERRY_DARK });
            d.outline_rounded(kr.0, kr.1, kr.2, kr.3, 2.0, if active { GOLD } else { LINE }, if active { 1.5 } else { 1.0 });
            d.text_centered(kr.0 + kr.2 * 0.5, kr.1 + 70.0, black_labels[j], 6.0, if active { GOLD } else { rgb(114, 139, 112) });
        }

        // Degree Shift Button
        let sr = degree_shift_rect(x0);
        self.button(d, sr, if shift_active { "SUS4 ACTIVE" } else { "DEGREE SHIFT" }, shift_active, GOLD);
    }

    /// 6-Box Modular OLED Assistant Display.
    fn draw_6box_oled(&self, d: &mut Draw, x: f32, y: f32) {
        d.rounded_rect(x, y, 112.0, 86.0, 6.0, OLED_BG);
        d.outline_rounded(x, y, 112.0, 86.0, 6.0, OLED_BEZEL, 2.0);

        // Derive active chord from live engine performance or captured snapshot
        let active_chord = if self.snapshot.root >= 0 {
            Some(SavedChord {
                root: (self.snapshot.root.rem_euclid(12)) as u8,
                second: (self.snapshot.second >= 0).then_some(self.snapshot.second as u8),
                quality: self.snapshot.quality.min(11),
                inversion: self.snapshot.inversion.min(5),
                spread: (self.params.spread.value() as u8).min(crate::harmony::VOICING_NAMES.len().saturating_sub(1) as u8),
                transpose: self.params.transpose.value() as i8,
            })
        } else {
            SavedChord::decode(self.snapshot.captured)
        };

        let (root_pitch, quality, second) = if let Some(chord) = active_chord {
            let p = (chord.root as i16 + chord.transpose as i16).rem_euclid(12) as u8;
            (p, chord.quality, chord.second)
        } else if self.snapshot.notes.len > 0 {
            (self.snapshot.notes.values[0] % 12, 0, None)
        } else {
            (self.params.key.value() as u8, 3, None)
        };

        // Note name spelling in key
        let full_note_name = self.note_name(root_pitch);
        let (root_letter, accidental) = if full_note_name.len() > 1 {
            (&full_note_name[..1], &full_note_name[1..])
        } else {
            (full_note_name.as_str(), "")
        };

        // Quality suffix
        let q_base = match quality {
            0 => "maj",
            1 => "m",
            2 => "7",
            3 => "maj7",
            4 => "m7",
            5 => "dim",
            6 => "aug",
            7 => "6",
            8 => "m6",
            9 => "dim7",
            10 => "m7b5",
            11 => "5",
            _ => "",
        };
        let qual_str = if let Some(sec) = second {
            let interval = (sec as i16 - root_pitch as i16).rem_euclid(12);
            if interval == 2 {
                "sus2"
            } else if interval == 5 {
                "sus4"
            } else {
                q_base
            }
        } else {
            q_base
        };

        // Extension & Tension from macro and harmonic structure
        let ext_val = self.params.extensions.value();
        let ext_str = if ext_val < 0.20 {
            "R"
        } else if ext_val < 0.40 {
            "1+5"
        } else if ext_val < 0.50 {
            "TRI"
        } else if ext_val < 0.85 {
            "7"
        } else {
            "9+"
        };

        let tension_str = if let Some(sec) = second {
            let interval = (sec as i16 - root_pitch as i16).rem_euclid(12);
            match interval {
                1 => "b9",
                2 => "9",
                3 => "#9",
                6 => "#11",
                8 => "b13",
                9 => "13",
                _ => "—",
            }
        } else if ext_val >= 0.85 {
            "9"
        } else {
            "—"
        };

        // Box 6: Slash bass
        let slash_str = if let Some(bass_pitch) = self.snapshot.bass_note {
            let bp = bass_pitch % 12;
            if bp != root_pitch {
                format!("/{}", self.note_name(bp))
            } else {
                format!("/{}", root_letter)
            }
        } else if let Some(min_note) = self.snapshot.notes.as_slice().iter().copied().min() {
            let bp = min_note % 12;
            if bp != root_pitch {
                format!("/{}", self.note_name(bp))
            } else {
                format!("/{}", root_letter)
            }
        } else {
            format!("/{}", root_letter)
        };

        // Box 1: Root (Huge)
        d.text(x + 8.0, y + 54.0, root_letter, 36.0, TEXT);
        // Box 2: Accidental
        if !accidental.is_empty() {
            d.text(x + 44.0, y + 36.0, accidental, 20.0, TEXT);
        }
        // Box 4: Ext Top
        d.text_right(x + 104.0, y + 20.0, ext_str, 11.0, TEAL);
        // Box 5: Tension
        d.text_right(x + 104.0, y + 38.0, tension_str, 12.0, MUTED);
        // Box 3: Quality
        d.text_right(x + 104.0, y + 56.0, qual_str, 14.0, TEXT);
        // Box 6: Slash bass
        d.text_right(x + 104.0, y + 74.0, &slash_str, 12.0, GOLD);
    }

    /// Column 2: Center Arp Rack, Hero Extensions Knob, Density Gauge, Voicing Dials.
    fn draw_col2_arp_hero(&self, d: &mut Draw, x: f32, x0: f32) {
        let rack_w = 216.0;
        let rack_y = 80.0;
        d.rounded_rect(x, rack_y, rack_w, 168.0, 6.0, alpha(PANEL, 0.4));
        d.outline_rounded(x, rack_y, rack_w, 168.0, 6.0, LINE, 1.0);

        // Row 1: Mini OLED, Loop/Once, Sync/Free, Octaves
        d.rounded_rect(x + 6.0, rack_y + 6.0, 36.0, 28.0, 3.0, OLED_BG);
        d.outline_rounded(x + 6.0, rack_y + 6.0, 36.0, 28.0, 3.0, LINE, 1.0);
        d.text(x + 8.0, rack_y + 16.0, "S:1/8", 5.5, TEAL);
        d.text_centered(x + 24.0, rack_y + 26.0, "♫", 9.0, TEXT);

        // Loop / Once buttons
        let is_loop = self.params.mode.value() == 3;
        let lo_rect = arp_loop_once_rect(x0);
        let half_lo = (lo_rect.2 - 2.0) * 0.5;
        let loop_btn = (lo_rect.0, lo_rect.1, half_lo, lo_rect.3);
        let once_btn = (lo_rect.0 + half_lo + 2.0, lo_rect.1, half_lo, lo_rect.3);
        self.button(d, loop_btn, "LOOP", is_loop, TEAL);
        self.button(d, once_btn, "ONCE", !is_loop, TEAL);

        // Sync / Free buttons
        let is_sync = self.params.tempo_sync.value();
        let sf_rect = arp_sync_free_rect(x0);
        let half_sf = (sf_rect.2 - 2.0) * 0.5;
        let sync_btn = (sf_rect.0, sf_rect.1, half_sf, sf_rect.3);
        let free_btn = (sf_rect.0 + half_sf + 2.0, sf_rect.1, half_sf, sf_rect.3);
        self.button(d, sync_btn, "SYNC", is_sync, GOLD);
        self.button(d, free_btn, "FREE", !is_sync, GOLD);

        // Octaves 1..4
        for oct in 1..=4 {
            let or = arp_octave_rect(x0, oct);
            let active = oct as i32 == self.params.octaves.value();
            self.button(d, or, &oct.to_string(), active, GOLD);
        }

        // Row 1.5: 8-Rate Division Strip
        let cur_rate = self.params.rate.value();
        for (i, (r_name, beats)) in ARP_RATES.iter().enumerate() {
            let rr = arp_rate_rect(x0, i);
            let active = (cur_rate - *beats).abs() < 0.001;
            self.button(d, rr, r_name, active, GOLD);
        }

        // Row 2: 7 Pattern Contour Glyphs
        let active_pat = self.params.arp_pattern.value() as usize;
        let pat_names = ["UP", "DOWN", "UP/DN", "UP 3RD", "DN 3RD", "PLAYED", "RANDOM"];
        for i in 0..7 {
            let pr = arp_pattern_rect(x0, i);
            let active = i == active_pat;
            self.button(d, pr, pat_names[i], active, GOLD);
        }

        // Row 3: 16-Step Adaptive Accent Lane
        let acc_y = rack_y + 88.0;
        d.rounded_rect(x + 6.0, acc_y, rack_w - 12.0, 24.0, 3.0, PANEL);
        d.outline_rounded(x + 6.0, acc_y, rack_w - 12.0, 24.0, 3.0, LINE, 1.0);
        let playhead = (self.snapshot.arp_cycle_step % 16) as usize;
        for s in 0..16 {
            let sr = arp_accent_step_rect(x0, s);
            let accent = self.params.step_accent(s);
            let is_cur = s == playhead;
            if is_cur {
                d.rounded_rect(sr.0, sr.1, sr.2, sr.3, 2.0, alpha(TEAL, 0.35));
            }
            match accent {
                1 => d.rounded_rect(sr.0 + 2.0, sr.1 + 5.0, 3.5, 12.0, 1.0, GOLD),
                2 => d.rounded_rect(sr.0 + 2.0, sr.1 + 11.0, 3.5, 5.0, 1.0, MUTED),
                _ => d.rounded_rect(sr.0 + 2.0, sr.1 + 14.0, 3.5, 2.5, 1.0, alpha(LINE, 0.6)),
            }
        }

        // Row 4: Groove, Gate & Anti-Collision Interlock Strip
        let grv_y = rack_y + 116.0;
        let swing_norm = self.params.swing.value();
        let sw_r = swing_knob_rect(x0);
        let sw_val = format!("{:.0}%", swing_norm * 100.0);
        self.draw_mini_knob(d, sw_r.0 + sw_r.2 * 0.5, sw_r.1, "SWING", &sw_val, swing_norm, GOLD);

        let gate_norm = self.params.gate.value();
        let gt_r = gate_knob_rect(x0);
        let gt_val = format!("{:.0}%", gate_norm * 100.0);
        self.draw_mini_knob(d, gt_r.0 + gt_r.2 * 0.5, gt_r.1, "GATE", &gt_val, gate_norm, GOLD);

        // 8-step groove visualizer
        d.rounded_rect(x + 84.0, grv_y + 4.0, 72.0, 14.0, 2.0, OLED_BG);
        for i in 0..8 {
            let gbx = x + 86.0 + (i as f32 * 8.6);
            d.rounded_rect(gbx, grv_y + 6.0, 7.0, 10.0, 1.0, PANEL);
            d.rounded_rect(gbx, grv_y + 6.0, 5.5, 10.0, 1.0, TEAL);
        }

        // Interlock Pill
        let lock_active = self.params.comp_interlock.value() > 0;
        let lock_rect = comp_interlock_rect(x0);
        self.button(d, lock_rect, "LOCK", lock_active, GOLD);

        // Hero Extensions Knob & Density Gauge
        let hero_cx = x + (rack_w * 0.5);
        let hero_cy = 320.0;
        let ext_val = self.params.extensions.value();
        let hero_layout = KnobLayout::custom(
            hero_cx,
            hero_cy,
            30.0,
            hero_cy - 40.0,
            hero_cy + 42.0,
            9.5,
            8.5,
            hero_cy + 46.0,
        );
        let color_tier = if ext_val < 0.25 { "TRIAD" } else if ext_val < 0.5 { "7th BASE" } else if ext_val < 0.75 { "9th COLOR" } else { "11th/13th" };
        d.knob_with_layout(&hero_layout, "EXTENSIONS", color_tier, ext_val, GOLD, false);

        // Density Meter Gauge
        let d_y = 420.0;
        d.rounded_rect(x + 14.0, d_y, rack_w - 28.0, 26.0, 4.0, PANEL);
        d.outline_rounded(x + 14.0, d_y, rack_w - 28.0, 26.0, 4.0, LINE, 1.0);
        d.text(x + 20.0, d_y + 11.0, "DENSITY", 7.0, MUTED);
        d.text_right(x + rack_w - 20.0, d_y + 11.0, color_tier, 7.0, TEXT);
        // Fill track
        d.rounded_rect(x + 20.0, d_y + 14.0, rack_w - 40.0, 5.0, 2.5, LINE);
        d.rounded_rect(x + 20.0, d_y + 14.0, (rack_w - 40.0) * ext_val, 5.0, 2.5, GOLD);

        // Voicing & Leading Knobs
        let spread_idx = self.params.spread.value() as usize;
        let spread_label = crate::harmony::VOICING_NAMES.get(spread_idx).copied().unwrap_or("CLOSE");
        let spread_norm = spread_idx as f32 / (crate::harmony::VOICING_NAMES.len() - 1).max(1) as f32;
        let sp_r = spread_dial_rect(x0);
        self.draw_std_knob(d, sp_r.0 + sp_r.2 * 0.5, sp_r.1, "SPREAD", spread_label, spread_norm, TEAL);

        let vl_idx = self.params.voice_leading.value() as usize;
        let vl_label = ["OFF", "NEAREST", "STRICT"].get(vl_idx).copied().unwrap_or("OFF");
        let vl_norm = vl_idx as f32 / 2.0;
        let vl_r = voice_leading_dial_rect(x0);
        self.draw_std_knob(d, vl_r.0 + vl_r.2 * 0.5, vl_r.1, "VOICE LDG", vl_label, vl_norm, TEAL);
    }

    /// Column 3: Two Square Bass Pads, Pad Module (Bank LEDs + Crossfader), 12 Circular Voice Nodes.
    fn draw_col3_performance(&self, d: &mut Draw, x: f32, x0: f32) {
        let p_w = 416.0;

        // Bass & Pad Row
        let row_y = 80.0;
        d.rounded_rect(x, row_y, p_w, 48.0, 6.0, alpha(PANEL, 0.4));
        d.outline_rounded(x, row_y, p_w, 48.0, 6.0, LINE, 1.0);

        // Bass controls: Sub knob + Toggle + Two Square Bass Pads
        let sub_norm = self.params.bass_velocity.value();
        let sb_r = sub_knob_rect(x0);
        let sub_val = format!("{:.0}%", sub_norm * 100.0);
        self.draw_mini_knob(d, sb_r.0 + sb_r.2 * 0.5, sb_r.1, "SUB BASS", &sub_val, sub_norm, GOLD);

        let bass_trigger = self.params.bass_pad_trigger.value();
        let ra_r = root_alt_toggle_rect(x0);
        self.draw_toggle_switch(d, ra_r.0, ra_r.1, "ROOT\nALT", bass_trigger == 1);

        // TWO SQUARE BASS PADS
        let bass_active_root = matches!(self.drag, Some(Drag::Key(k)) if k == CONSOLE_TOKEN_BASS_ROOT)
            || (self.snapshot.bass_note.is_some() && bass_trigger == 0);
        let bass_active_alt = matches!(self.drag, Some(Drag::Key(k)) if k == CONSOLE_TOKEN_BASS_ALT)
            || (self.snapshot.bass_note.is_some() && bass_trigger == 1);

        let alt_rect = bass_pad_alt_rect(x0);
        self.button(d, alt_rect, "ALT", bass_active_alt, if bass_active_alt { GOLD } else { TEAL });

        let root_rect = bass_pad_root_rect(x0);
        self.button(d, root_rect, "ROOT", bass_active_root, GOLD);

        // Pad module: Bank A (Amber) + Toggle + Bank B (Purple) + Crossfader
        let bank = self.params.bank.value();
        let bk_r = pad_bank_toggle_rect(x0);
        d.circle(bk_r.0 - 18.0, row_y + 24.0, 5.5, if bank == 0 { LED_AMBER } else { alpha(LED_AMBER, 0.3) }, true);
        self.draw_toggle_switch(d, bk_r.0, bk_r.1, "", bank == 1);
        d.circle(bk_r.0 + 24.0, row_y + 24.0, 5.5, if bank == 1 { LED_PURPLE } else { alpha(LED_PURPLE, 0.3) }, true);

        // Horizontal crossfader track & thumb
        let cf_norm = self.params.pad_crossfade.value();
        let cf_r = pad_crossfader_rect(x0);
        d.rounded_rect(cf_r.0, row_y + 20.0, 76.0, 8.0, 4.0, PANEL);
        d.outline_rounded(cf_r.0, row_y + 20.0, 76.0, 8.0, 4.0, LINE, 1.0);
        let thumb_x = cf_r.0 + (cf_norm * 62.0);
        d.rounded_rect(thumb_x, row_y + 14.0, 14.0, 20.0, 3.0, TEXT);
        d.outline_rounded(thumb_x, row_y + 14.0, 14.0, 20.0, 3.0, GOLD, 1.0);

        // Vertical Roller Wheel
        let wheel_x = x + p_w - 32.0;
        let mod_norm = self.params.mod_wheel.value();
        d.rounded_rect(wheel_x, row_y + 4.0, 20.0, 40.0, 8.0, OLED_BG);
        d.outline_rounded(wheel_x, row_y + 4.0, 20.0, 40.0, 8.0, LINE, 1.0);
        let roller_thumb_y = row_y + 8.0 + ((1.0 - mod_norm) * 16.0);
        d.rounded_rect(wheel_x + 3.0, roller_thumb_y, 14.0, 16.0, 4.0, TEXT);

        // 12 CIRCULAR PIANO NODES
        let circ_deck_y = 144.0;
        d.rounded_rect(x, circ_deck_y, p_w, 140.0, 8.0, PANEL);
        d.outline_rounded(x, circ_deck_y, p_w, 140.0, 8.0, LINE, 2.0);
        d.text(x + 12.0, circ_deck_y + 18.0, "VOICE NODES (PIANO MATRIX)", 7.5, MUTED);
        d.text_right(x + p_w - 12.0, circ_deck_y + 18.0, "REACTIVE CHORD TONES", 7.5, TEXT);

        // 7 White Nodes (Bottom Row: C, D, E, F, G, A, B)
        let white_tones = ["C", "D", "E", "F", "G", "A", "B"];
        let white_pitches = [0u8, 2, 4, 5, 7, 9, 11];
        for i in 0..7 {
            let (cx, cy, r) = white_voice_node(x0, i);
            let p = white_pitches[i];
            let is_held_drag = matches!(self.drag, Some(Drag::Key(k)) if k == CONSOLE_TOKEN_VOICE_NODE + i as u8);
            let is_sounding = is_held_drag || (0..128).any(|n| (n % 12 == p) && (self.snapshot.sounding_notes[n as usize] || self.snapshot.held_notes[n as usize]));
            let is_chord_tone = self.snapshot.notes.as_slice().iter().any(|&n| n % 12 == p)
                || (self.snapshot.root >= 0 && (self.snapshot.root.rem_euclid(12) as u8) == p);
            let is_arp_step = self.snapshot.notes.len > 0
                && (self.snapshot.notes.values[self.snapshot.arp_cycle_step % self.snapshot.notes.len] % 12 == p);

            let (fill_col, stroke_col, text_col) = if is_arp_step && (is_sounding || self.params.mode.value() == 3) {
                (TEAL, TEXT, PANEL)
            } else if is_sounding {
                (GOLD, TEXT, PANEL)
            } else if is_chord_tone {
                (TEXT, LINE, PANEL)
            } else {
                (PANEL, LINE, MUTED)
            };

            d.circle(cx, cy, r, fill_col, true);
            d.circle(cx, cy, r, stroke_col, false);
            d.text_centered(cx, cy + 3.0, white_tones[i], 9.0, text_col);
        }

        // 5 Black Nodes (Top Row: C#, D#, [gap], F#, G#, A#)
        let black_tones = ["C#", "D#", "F#", "G#", "A#"];
        let black_pitches = [1u8, 3, 6, 8, 10];
        for j in 0..5 {
            let (cx, cy, r) = black_voice_node(x0, j);
            let p = black_pitches[j];
            let is_held_drag = matches!(self.drag, Some(Drag::Key(k)) if k == CONSOLE_TOKEN_VOICE_NODE + 7 + j as u8);
            let is_sounding = is_held_drag || (0..128).any(|n| (n % 12 == p) && (self.snapshot.sounding_notes[n as usize] || self.snapshot.held_notes[n as usize]));
            let is_chord_tone = self.snapshot.notes.as_slice().iter().any(|&n| n % 12 == p)
                || (self.snapshot.root >= 0 && (self.snapshot.root.rem_euclid(12) as u8) == p);
            let is_arp_step = self.snapshot.notes.len > 0
                && (self.snapshot.notes.values[self.snapshot.arp_cycle_step % self.snapshot.notes.len] % 12 == p);

            let (fill_col, stroke_col, text_col) = if is_arp_step && (is_sounding || self.params.mode.value() == 3) {
                (TEAL, TEXT, PANEL)
            } else if is_sounding {
                (GOLD, TEXT, PANEL)
            } else if is_chord_tone {
                (TEXT, LINE, PANEL)
            } else {
                (rgb(20, 26, 21), LINE, MUTED)
            };

            d.circle(cx, cy, r, fill_col, true);
            d.circle(cx, cy, r, stroke_col, false);
            d.text_centered(cx, cy + 3.0, black_tones[j], 8.0, text_col);
        }

        // Bottom pitch bend / vibrato touch rail
        let rail_y = circ_deck_y + 118.0;
        let pb_val = self.params.pitch_bend.value();
        d.rounded_rect(x + 12.0, rail_y, p_w - 24.0, 14.0, 7.0, OLED_BG);
        d.line(x + 20.0, rail_y + 7.0, x + p_w - 20.0, rail_y + 7.0, LINE, 2.0);
        let pb_center_x = x + (p_w * 0.5) + (pb_val * ((p_w - 40.0) * 0.45));
        d.circle(pb_center_x, rail_y + 7.0, 5.0, GOLD, true);
    }

    /// Section 3: Bottom Deck (4-Octave Keys vs Physical Strings Harp).
    fn draw_bottom_deck_live(&self, d: &mut Draw, x0: f32) {
        let deck_y = 588.0;
        let deck_w = CONSOLE_W - 28.0;
        let x = x0 + 14.0;

        d.rect(x, deck_y, deck_w, 205.0, PANEL);
        d.line(x, deck_y, x + deck_w, deck_y, LINE, 2.0);

        let is_harp = self.params.bottom_deck_mode.value() == 1;
        let dp = bottom_deck_pill_rect(x0);
        let half_dp = (dp.2 - 4.0) * 0.5;
        let keys_btn = (dp.0, dp.1, half_dp, dp.3);
        let harp_btn = (dp.0 + half_dp + 4.0, dp.1, half_dp, dp.3);
        self.button(d, keys_btn, "🎹 4-OCT KEYS", !is_harp, TEAL);
        self.button(d, harp_btn, "𝄢 STRINGS HARP", is_harp, GOLD);

        if is_harp {
            self.draw_harp_matrix_live(d, x, deck_y, x0);
        } else {
            self.draw_piano_roll_live(d, x, deck_y, x0);
        }
    }

    /// Mode A: 4-Octave Piano Roll with 28 White Keys, 20 Black Keys, Zone Splits & Arp Loop Rail.
    fn draw_piano_roll_live(&self, d: &mut Draw, x: f32, deck_y: f32, x0: f32) {
        let deck_w = CONSOLE_W - 28.0;

        // Zone Split Bar
        let split_y = deck_y + 28.0;
        d.rounded_rect(x + 12.0, split_y, deck_w - 24.0, 14.0, 2.0, OLED_BG);
        d.rect(x + 12.0, split_y, 240.0, 14.0, alpha(GOLD, 0.25));
        d.text(x + 20.0, split_y + 10.0, "BASS ZONE (C2 - B2)", 6.5, GOLD);
        d.rect(x + 252.0, split_y, 360.0, 14.0, alpha(TEAL, 0.25));
        d.text(x + 260.0, split_y + 10.0, "CHORD TRIGGER (C3 - B4)", 6.5, TEAL);
        d.rect(x + 612.0, split_y, deck_w - 624.0, 14.0, alpha(LED_PURPLE, 0.25));
        d.text(x + 620.0, split_y + 10.0, "LEAD ZONE (C5 - B5)", 6.5, LED_PURPLE);

        // Arp Loop Rail with S and E tags
        let loop_y = deck_y + 45.0;
        d.rounded_rect(x + 12.0, loop_y, deck_w - 24.0, 14.0, 2.0, OLED_BG);
        d.line(x + 20.0, loop_y + 7.0, x + deck_w - 20.0, loop_y + 7.0, LINE, 2.0);
        let s_val = (self.params.loop_start.value() as f32 / 32.0).clamp(0.0, 1.0);
        let e_val = if self.params.loop_end.value() == 0 { 1.0 } else { (self.params.loop_end.value() as f32 / 32.0).clamp(0.0, 1.0) };
        let s_x = x + 20.0 + (s_val * (deck_w - 40.0));
        let e_x = x + 20.0 + (e_val * (deck_w - 40.0));
        d.line(s_x, loop_y + 7.0, e_x, loop_y + 7.0, GOLD, 2.0);
        d.rounded_rect(s_x - 6.0, loop_y + 1.0, 12.0, 12.0, 2.0, GOLD);
        d.text_centered(s_x, loop_y + 9.5, "S", 7.0, PANEL);
        d.rounded_rect(e_x - 6.0, loop_y + 1.0, 12.0, 12.0, 2.0, GOLD);
        d.text_centered(e_x, loop_y + 9.5, "E", 7.0, PANEL);

        // 4-Octave Piano Bed Canvas
        let keys_y = deck_y + 63.0;
        let keys_h = 134.0;
        d.rounded_rect(x + 12.0, keys_y, deck_w - 24.0, keys_h, 3.0, rgb(0, 0, 0));

        // 1. Layer 1: 28 White Keys
        for i in 0..28 {
            let kr = bottom_4octave_white_key_rect(x0, i);
            let oct = i / 7;
            let w_idx = i % 7;
            let note = (36 + (oct as i32 * 12) + [0, 2, 4, 5, 7, 9, 11][w_idx]).clamp(0, 127) as u8;
            let is_held_drag = matches!(self.drag, Some(Drag::Key(k)) if k == CONSOLE_TOKEN_BOTTOM_KEY + (note % 38));
            let is_sounding = is_held_drag || self.snapshot.sounding_notes[note as usize] || self.snapshot.held_notes[note as usize];
            let is_chord_note = self.snapshot.notes.as_slice().contains(&note) || self.snapshot.full_notes.as_slice().contains(&note);
            let is_chord_pitch = self.snapshot.notes.as_slice().iter().any(|&n| n % 12 == note % 12);
            let is_arp_step = self.snapshot.notes.len > 0
                && (self.snapshot.notes.values[self.snapshot.arp_cycle_step % self.snapshot.notes.len] == note);

            let col = if is_arp_step {
                TEAL
            } else if is_sounding {
                GOLD
            } else if is_chord_note {
                rgb(186, 230, 253)
            } else if is_chord_pitch {
                rgb(235, 245, 233)
            } else {
                rgb(228, 234, 226)
            };

            d.rounded_rect(kr.0, kr.1, kr.2, kr.3, 2.0, col);
            if is_sounding || is_chord_note || is_arp_step {
                let badge_col = if is_arp_step { TEAL } else if is_sounding { GOLD } else { TEAL };
                d.rounded_rect(kr.0 + 2.0, kr.1 + kr.3 - 16.0, kr.2 - 4.0, 12.0, 2.0, alpha(badge_col, 0.85));
                d.text_centered(kr.0 + kr.2 * 0.5, kr.1 + kr.3 - 7.0, &self.note_name(note), 6.0, TEXT);
            }
        }

        // 2. Layer 2: 20 Black Keys (Accidentals on top)
        for j in 0..20 {
            let br = bottom_4octave_black_key_rect(x0, j);
            let oct = j / 5;
            let b_idx = j % 5;
            let note = (36 + (oct as i32 * 12) + [1, 3, 6, 8, 10][b_idx]).clamp(0, 127) as u8;
            let is_held_drag = matches!(self.drag, Some(Drag::Key(k)) if k == CONSOLE_TOKEN_BOTTOM_KEY + (note % 38));
            let is_sounding = is_held_drag || self.snapshot.sounding_notes[note as usize] || self.snapshot.held_notes[note as usize];
            let is_chord_note = self.snapshot.notes.as_slice().contains(&note) || self.snapshot.full_notes.as_slice().contains(&note);
            let is_chord_pitch = self.snapshot.notes.as_slice().iter().any(|&n| n % 12 == note % 12);
            let is_arp_step = self.snapshot.notes.len > 0
                && (self.snapshot.notes.values[self.snapshot.arp_cycle_step % self.snapshot.notes.len] == note);

            let (col, border_col) = if is_arp_step {
                (rgb(2, 132, 199), TEXT)
            } else if is_sounding {
                (GOLD, TEXT)
            } else if is_chord_note {
                (rgb(14, 116, 144), TEAL)
            } else if is_chord_pitch {
                (rgb(38, 52, 42), LINE)
            } else {
                (CHERRY_DARK, rgb(0, 0, 0))
            };

            d.rounded_rect(br.0, br.1, br.2, br.3, 2.0, col);
            d.outline_rounded(br.0, br.1, br.2, br.3, 2.0, border_col, 1.0);
        }
    }

    /// Mode B: Physical Strings Harp Matrix (12 Strings with Deflection Wave & Volume Pegs).
    fn draw_harp_matrix_live(&self, d: &mut Draw, x: f32, deck_y: f32, x0: f32) {
        let deck_w = CONSOLE_W - 28.0;
        let harp_y = deck_y + 28.0;
        let harp_h = 168.0;

        d.rounded_rect(x + 12.0, harp_y, deck_w - 24.0, harp_h, 4.0, OLED_BG);
        d.outline_rounded(x + 12.0, harp_y, deck_w - 24.0, harp_h, 4.0, LINE, 1.0);

        for i in 0..12 {
            let hr = harp_string_rect(x0, i);
            let sx = hr.0 + (hr.2 * 0.5);
            let note_opt = self.snapshot.notes.as_slice().get(i).copied()
                .or_else(|| self.snapshot.full_notes.as_slice().get(i).copied());
            let is_muted = self.params.is_string_muted(i);
            let is_skipped = self.params.is_string_skipped(i);
            let sounding = note_opt.map_or(false, |n| self.snapshot.sounding_notes[n as usize] || self.snapshot.held_notes[n as usize]);
            let wire_col = if is_skipped {
                alpha(MUTED, 0.3)
            } else if is_muted {
                alpha(LED_AMBER, 0.4)
            } else if sounding {
                GOLD
            } else {
                alpha(TEAL, 0.45)
            };
            let stroke_w = if sounding { 2.5 } else { 1.5 };
            d.line(sx, harp_y + 12.0, sx, harp_y + harp_h - 32.0, wire_col, stroke_w);

            let vol = self.params.string_volume(i);
            let peg_y = harp_y + 16.0 + (1.0 - vol) * (harp_h - 48.0);
            d.rounded_rect(sx - 7.0, peg_y, 14.0, 7.0, 2.0, if sounding { GOLD } else if is_muted { LED_AMBER } else { KNOB_BODY });

            let note_str = note_opt.map(|n| self.note_name(n)).unwrap_or_else(|| format!("#{}", i + 1));
            d.text_centered(sx, harp_y + harp_h - 18.0, &note_str, 8.0, if sounding { GOLD } else if is_skipped { MUTED } else { TEXT });
            d.rounded_rect(sx - 9.0, harp_y + harp_h - 13.0, 18.0, 10.0, 2.0, if is_muted { rgb(80, 50, 10) } else if is_skipped { PANEL } else { rgb(24, 34, 26) });
            let badge_text = if is_skipped { "SKP" } else if is_muted { "MUT" } else { &format!("#{}", i + 1) };
            d.text_centered(sx, harp_y + harp_h - 6.0, badge_text, 6.0, if is_muted { LED_AMBER } else if is_skipped { MUTED } else { TEAL });
        }
    }

    /// Section 4: Status Footer.
    fn draw_footer_live(&self, d: &mut Draw, x0: f32) {
        let y = 793.0;
        let x = x0 + 14.0;
        let w = CONSOLE_W - 28.0;

        d.rect(x, y, w, 47.0, OLED_BG);
        d.line(x, y, x + w, y, LINE, 1.0);

        d.rounded_rect(x + 18.0, y + 14.0, 220.0, 18.0, 3.0, PANEL);
        d.text(x + 24.0, y + 26.0, "4 MIDI LANES: KEYS · BASS · ARP · PAD", 7.5, MUTED);

        d.rounded_rect(x + 246.0, y + 14.0, 170.0, 18.0, 3.0, PANEL);
        let spread_idx = self.params.spread.value() as usize;
        let spread_label = crate::harmony::VOICING_NAMES.get(spread_idx).copied().unwrap_or("CLOSE");
        let vl_idx = self.params.voice_leading.value() as usize;
        let vl_label = ["OFF", "NEAREST", "STRICT"].get(vl_idx).copied().unwrap_or("OFF");
        d.text(x + 252.0, y + 26.0, &format!("VOICING: {} · {}", spread_label, vl_label), 7.5, MUTED);

        d.text_right(x + w - 18.0, y + 26.0, "DSP: 0.1ms | LATENCY: 64 SAMPLES", 8.0, LED_GREEN);
    }

    /// Dynamic parameter HUD overlay across the OLED screen.
    fn draw_oled_hud_overlay(&self, d: &mut Draw, x0: f32) {
        let x = x0 + 40.0;
        let y = 80.0;
        if !self.status.is_empty() && self.status.contains(':') {
            d.rounded_rect(x, y, 112.0, 86.0, 6.0, OLED_BG);
            d.outline_rounded(x, y, 112.0, 86.0, 6.0, LINE, 1.5);
            d.text_centered(x + 56.0, y + 36.0, "PARAMETER", 8.0, TEAL);
            d.text_centered(x + 56.0, y + 56.0, &self.status, 11.0, TEXT);
        }
    }

    // --- Helper Drawing Primitives for Controls Using Pleasant UI ---

    fn draw_mini_knob(
        &self,
        d: &mut Draw,
        cx: f32,
        cy: f32,
        label: &str,
        value: &str,
        norm: f32,
        color: Color,
    ) {
        let layout = KnobLayout::custom(
            cx,
            cy + 13.0,
            11.0,
            cy + 0.0,
            cy + 33.0,
            7.0,
            6.5,
            cy + 37.0,
        );
        d.knob_with_layout(&layout, label, value, norm, color, false);
    }

    fn draw_std_knob(
        &self,
        d: &mut Draw,
        cx: f32,
        cy: f32,
        label: &str,
        value: &str,
        norm: f32,
        color: Color,
    ) {
        let layout = KnobLayout::custom(
            cx,
            cy + 16.0,
            15.0,
            cy + 1.0,
            cy + 41.0,
            7.8,
            7.2,
            cy + 45.0,
        );
        d.knob_with_layout(&layout, label, value, norm, color, false);
    }

    fn draw_toggle_switch(&self, d: &mut Draw, x: f32, y: f32, caption: &str, down: bool) {
        let w = 20.0;
        let h = 30.0;
        d.rounded_rect(x, y, w, h, 6.0, PANEL);
        d.outline_rounded(x, y, w, h, 6.0, LINE, 1.0);
        let bat_y = if down { y + 16.0 } else { y + 3.0 };
        let bat_col = if down { TEAL } else { MUTED };
        d.rounded_rect(x + 2.5, bat_y, w - 5.0, 11.0, 3.0, if down { TEXT } else { rgb(40, 50, 42) });
        d.circle(x + w * 0.5, bat_y + 5.5, 2.5, bat_col, true);
        if !caption.is_empty() {
            let lines: Vec<&str> = caption.split('\n').collect();
            for (i, line) in lines.iter().enumerate() {
                d.text_centered(x + w * 0.5, y + h + 8.0 + (i as f32 * 8.0), line, 6.0, if down { TEAL } else { MUTED });
            }
        }
    }

    // --- Helper to start parameter drag for any knob ---
    fn start_drag_param(&mut self, cx: &mut EventContext, id: &'static str, ptr: ParamPtr, norm: f32, rect: Rect, x: f32, y: f32) {
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
        self.drag = Some(Drag::Control(ControlDrag {
            press: pleasant_ui::pointer::ValuePress::new(id, rect, (x, y)),
            ptr,
            norm,
            editable: false,
            started: true,
        }));
        cx.capture();
    }

    // --- Interactive Mouse Handlers for Live Console ---

    pub(super) fn handle_live_console_mouse_down(
        &mut self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
    ) -> bool {
        let x0 = console_x_offset();

        // 0. 6-Box OLED Chord Display: Left half opens Key menu, Right half opens Scale menu
        let oled_r = col1_oled_rect(x0);
        if hit(oled_r, x, y) {
            if x < oled_r.0 + oled_r.2 * 0.5 {
                self.open_menu(Menu::Key);
            } else {
                self.open_menu(Menu::Scale);
            }
            return true;
        }

        // 0b. Column 1 Octave Knob
        let oct_r = col1_octave_rect(x0);
        if hit(oct_r, x, y) {
            let cur = self.params.keyboard_octave.value();
            let norm = (cur - 12) as f32 / 84.0;
            self.start_drag_param(cx, "keyboard_octave", self.params.keyboard_octave.as_ptr(), norm, oct_r, x, y);
            return true;
        }

        // 1. Dual View Switcher Pill [ STUDIO | LIVE ]
        let pill = console_view_pill_rect(x0);
        if hit(pill, x, y) {
            let target_view = if x < pill.0 + pill.2 * 0.5 { 0 } else { 1 };
            Self::emit(
                cx,
                self.params.console_view.as_ptr(),
                self.params.console_view.preview_normalized(target_view),
            );
            return true;
        }

        // 2. Top Deck: FX Knobs & Buttons
        let dr_rect = fx_drive_rect(x0);
        if hit(dr_rect, x, y) {
            let norm = self.params.fx_drive.value();
            self.start_drag_param(cx, "fx_drive", self.params.fx_drive.as_ptr(), norm, dr_rect, x, y);
            return true;
        }
        let sp_rect = fx_space_rect(x0);
        if hit(sp_rect, x, y) {
            let norm = self.params.fx_space.value();
            self.start_drag_param(cx, "fx_space", self.params.fx_space.as_ptr(), norm, sp_rect, x, y);
            return true;
        }
        if hit(fx_button_rect(x0, 0), x, y) {
            Self::emit(cx, self.params.fx_tape.as_ptr(), if self.params.fx_tape.value() { 0.0 } else { 1.0 });
            return true;
        }
        if hit(fx_button_rect(x0, 1), x, y) {
            Self::emit(cx, self.params.fx_delay.as_ptr(), if self.params.fx_delay.value() { 0.0 } else { 1.0 });
            return true;
        }
        if hit(fx_button_rect(x0, 2), x, y) {
            Self::emit(cx, self.params.fx_reverb.as_ptr(), if self.params.fx_reverb.value() { 0.0 } else { 1.0 });
            return true;
        }

        // 3. Top Deck: Looper Section
        let rec_r = looper_rec_rect(x0);
        if hit(rec_r, x, y) {
            Self::emit(cx, self.params.looper_rec.as_ptr(), if self.params.looper_rec.value() { 0.0 } else { 1.0 });
            return true;
        }
        for i in 0..3 {
            let lr = looper_bar_rect(x0, i);
            if hit(lr, x, y) {
                Self::emit(cx, self.params.looper_slot.as_ptr(), self.params.looper_slot.preview_normalized(i as i32));
                let word = self.params.slot(i).load(Ordering::Relaxed);
                if word != 0 {
                    self.bridge.send(Command::Recall(word));
                }
                return true;
            }
        }

        // 4. Top Deck: Strum Knob & Capacitive Strum Plate
        let st_rect = strum_knob_rect(x0);
        if hit(st_rect, x, y) {
            let norm = self.params.strum_ms.preview_normalized(self.params.strum_ms.value());
            self.start_drag_param(cx, "strum_ms", self.params.strum_ms.as_ptr(), norm, st_rect, x, y);
            return true;
        }
        let sp_plate = strum_plate_rect(x0);
        if hit(sp_plate, x, y) {
            let px = ((x - sp_plate.0) / sp_plate.2).clamp(0.0, 1.0);
            self.bridge.send(Command::BeginGesture(px, 0.8));
            self.drag = Some(Drag::Pad);
            cx.capture();
            return true;
        }

        // 5. Column 1: Flank Toggles & Controls
        if hit(static_real_toggle_rect(x0), x, y) {
            let next = if self.params.harmonization_mode.value() == 0 { 1 } else { 0 };
            Self::emit(cx, self.params.harmonization_mode.as_ptr(), self.params.harmonization_mode.preview_normalized(next));
            return true;
        }
        let snd_rect = sound_knob_rect(x0);
        if hit(snd_rect, x, y) {
            let norm = self.params.filter.preview_normalized(self.params.filter.value());
            self.start_drag_param(cx, "filter", self.params.filter.as_ptr(), norm, snd_rect, x, y);
            return true;
        }
        if hit(open_reg_toggle_rect(x0), x, y) {
            let cur = self.params.open_register.value();
            Self::emit(cx, self.params.open_register.as_ptr(), if cur { 0.0 } else { 1.0 });
            let next_spread = (self.params.spread.value() + 1) % crate::harmony::VOICING_NAMES.len() as i32;
            Self::emit(cx, self.params.spread.as_ptr(), self.params.spread.preview_normalized(next_spread));
            return true;
        }
        if hit(degree_shift_rect(x0), x, y) {
            Self::emit(cx, self.params.degree_shift.as_ptr(), if self.params.degree_shift.value() { 0.0 } else { 1.0 });
            return true;
        }

        // 6. Column 1: Mechanical Cherry Keybed
        let octave = self.params.keyboard_octave.value();
        let key_tonic = self.params.key.value() as u8;
        // White keys
        let degree_pitches = [0u8, 2, 4, 5, 7, 9, 11];
        for i in 0..7 {
            let kr = cherry_white_key_rect(x0, i);
            if hit(kr, x, y) {
                let note = (octave + (key_tonic + degree_pitches[i]) as i32).clamp(12, 115) as u8;
                let key_id = CONSOLE_TOKEN_CHERRY_WHITE + i as u8;
                self.bridge.send(Command::KeyDown(key_id, note, 0));
                self.drag = Some(Drag::Key(key_id));
                cx.capture();
                return true;
            }
        }
        // Black keys
        let black_pitches = [1u8, 3, 6, 8, 10];
        for j in 0..5 {
            let kr = cherry_black_key_rect(x0, j);
            if hit(kr, x, y) {
                let note = (octave + (key_tonic + black_pitches[j]) as i32).clamp(12, 115) as u8;
                let key_id = CONSOLE_TOKEN_CHERRY_BLACK + j as u8;
                self.bridge.send(Command::KeyDown(key_id, note, 0));
                self.drag = Some(Drag::Key(key_id));
                cx.capture();
                return true;
            }
        }

        // 7. Column 2: Center Arp & Voicing
        let lo_rect = arp_loop_once_rect(x0);
        if hit(lo_rect, x, y) {
            let next_mode = if x < lo_rect.0 + lo_rect.2 * 0.5 { 3 } else { 1 };
            Self::emit(cx, self.params.mode.as_ptr(), self.params.mode.preview_normalized(next_mode));
            return true;
        }
        let sf_rect = arp_sync_free_rect(x0);
        if hit(sf_rect, x, y) {
            let next_sync = x < sf_rect.0 + sf_rect.2 * 0.5;
            Self::emit(cx, self.params.tempo_sync.as_ptr(), if next_sync { 1.0 } else { 0.0 });
            return true;
        }
        for oct in 1..=4 {
            let or = arp_octave_rect(x0, oct);
            if hit(or, x, y) {
                Self::emit(cx, self.params.octaves.as_ptr(), self.params.octaves.preview_normalized(oct as i32));
                return true;
            }
        }
        for (i, (_, beats)) in ARP_RATES.iter().enumerate() {
            let rr = arp_rate_rect(x0, i);
            if hit(rr, x, y) {
                Self::emit(cx, self.params.rate.as_ptr(), self.params.rate.preview_normalized(*beats));
                return true;
            }
        }
        for i in 0..7 {
            let pr = arp_pattern_rect(x0, i);
            if hit(pr, x, y) {
                Self::emit(cx, self.params.arp_pattern.as_ptr(), self.params.arp_pattern.preview_normalized(i as i32));
                return true;
            }
        }
        for s in 0..16 {
            let sr = arp_accent_step_rect(x0, s);
            if hit(sr, x, y) {
                let cur = self.params.step_accent(s);
                let next = match cur { 0 => 1, 1 => 2, _ => 0 };
                self.params.set_step_accent(s, next);
                cx.needs_redraw();
                return true;
            }
        }
        let sw_rect = swing_knob_rect(x0);
        if hit(sw_rect, x, y) {
            let norm = self.params.swing.value();
            self.start_drag_param(cx, "swing", self.params.swing.as_ptr(), norm, sw_rect, x, y);
            return true;
        }
        let gt_rect = gate_knob_rect(x0);
        if hit(gt_rect, x, y) {
            let norm = self.params.gate.value();
            self.start_drag_param(cx, "gate", self.params.gate.as_ptr(), norm, gt_rect, x, y);
            return true;
        }
        let lk_rect = comp_interlock_rect(x0);
        if hit(lk_rect, x, y) {
            let cur = self.params.comp_interlock.value();
            let next = if cur == 0 { 1 } else { 0 };
            Self::emit(cx, self.params.comp_interlock.as_ptr(), self.params.comp_interlock.preview_normalized(next));
            return true;
        }
        let hero_rect = hero_extensions_knob_rect(x0);
        if hit(hero_rect, x, y) {
            let norm = self.params.extensions.value();
            self.start_drag_param(cx, "extensions", self.params.extensions.as_ptr(), norm, hero_rect, x, y);
            return true;
        }
        let sp_dial = spread_dial_rect(x0);
        if hit(sp_dial, x, y) {
            let next = (self.params.spread.value() + 1) % crate::harmony::VOICING_NAMES.len() as i32;
            Self::emit(cx, self.params.spread.as_ptr(), self.params.spread.preview_normalized(next));
            self.start_drag_param(cx, "spread", self.params.spread.as_ptr(), self.params.spread.preview_normalized(next), sp_dial, x, y);
            return true;
        }
        let vl_dial = voice_leading_dial_rect(x0);
        if hit(vl_dial, x, y) {
            let next = (self.params.voice_leading.value() + 1) % 3;
            Self::emit(cx, self.params.voice_leading.as_ptr(), self.params.voice_leading.preview_normalized(next));
            self.start_drag_param(cx, "voice_leading", self.params.voice_leading.as_ptr(), self.params.voice_leading.preview_normalized(next), vl_dial, x, y);
            return true;
        }

        // 8. Column 3: Right Hand Performance Deck
        let sb_rect = sub_knob_rect(x0);
        if hit(sb_rect, x, y) {
            let norm = self.params.bass_velocity.value();
            self.start_drag_param(cx, "bass_velocity", self.params.bass_velocity.as_ptr(), norm, sb_rect, x, y);
            return true;
        }
        if hit(root_alt_toggle_rect(x0), x, y) {
            let cur = self.params.bass_pad_trigger.value();
            let next = if cur == 0 { 1 } else { 0 };
            Self::emit(cx, self.params.bass_pad_trigger.as_ptr(), self.params.bass_pad_trigger.preview_normalized(next));
            return true;
        }
        if hit(bass_pad_alt_rect(x0), x, y) {
            Self::emit(cx, self.params.bass_pad_trigger.as_ptr(), self.params.bass_pad_trigger.preview_normalized(1));
            let bass_note = (octave.saturating_sub(12) + 7).clamp(24, 72) as u8;
            let key_id = CONSOLE_TOKEN_BASS_ALT;
            self.bridge.send(Command::KeyDown(key_id, bass_note, 0));
            self.drag = Some(Drag::Key(key_id));
            cx.capture();
            return true;
        }
        if hit(bass_pad_root_rect(x0), x, y) {
            Self::emit(cx, self.params.bass_pad_trigger.as_ptr(), self.params.bass_pad_trigger.preview_normalized(0));
            let bass_note = octave.saturating_sub(12).clamp(24, 72) as u8;
            let key_id = CONSOLE_TOKEN_BASS_ROOT;
            self.bridge.send(Command::KeyDown(key_id, bass_note, 0));
            self.drag = Some(Drag::Key(key_id));
            cx.capture();
            return true;
        }
        if hit(pad_bank_toggle_rect(x0), x, y) {
            let cur = self.params.bank.value();
            let next = if cur == 0 { 1 } else { 0 };
            Self::emit(cx, self.params.bank.as_ptr(), self.params.bank.preview_normalized(next));
            return true;
        }
        let cf_rect = pad_crossfader_rect(x0);
        if hit(cf_rect, x, y) {
            let norm = ((x - cf_rect.0) / cf_rect.2).clamp(0.0, 1.0);
            Self::emit(cx, self.params.pad_crossfade.as_ptr(), norm);
            self.start_drag_param(cx, "pad_crossfade", self.params.pad_crossfade.as_ptr(), norm, cf_rect, x, y);
            return true;
        }
        let rw_rect = roller_wheel_rect(x0);
        if hit(rw_rect, x, y) {
            let norm = (1.0 - ((y - rw_rect.1) / rw_rect.3)).clamp(0.0, 1.0);
            Self::emit(cx, self.params.mod_wheel.as_ptr(), norm);
            self.start_drag_param(cx, "mod_wheel", self.params.mod_wheel.as_ptr(), norm, rw_rect, x, y);
            return true;
        }
        for (i, pitch) in [1, 3, 6, 8, 10].iter().enumerate() {
            let (cx_n, cy_n, r) = black_voice_node(x0, i);
            if (x - cx_n).hypot(y - cy_n) <= r {
                let note = (octave + pitch).clamp(12, 115) as u8;
                let key_id = CONSOLE_TOKEN_VOICE_NODE + 7 + i as u8;
                self.bridge.send(Command::KeyDown(key_id, note, 0));
                self.drag = Some(Drag::Key(key_id));
                cx.capture();
                return true;
            }
        }
        for (i, pitch) in [0, 2, 4, 5, 7, 9, 11].iter().enumerate() {
            let (cx_n, cy_n, r) = white_voice_node(x0, i);
            if (x - cx_n).hypot(y - cy_n) <= r {
                let note = (octave + pitch).clamp(12, 115) as u8;
                let key_id = CONSOLE_TOKEN_VOICE_NODE + i as u8;
                self.bridge.send(Command::KeyDown(key_id, note, 0));
                self.drag = Some(Drag::Key(key_id));
                cx.capture();
                return true;
            }
        }
        let pb_rect = pitch_bend_rail_rect(x0);
        if hit(pb_rect, x, y) {
            let norm = (((x - pb_rect.0) / pb_rect.2) * 2.0 - 1.0).clamp(-1.0, 1.0);
            Self::emit(cx, self.params.pitch_bend.as_ptr(), self.params.pitch_bend.preview_normalized(norm));
            self.start_drag_param(cx, "pitch_bend", self.params.pitch_bend.as_ptr(), self.params.pitch_bend.preview_normalized(norm), pb_rect, x, y);
            return true;
        }

        // 9. Bottom Deck: Mode Pill & Instruments
        let deck_pill = bottom_deck_pill_rect(x0);
        if hit(deck_pill, x, y) {
            let mode = if x < deck_pill.0 + deck_pill.2 * 0.5 { 0 } else { 1 };
            Self::emit(cx, self.params.bottom_deck_mode.as_ptr(), self.params.bottom_deck_mode.preview_normalized(mode));
            return true;
        }

        if self.params.bottom_deck_mode.value() == 0 {
            // Mode A: Arp Loop Rail & 4-Octave Keyboard
            let lr = bottom_loop_rail_rect(x0);
            if hit(lr, x, y) {
                let deck_w = CONSOLE_W - 28.0;
                let s_val = (self.params.loop_start.value() as f32 / 32.0).clamp(0.0, 1.0);
                let e_val = if self.params.loop_end.value() == 0 { 1.0 } else { (self.params.loop_end.value() as f32 / 32.0).clamp(0.0, 1.0) };
                let s_x = x0 + 14.0 + 20.0 + (s_val * (deck_w - 40.0));
                let e_x = x0 + 14.0 + 20.0 + (e_val * (deck_w - 40.0));
                let is_end = (x - e_x).abs() < (x - s_x).abs();
                self.drag = Some(Drag::ArpLoopBound(is_end));
                cx.capture();
                let ptr = if is_end {
                    self.params.loop_end.as_ptr()
                } else {
                    self.params.loop_start.as_ptr()
                };
                cx.emit(RawParamEvent::BeginSetParameter(ptr));
                let t = ((x - (x0 + 14.0 + 20.0)) / (deck_w - 40.0)).clamp(0.0, 1.0);
                let step = (t * 32.0).round() as i32;
                if is_end {
                    let val = step.clamp(self.params.loop_start.value() as i32, 32);
                    Self::emit(cx, self.params.loop_end.as_ptr(), self.params.loop_end.preview_normalized(val));
                } else {
                    let val = step.clamp(1, 32);
                    Self::emit(cx, self.params.loop_start.as_ptr(), self.params.loop_start.preview_normalized(val));
                }
                cx.needs_redraw();
                return true;
            }

            // Black keys first (top layer)
            for j in 0..20 {
                let br = bottom_4octave_black_key_rect(x0, j);
                if hit(br, x, y) {
                    let oct = j / 5;
                    let b_idx = j % 5;
                    let note = (36 + (oct as i32 * 12) + [1, 3, 6, 8, 10][b_idx]).clamp(0, 127) as u8;
                    let key_id = CONSOLE_TOKEN_BOTTOM_KEY + (note % 38);
                    self.bridge.send(Command::KeyDown(key_id, note, 0));
                    self.drag = Some(Drag::Key(key_id));
                    cx.capture();
                    return true;
                }
            }
            // White keys second
            for i in 0..28 {
                let kr = bottom_4octave_white_key_rect(x0, i);
                if hit(kr, x, y) {
                    let oct = i / 7;
                    let w_idx = i % 7;
                    let note = (36 + (oct as i32 * 12) + [0, 2, 4, 5, 7, 9, 11][w_idx]).clamp(0, 127) as u8;
                    let key_id = CONSOLE_TOKEN_BOTTOM_KEY + (note % 38);
                    self.bridge.send(Command::KeyDown(key_id, note, 0));
                    self.drag = Some(Drag::Key(key_id));
                    cx.capture();
                    return true;
                }
            }
        } else {
            // Mode B: 12 Harp Strings
            for i in 0..12 {
                let hr = harp_string_rect(x0, i);
                if hit(hr, x, y) {
                    self.bridge.send(Command::BeginGesture(i as f32 / 11.0, 0.8));
                    return true;
                }
            }
        }

        false
    }

    /// Handles mouse drag movements across the Live Console.
    pub(super) fn handle_live_console_mouse_move(
        &mut self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
    ) -> bool {
        let x0 = console_x_offset();

        // 1. Capacitive Strum Plate gesture drag
        if matches!(self.drag, Some(Drag::Pad)) {
            let sp = strum_plate_rect(x0);
            let px = ((x - sp.0) / sp.2).clamp(0.0, 1.0);
            self.bridge.send(Command::X(px));
            return true;
        }

        // 2. Arp Loop Rail dragging on console
        if let Some(Drag::ArpLoopBound(is_end)) = self.drag {
            let deck_w = CONSOLE_W - 28.0;
            let t = ((x - (x0 + 14.0 + 20.0)) / (deck_w - 40.0)).clamp(0.0, 1.0);
            let step = (t * 32.0).round() as i32;
            if is_end {
                let val = step.clamp(self.params.loop_start.value() as i32, 32);
                Self::emit(cx, self.params.loop_end.as_ptr(), self.params.loop_end.preview_normalized(val));
            } else {
                let val = step.clamp(1, 32);
                Self::emit(cx, self.params.loop_start.as_ptr(), self.params.loop_start.preview_normalized(val));
            }
            cx.needs_redraw();
            return true;
        }

        // 3. Direct tracking controls: Crossfader, Pitch Bend, Mod Wheel
        if let Some(Drag::Control(ref drag)) = self.drag {
            if drag.press.target == "pad_crossfade" {
                let cf_rect = pad_crossfader_rect(x0);
                let norm = ((x - cf_rect.0) / cf_rect.2).clamp(0.0, 1.0);
                Self::emit(cx, self.params.pad_crossfade.as_ptr(), norm);
                return true;
            }
            if drag.press.target == "pitch_bend" {
                let pb_rect = pitch_bend_rail_rect(x0);
                let norm = (((x - pb_rect.0) / pb_rect.2) * 2.0 - 1.0).clamp(-1.0, 1.0);
                Self::emit(cx, self.params.pitch_bend.as_ptr(), self.params.pitch_bend.preview_normalized(norm));
                return true;
            }
            if drag.press.target == "mod_wheel" {
                let rw_rect = roller_wheel_rect(x0);
                let norm = (1.0 - ((y - rw_rect.1) / rw_rect.3)).clamp(0.0, 1.0);
                Self::emit(cx, self.params.mod_wheel.as_ptr(), norm);
                return true;
            }
        }

        // 4. Harp string volume peg dragging
        if self.params.bottom_deck_mode.value() == 1 && cx.mouse().left.state == MouseButtonState::Pressed {
            for i in 0..12 {
                let hr = harp_string_rect(x0, i);
                if (x >= hr.0) && (x <= hr.0 + hr.2) {
                    let vol = (1.0 - ((y - (616.0 + 16.0)) / 88.0)).clamp(0.0, 1.0);
                    self.params.set_string_volume(i, vol);
                    cx.needs_redraw();
                    return true;
                }
            }
        }

        false
    }

    /// Handles mouse button release across the Live Console.
    pub(super) fn handle_live_console_mouse_up(
        &mut self,
        cx: &mut EventContext,
        _x: f32,
        _y: f32,
    ) -> bool {
        // Pitch bend returns to center on release
        if self.params.pitch_bend.value().abs() > 0.001 {
            Self::emit(cx, self.params.pitch_bend.as_ptr(), self.params.pitch_bend.preview_normalized(0.0));
        }

        // Strum plate gesture ends
        if matches!(self.drag, Some(Drag::Pad)) {
            self.bridge.send(Command::EndGesture);
            self.drag = None;
            cx.release();
            return true;
        }

        false
    }

    /// Handles right-click interactions on the Live Console (e.g. node mute/skip).
    pub(super) fn handle_live_console_right_click(
        &mut self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
    ) -> bool {
        let x0 = console_x_offset();

        for i in 0..5 {
            let (cx_n, cy_n, r) = black_voice_node(x0, i);
            if (x - cx_n).hypot(y - cy_n) <= r {
                self.params.toggle_string_skipped(i + 7);
                cx.needs_redraw();
                return true;
            }
        }
        for i in 0..7 {
            let (cx_n, cy_n, r) = white_voice_node(x0, i);
            if (x - cx_n).hypot(y - cy_n) <= r {
                self.params.toggle_string_skipped(i);
                cx.needs_redraw();
                return true;
            }
        }

        false
    }

    /// Handles double-click interactions (e.g. reset parameters or volumes).
    pub(super) fn handle_live_console_double_click(
        &mut self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
    ) -> bool {
        let x0 = console_x_offset();

        // Double click on Harp string resets volume to 1.0
        if self.params.bottom_deck_mode.value() == 1 {
            for i in 0..12 {
                let hr = harp_string_rect(x0, i);
                if hit(hr, x, y) {
                    self.params.set_string_volume(i, 1.0);
                    cx.needs_redraw();
                    return true;
                }
            }
        }

        // Double click on bottom loop rail resets loop start to 1 and loop end to 0 (full span)
        if self.params.bottom_deck_mode.value() == 0 && hit(bottom_loop_rail_rect(x0), x, y) {
            self.params.reset_all_arp_loop();
            Self::emit(cx, self.params.loop_start.as_ptr(), self.params.loop_start.preview_normalized(1));
            Self::emit(cx, self.params.loop_end.as_ptr(), self.params.loop_end.preview_normalized(0));
            cx.needs_redraw();
            return true;
        }

        // Double click on Pitch Bend resets to 0.0
        if hit(pitch_bend_rail_rect(x0), x, y) {
            Self::emit(cx, self.params.pitch_bend.as_ptr(), self.params.pitch_bend.preview_normalized(0.0));
            return true;
        }

        // Double click on Crossfader resets to center 0.5
        if hit(pad_crossfader_rect(x0), x, y) {
            Self::emit(cx, self.params.pad_crossfade.as_ptr(), 0.5);
            return true;
        }

        false
    }

    /// Handles mouse scroll events over the Live Console.
    pub(super) fn handle_live_console_scroll(
        &mut self,
        cx: &mut EventContext,
        x: f32,
        y: f32,
        dy: f32,
    ) -> bool {
        let x0 = console_x_offset();
        let step = if dy > 0.0 { 1.0 } else { -1.0 };

        // 1. Hero Extensions knob
        let hero = hero_extensions_knob_rect(x0);
        if hit(hero, x, y) {
            let next = (self.params.extensions.value() + step * 0.05).clamp(0.0, 1.0);
            Self::emit(cx, self.params.extensions.as_ptr(), next);
            return true;
        }

        // 2. Spread dial
        let sp = spread_dial_rect(x0);
        if hit(sp, x, y) {
            let max_sp = (crate::harmony::VOICING_NAMES.len() - 1) as i32;
            let next = (self.params.spread.value() + step as i32).clamp(0, max_sp);
            Self::emit(cx, self.params.spread.as_ptr(), self.params.spread.preview_normalized(next));
            return true;
        }

        // 3. Voice Leading dial
        let vl = voice_leading_dial_rect(x0);
        if hit(vl, x, y) {
            let next = (self.params.voice_leading.value() + step as i32).clamp(0, 2);
            Self::emit(cx, self.params.voice_leading.as_ptr(), self.params.voice_leading.preview_normalized(next));
            return true;
        }

        // 4. Swing knob
        let sw = swing_knob_rect(x0);
        if hit(sw, x, y) {
            let next = (self.params.swing.value() + step * 0.05).clamp(0.0, 1.0);
            Self::emit(cx, self.params.swing.as_ptr(), next);
            return true;
        }

        // 5. Gate knob
        let gt = gate_knob_rect(x0);
        if hit(gt, x, y) {
            let next = (self.params.gate.value() + step * 0.05).clamp(0.0, 1.0);
            Self::emit(cx, self.params.gate.as_ptr(), next);
            return true;
        }

        // 6. Sub bass knob
        let sb = sub_knob_rect(x0);
        if hit(sb, x, y) {
            let next = (self.params.bass_velocity.value() + step * 0.05).clamp(0.0, 1.0);
            Self::emit(cx, self.params.bass_velocity.as_ptr(), next);
            return true;
        }

        // 7. Filter Sound knob
        let snd = sound_knob_rect(x0);
        if hit(snd, x, y) {
            let next = (self.params.filter.value() + (step * 200.0) as i32).clamp(20, 20000);
            Self::emit(cx, self.params.filter.as_ptr(), self.params.filter.preview_normalized(next));
            return true;
        }

        // 8. Strum knob
        let st = strum_knob_rect(x0);
        if hit(st, x, y) {
            let next = (self.params.strum_ms.value() + step * 5.0).clamp(0.0, 500.0);
            Self::emit(cx, self.params.strum_ms.as_ptr(), self.params.strum_ms.preview_normalized(next));
            return true;
        }

        // 9. FX Drive & Space
        let dr = fx_drive_rect(x0);
        if hit(dr, x, y) {
            let next = (self.params.fx_drive.value() + step * 0.05).clamp(0.0, 1.0);
            Self::emit(cx, self.params.fx_drive.as_ptr(), next);
            return true;
        }
        let sp_fx = fx_space_rect(x0);
        if hit(sp_fx, x, y) {
            let next = (self.params.fx_space.value() + step * 0.05).clamp(0.0, 1.0);
            Self::emit(cx, self.params.fx_space.as_ptr(), next);
            return true;
        }

        // 10. Pad Crossfader
        let cf = pad_crossfader_rect(x0);
        if hit(cf, x, y) {
            let next = (self.params.pad_crossfade.value() + step * 0.05).clamp(0.0, 1.0);
            Self::emit(cx, self.params.pad_crossfade.as_ptr(), next);
            return true;
        }

        // 11. Roller Wheel
        let rw = roller_wheel_rect(x0);
        if hit(rw, x, y) {
            let next = (self.params.mod_wheel.value() + step * 0.05).clamp(0.0, 1.0);
            Self::emit(cx, self.params.mod_wheel.as_ptr(), next);
            return true;
        }

        // 12. Octave in Column 1
        let oct_r = col1_octave_rect(x0);
        if hit(oct_r, x, y) {
            let next = (self.params.keyboard_octave.value() + step as i32 * 12).clamp(12, 96);
            Self::emit(cx, self.params.keyboard_octave.as_ptr(), self.params.keyboard_octave.preview_normalized(next));
            return true;
        }

        false
    }
}
