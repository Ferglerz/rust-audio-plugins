//! Absolute control positions — mirrors `04_UI_Controls/03_control_layout.jsfx-inc`.
//!
//! JSFX `define_group` + `set_control_group` stores control x/y **relative** to the
//! group origin; rendering adds `group_defs[idx*6+0/1]`. Coordinates are computed
//! in JSFX gfx space, then mapped to the 0.32 BG asset via `theme::sx()` (JSFX blits
//! BG at 0.29 horizontally — see `theme::LAYOUT_X_SCALE`).

use super::theme;

const UI_PANEL_X: f32 = 25.0;
const SLIDER_H: f32 = 27.0;
const FILTER_GAP: f32 = 6.0;
const BTN_ADJ_PREVIEW_W: f32 = 44.0;
const BTN_SC_W: f32 = 30.0;
const LOOKAHEAD_GAP: f32 = 10.0;
const BRICKWALL_BTN_W: f32 = 80.0;
const PROG_GAP: f32 = 10.0;
const CURVE_ROW_GAP: f32 = 15.0;
const TOOLBAR_PAD: f32 = 10.0;
const TOOLBAR_GAP: f32 = 10.0;
const TOOLBAR_KNOB_GAP: f32 = 5.0;
const TB_TOGGLE_W: f32 = theme::SWITCH_SLOT_W;
const TB_TOGGLE_H: f32 = theme::SWITCH_SLOT_H;
const SLIDER_W_JSFX: f32 = theme::jsfx_x(theme::PARALLAX_SLOT_W);

const _: () = {
    assert!(theme::ENVELOPE_GROUP_H == theme::GRAPH_SIZE);
    assert!(theme::INPUT_GROUP_H == theme::GRAPH_SIZE);
    assert!(theme::HARMONICS_GROUP_H == theme::GRAPH_SIZE);
};

fn center_in_group_x(group_w: f32, control_w: f32) -> f32 {
    (group_w - control_w) * 0.5
}

fn advance_group_row_y(y: f32, row_h: f32, gap: f32) -> f32 {
    y + row_h + gap
}

fn map_x(x: f32) -> f32 {
    theme::sx(x)
}

fn map_w(w: f32) -> f32 {
    theme::sx(w)
}

/// Screen-space layout for every production control.
#[derive(Debug, Clone, Copy)]
pub struct ControlLayout {
    pub attack_knob: (f32, f32),
    pub release_knob: (f32, f32),
    pub hold_knob: (f32, f32),
    pub hp_slider: (f32, f32, f32),
    pub listen_btn: (f32, f32, f32),
    pub sc_btn: (f32, f32, f32),
    pub lp_slider: (f32, f32, f32),
    pub lookahead_slider: (f32, f32, f32),
    pub brickwall_btn: (f32, f32, f32),
    pub prog_blend_slider: (f32, f32, f32),
    pub inverse_btn: (f32, f32, f32),
    pub attack_curve_slider: (f32, f32, f32),
    pub release_curve_slider: (f32, f32, f32),
    pub detection_btn: (f32, f32),
    pub harmonic_type_switch: (f32, f32),
    pub harmonic_drive: (f32, f32),
    pub harmonic_mix: (f32, f32),
    pub harmonic_even: (f32, f32),
    pub harmonic_odd: (f32, f32),
    pub makeup_knob: (f32, f32),
    pub strength_knob: (f32, f32),
    pub prog_mode_switch: (f32, f32),
    pub ms_switch: (f32, f32),
    pub norm_switch: (f32, f32),
    pub rms_knob: (f32, f32),
    pub offset_knob: (f32, f32),
}

impl ControlLayout {
    pub fn from_jsfx() -> Self {
        let harmonics_envelope_group_y = theme::ENVELOPE_GROUP_Y;
        let envelope_group_x = theme::ENVELOPE_GROUP_X;
        let envelope_group_w = theme::jsfx_x(theme::ENVELOPE_GROUP_W);

        // --- Envelope knobs (group 3) — relative y starts at 0 (no title pad) ---
        let env_knob_rel_x =
            UI_PANEL_X + 10.0 + (envelope_group_w - 20.0) * 0.5 - theme::KNOB_SIZE * 0.5;
        let knob_x = envelope_group_x + env_knob_rel_x;
        let mut env_rel_y = 0.0;
        let attack_knob = (map_x(knob_x), harmonics_envelope_group_y + env_rel_y);
        env_rel_y += theme::KNOB_SIZE + 12.0;
        let release_knob = (map_x(knob_x), harmonics_envelope_group_y + env_rel_y);
        env_rel_y += theme::KNOB_SIZE + 12.0;
        let hold_knob = (map_x(knob_x), harmonics_envelope_group_y + env_rel_y);

        // --- Input / settings (group 0) — relative y includes title pad ---
        let input_settings_w = theme::jsfx_x(theme::INPUT_GROUP_W);
        let input_group_x = theme::jsfx_x(theme::INPUT_GROUP_X);
        let input_group_y = theme::INPUT_GROUP_Y;
        let filter_w = input_settings_w - 20.0;
        let rel_x = input_group_x + center_in_group_x(input_settings_w, filter_w);
        let mut current_y =
            input_group_y + theme::GROUP_TITLE_H + theme::GROUP_PAD + 5.0;

        let hp_lp_w = (filter_w - (BTN_ADJ_PREVIEW_W + BTN_SC_W + FILTER_GAP * 3.0)) * 0.5;
        let hp_x = rel_x;
        let hp_slider = (map_x(hp_x), current_y, map_w(hp_lp_w));
        let listen_x = hp_x + hp_lp_w + FILTER_GAP;
        let listen_btn = (map_x(listen_x), current_y, map_w(BTN_ADJ_PREVIEW_W));
        let sc_x = listen_x + BTN_ADJ_PREVIEW_W + FILTER_GAP;
        let sc_btn = (map_x(sc_x), current_y, map_w(BTN_SC_W));
        let lp_x = sc_x + BTN_SC_W + FILTER_GAP;
        let lp_slider = (map_x(lp_x), current_y, map_w(hp_lp_w));
        current_y += SLIDER_H + 20.0;

        let lookahead_w = filter_w - BRICKWALL_BTN_W - LOOKAHEAD_GAP;
        let lookahead_slider = (map_x(rel_x), current_y, map_w(lookahead_w));
        let brickwall_btn = (
            map_x(rel_x + lookahead_w + LOOKAHEAD_GAP),
            current_y,
            map_w(BRICKWALL_BTN_W),
        );
        current_y += SLIDER_H + 20.0;

        let third_w = (filter_w - PROG_GAP * 2.0) / 3.0;
        let prog_btn_w = third_w - 18.0;
        let prog_slider_w = filter_w - PROG_GAP - prog_btn_w;
        let prog_blend_slider = (map_x(rel_x), current_y, map_w(prog_slider_w));
        let inverse_btn = (
            map_x(rel_x + prog_slider_w + PROG_GAP),
            current_y,
            map_w(prog_btn_w),
        );
        current_y += SLIDER_H + 20.0;

        let half_w = filter_w * 0.5 - 5.0;
        let curve_w = half_w + 20.0;
        let curve_y = current_y;
        let attack_curve_slider = (map_x(rel_x), curve_y, map_w(curve_w));
        let release_curve_slider = (
            map_x(rel_x),
            curve_y + SLIDER_H + CURVE_ROW_GAP,
            map_w(curve_w),
        );
        let detection_btn = (map_x(rel_x + curve_w + 10.0), curve_y);

        // --- Harmonics (group 2) — relative y starts at 0 ---
        let harmonics_group_x = theme::jsfx_x(theme::HARMONICS_GROUP_X);
        let harmonics_group_y = theme::HARMONICS_GROUP_Y;
        let harmonics_group_w = theme::jsfx_x(theme::HARMONICS_GROUP_W);
        let rel_x_slider =
            harmonics_group_x + center_in_group_x(harmonics_group_w, SLIDER_W_JSFX);
        let rel_x_button =
            harmonics_group_x + center_in_group_x(harmonics_group_w, TB_TOGGLE_W);

        let mut harm_y = 0.0;
        let harmonic_type_switch = (map_x(rel_x_button), harmonics_group_y + harm_y);
        harm_y = advance_group_row_y(harm_y, TB_TOGGLE_H, 22.0);
        let harmonic_drive = (map_x(rel_x_slider), harmonics_group_y + harm_y);
        harm_y = advance_group_row_y(harm_y, SLIDER_H, 20.0);
        let harmonic_mix = (map_x(rel_x_slider), harmonics_group_y + harm_y);
        harm_y = advance_group_row_y(harm_y, SLIDER_H, 20.0);
        let harmonic_even = (map_x(rel_x_slider), harmonics_group_y + harm_y);
        harm_y = advance_group_row_y(harm_y, SLIDER_H, 20.0);
        let harmonic_odd = (map_x(rel_x_slider), harmonics_group_y + harm_y);
        harm_y = advance_group_row_y(harm_y, SLIDER_H, 20.0);
        let mk_x = harmonics_group_x + center_in_group_x(harmonics_group_w, theme::KNOB_SIZE);
        let makeup_knob = (map_x(mk_x), harmonics_group_y + harm_y - 11.0);

        // --- Toolbar (group 4) ---
        let toolbar_x_jsfx = theme::jsfx_x(theme::TOOLBAR_X);
        let toolbar_w_jsfx = theme::jsfx_x(theme::TOOLBAR_W);
        let tb_knob_y = theme::TOOLBAR_Y + (theme::TOOLBAR_H - theme::KNOB_SIZE) * 0.5;
        let tb_toggle_y = theme::TOOLBAR_Y + (theme::TOOLBAR_H - TB_TOGGLE_H) * 0.5;
        let strength_knob = (map_x(toolbar_x_jsfx + TOOLBAR_PAD), tb_knob_y);
        let prog_mode_switch = (
            map_x(toolbar_x_jsfx + TOOLBAR_PAD + theme::KNOB_SIZE + TOOLBAR_GAP),
            tb_toggle_y,
        );

        let io_x = toolbar_w_jsfx - TOOLBAR_PAD - theme::KNOB_SIZE;
        let rms_x = io_x - TOOLBAR_KNOB_GAP - theme::KNOB_SIZE;
        let norm_x = rms_x - TOOLBAR_GAP - TB_TOGGLE_W;
        let ms_x = norm_x - TOOLBAR_GAP - TB_TOGGLE_W;

        Self {
            attack_knob,
            release_knob,
            hold_knob,
            hp_slider,
            listen_btn,
            sc_btn,
            lp_slider,
            lookahead_slider,
            brickwall_btn,
            prog_blend_slider,
            inverse_btn,
            attack_curve_slider,
            release_curve_slider,
            detection_btn,
            harmonic_type_switch,
            harmonic_drive,
            harmonic_mix,
            harmonic_even,
            harmonic_odd,
            makeup_knob,
            strength_knob,
            prog_mode_switch,
            ms_switch: (map_x(toolbar_x_jsfx + ms_x), tb_toggle_y),
            norm_switch: (map_x(toolbar_x_jsfx + norm_x), tb_toggle_y),
            rms_knob: (map_x(toolbar_x_jsfx + rms_x), tb_knob_y),
            offset_knob: (map_x(toolbar_x_jsfx + io_x), tb_knob_y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_knobs_use_group_offset() {
        let l = ControlLayout::from_jsfx();
        assert!((l.attack_knob.0 - map_x(45.5)).abs() < 0.1);
        assert!((l.attack_knob.1 - 40.0).abs() < 0.1);
        assert!((l.release_knob.1 - 161.0).abs() < 0.1);
        assert!((l.hold_knob.1 - 282.0).abs() < 0.1);
    }

    #[test]
    fn input_row_y_uses_jsfx_offset() {
        let l = ControlLayout::from_jsfx();
        assert!((l.hp_slider.1 - 73.0).abs() < 0.1);
        assert!((l.hp_slider.0 - map_x(204.0)).abs() < 0.1);
    }

    #[test]
    fn harmonics_start_at_group_origin() {
        let l = ControlLayout::from_jsfx();
        assert!((l.harmonic_type_switch.1 - 40.0).abs() < 0.1);
        assert!((l.harmonic_drive.1 - 139.0).abs() < 0.1);
        assert!((l.harmonic_type_switch.0 - map_x(891.0 + 41.5)).abs() < 0.5);
    }

    #[test]
    fn detection_button_x_matches_curve_row() {
        let l = ControlLayout::from_jsfx();
        assert!((l.detection_btn.0 - map_x(403.0)).abs() < 0.1);
    }

    #[test]
    fn layout_x_scale_matches_bg_asset() {
        assert!((theme::LAYOUT_X_SCALE - 32.0 / 29.0).abs() < 0.0001);
    }

    #[test]
    fn switch_widget_includes_jsfx_overhang() {
        assert!((theme::SWITCH_LEFT_OVERHANG - 25.0).abs() < 0.01);
        assert!((theme::SWITCH_WIDGET_W - 191.0).abs() < 0.01);
        assert!((theme::BUTTON_H - 27.0).abs() < 0.01);
    }

    #[test]
    fn graph_is_square_on_bg_asset() {
        assert!((theme::GRAPH_SIZE - theme::GRAPH_SIZE_X).abs() < 0.01);
    }

    #[test]
    fn group_heights_match_graph_extent() {
        assert!((theme::INPUT_GROUP_H - theme::GRAPH_SIZE).abs() < 0.01);
        assert!((theme::ENVELOPE_GROUP_H - theme::GRAPH_SIZE).abs() < 0.01);
        assert!((theme::HARMONICS_GROUP_H - theme::GRAPH_SIZE).abs() < 0.01);
    }
}
