//! Artwork-space layout and coordinate transforms shared by drawing and input.
use super::*;

// Original HISE layout (UI_MakeInterface.js at 85% zoom of 1390×719).
pub(super) const HEADER_H: f32 = 30.0;
pub(super) const STRIP_W: f32 = 72.0;
pub(super) const MIXER_Y: f32 = 120.0;
pub(super) const FADER_Y: f32 = MIXER_Y + 55.0;
pub(super) const FADER_H: f32 = 250.0;
pub(super) const PITCH_Y: f32 = MIXER_Y;
pub(super) const PAN_Y: f32 = MIXER_Y + 22.0;
pub(super) const PUNCH_Y: f32 = MIXER_Y + 322.0;
pub(super) const SLIDER_H: f32 = 14.0;
pub(super) const PUNCH_SIZE: f32 = 50.0;
pub(super) const SOF_Y: f32 = MIXER_Y + 55.0 + FADER_H + 98.0;
pub(super) const SOF_W: f32 = 65.0;
pub(super) const SOF_H: f32 = 30.0;
pub(super) const SOF_PAD: f32 = 14.0;
pub(super) const LOCK_SIZE: f32 = 26.0;
pub(super) const SUB_KICK: (f32, f32, f32, f32) = (40.0, 400.0, 70.0, 20.0);
pub(super) const PRESET_X: f32 = 8.0;
pub(super) const PRESET_Y: f32 = 0.0;
pub(super) const PRESET_W: f32 = 82.0;
pub(super) const PRESET_H: f32 = HEADER_H;
pub(super) const PRESET_MENU_W: f32 = 210.0;
pub(super) const PRESET_ITEM_H: f32 = 22.0;
pub(super) const PRESET_RULE_H: f32 = 8.0;
pub(super) const RESET_FLYOUT_W: f32 = 140.0;
pub(super) const SAMPLES_X: f32 = PRESET_X + PRESET_W + 6.0;
pub(super) const SAMPLES_Y: f32 = 0.0;
pub(super) const SAMPLES_W: f32 = 92.0;
pub(super) const SAMPLES_H: f32 = HEADER_H;
pub(super) const SAMPLES_MENU_W: f32 = 252.0;
pub(super) const MAP_NOTE_W: f32 = 36.0;
pub(super) const MASTER_VU: (f32, f32, f32, f32) = (WINDOW_W - 330.0, 3.0, 110.0, HEADER_H - 6.0);
pub(super) const CC_INVERT: (f32, f32, f32, f32) = (WINDOW_W - 200.0, 0.0, 60.0, HEADER_H);
pub(super) const CC_LABEL: (f32, f32, f32, f32) = (WINDOW_W - 124.0, 0.0, 60.0, HEADER_H);
pub(super) const CC_SELECT: (f32, f32, f32, f32) = (WINDOW_W - 65.0, 0.0, 43.0, HEADER_H);
pub(super) const CC_DISPLAY: (f32, f32, f32, f32) = (WINDOW_W - 30.0, 0.0, 30.0, HEADER_H);
pub(super) const CC_CELL_W: f32 = 28.0;
pub(super) const CC_CELL_H: f32 = 18.0;
pub(super) const CC_COLS: usize = 8;
pub(super) const CC_ROWS: usize = 16;
pub(super) const LOGO_SC: (f32, f32, f32, f32) =
    (0.0, MIXER_Y + 55.0 + FADER_H + 75.0, 150.0, 60.0);
pub(super) const LOGO_SH: (f32, f32, f32, f32) = (
    0.0,
    MIXER_Y + 55.0 + FADER_H + 100.0,
    1074.0 / 7.0,
    175.0 / 7.0,
);
pub(super) const LABEL_X: f32 = 68.0;
pub(super) const FADER_HANDLE_H: f32 = 26.0;
pub(super) const FADER_HANDLE_INSET: f32 = 10.0;

const SUB_MODAL_W: f32 = 420.0;
const SUB_MODAL_H: f32 = 160.0;
const ADD_MODAL_W: f32 = 360.0;
const ADD_MODAL_H: f32 = 248.0;
const VEL_MODAL_W: f32 = 1040.0;
const VEL_MODAL_H: f32 = 448.0;

pub(super) fn artwork_scale(width: f32) -> f32 {
    width / WINDOW_W
}

impl ScdEditorView {
    pub(super) fn mixer_x() -> f32 {
        (WINDOW_W - KitPieceId::COUNT as f32 * STRIP_W) * 0.5
    }

    pub(super) fn lock_rect() -> (f32, f32, f32, f32) {
        (
            Self::mixer_x() - 35.0,
            MIXER_Y + 120.0,
            LOCK_SIZE,
            LOCK_SIZE,
        )
    }

    pub(super) fn logo_sc_rect() -> (f32, f32, f32, f32) {
        (
            Self::mixer_x() + STRIP_W * KitPieceId::COUNT as f32 - LOGO_SC.2,
            LOGO_SC.1,
            LOGO_SC.2,
            LOGO_SC.3,
        )
    }

    pub(super) fn logo_sh_rect() -> (f32, f32, f32, f32) {
        (Self::mixer_x(), LOGO_SH.1, LOGO_SH.2, LOGO_SH.3)
    }

    pub(super) fn sof_origin() -> (f32, f32) {
        let n = MicChannel::COUNT as f32;
        let total = n * SOF_W + (n - 1.0) * SOF_PAD;
        ((WINDOW_W - total) * 0.5, SOF_Y)
    }

    pub(super) fn sof_rect(i: usize) -> (f32, f32, f32, f32) {
        let (x, y) = Self::sof_origin();
        (x + i as f32 * (SOF_W + SOF_PAD), y, SOF_W, SOF_H)
    }

    pub(super) fn sof_at(x: f32, y: f32) -> Option<MicChannel> {
        MicChannel::ALL.iter().enumerate().find_map(|(i, mic)| {
            let (bx, by, w, h) = Self::sof_rect(i);
            Self::hit(x, y, bx, by, w, h).then_some(*mic)
        })
    }

    pub(super) fn sub_kick_modal() -> (f32, f32, f32, f32) {
        (
            WINDOW_W * 0.5 - SUB_MODAL_W * 0.5,
            WINDOW_H * 0.5 - SUB_MODAL_H * 0.5,
            SUB_MODAL_W,
            SUB_MODAL_H,
        )
    }

    pub(super) fn sub_kick_knob_x(modal_x: f32, i: usize) -> f32 {
        modal_x + 60.0 + i as f32 * 75.0
    }

    pub(super) fn preset_combo_rect() -> (f32, f32, f32, f32) {
        (PRESET_X, PRESET_Y, PRESET_W, PRESET_H)
    }

    pub(super) fn samples_combo_rect() -> (f32, f32, f32, f32) {
        (SAMPLES_X, SAMPLES_Y, SAMPLES_W, SAMPLES_H)
    }

    pub(super) fn samples_item_rect(i: usize) -> (f32, f32, f32, f32) {
        let menu_y = SAMPLES_Y + SAMPLES_H + 4.0;
        (
            SAMPLES_X,
            menu_y + 4.0 + (i + 1) as f32 * PRESET_ITEM_H,
            SAMPLES_MENU_W,
            PRESET_ITEM_H,
        )
    }

    pub(super) fn mapping_header_rect() -> (f32, f32, f32, f32) {
        let menu_y = SAMPLES_Y + SAMPLES_H + 4.0;
        (SAMPLES_X, menu_y + 4.0, SAMPLES_MENU_W, PRESET_ITEM_H)
    }

    pub(super) fn mapping_note_rect(i: usize, note2: bool) -> (f32, f32, f32, f32) {
        let (x, y, _, h) = Self::samples_item_rect(i);
        let ox = if note2 { MAP_NOTE_W } else { 0.0 };
        (x + 4.0 + ox, y + 2.0, MAP_NOTE_W - 4.0, h - 4.0)
    }

    pub(super) fn mapping_name_rect(i: usize) -> (f32, f32, f32, f32) {
        let (x, y, w, h) = Self::samples_item_rect(i);
        let left = 4.0 + MAP_NOTE_W * 2.0;
        (x + left, y, w - left, h)
    }

    pub(super) fn reset_flyout_item(i: usize) -> (f32, f32, f32, f32) {
        let menu_y = PRESET_Y + PRESET_H + 8.0;
        (
            PRESET_X + PRESET_MENU_W - 4.0,
            menu_y + 4.0 + i as f32 * PRESET_ITEM_H,
            RESET_FLYOUT_W,
            PRESET_ITEM_H,
        )
    }

    pub(super) fn reset_flyout_rect() -> (f32, f32, f32, f32) {
        let (x, y, _, _) = Self::reset_flyout_item(0);
        let n = FactoryPreset::RESETS.len() as f32;
        (x, y - 4.0, RESET_FLYOUT_W, n * PRESET_ITEM_H + 8.0)
    }

    pub(super) fn add_preset_modal() -> (f32, f32, f32, f32) {
        (
            WINDOW_W * 0.5 - ADD_MODAL_W * 0.5,
            WINDOW_H * 0.5 - ADD_MODAL_H * 0.5,
            ADD_MODAL_W,
            ADD_MODAL_H,
        )
    }

    pub(super) fn add_name_rect(modal: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        (modal.0 + 20.0, modal.1 + 52.0, modal.2 - 40.0, 28.0)
    }

    pub(super) fn add_scope_rect(modal: (f32, f32, f32, f32), i: usize) -> (f32, f32, f32, f32) {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        (
            modal.0 + 24.0 + col * 160.0,
            modal.1 + 96.0 + row * 28.0,
            150.0,
            24.0,
        )
    }

    pub(super) fn add_save_rect(modal: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        (
            modal.0 + modal.2 - 168.0,
            modal.1 + modal.3 - 44.0,
            70.0,
            26.0,
        )
    }

    pub(super) fn add_cancel_rect(modal: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        (
            modal.0 + modal.2 - 90.0,
            modal.1 + modal.3 - 44.0,
            70.0,
            26.0,
        )
    }

    pub(super) fn vel_map_modal() -> (f32, f32, f32, f32) {
        (
            WINDOW_W * 0.5 - VEL_MODAL_W * 0.5,
            WINDOW_H * 0.5 - VEL_MODAL_H * 0.5,
            VEL_MODAL_W,
            VEL_MODAL_H,
        )
    }

    pub(super) fn vel_map_graph() -> (f32, f32, f32, f32) {
        let (x, y, w, h) = Self::vel_map_modal();
        (x + 24.0, y + 186.0, w - 48.0, h - 210.0)
    }

    pub(super) fn vel_art_dropdown_rect() -> (f32, f32, f32, f32) {
        let (x, y, w, _) = Self::vel_map_modal();
        (x + 24.0, y + 126.0, w - 48.0, 32.0)
    }

    pub(super) fn vel_art_item_rect(i: usize) -> (f32, f32, f32, f32) {
        let (x, y, w, h) = Self::vel_art_dropdown_rect();
        (x, y + h + 4.0 + i as f32 * 22.0, w, 22.0)
    }

    pub(super) fn graph_to_xy(x: f32, y: f32) -> (f32, f32) {
        pleasant_ui::graph::Viewport::from_tuple(Self::vel_map_graph()).to_normalized(x, y)
    }

    pub(super) fn graph_to_norm(x: f32, y: f32) -> (f32, f32) {
        pleasant_ui::graph::Viewport::from_tuple(Self::vel_map_graph()).to_normalized_clamped(x, y)
    }

    pub(super) fn norm_to_graph(nx: f32, ny: f32) -> (f32, f32) {
        pleasant_ui::graph::Viewport::from_tuple(Self::vel_map_graph()).from_normalized(nx, ny)
    }

    pub(super) fn cc_cell_rect(menu: (f32, f32, f32, f32), n: usize) -> (f32, f32, f32, f32) {
        let col = (n % CC_COLS) as f32;
        let row = (n / CC_COLS) as f32;
        (
            menu.0 + 4.0 + col * CC_CELL_W,
            menu.1 + 4.0 + row * CC_CELL_H,
            CC_CELL_W,
            CC_CELL_H,
        )
    }

    pub(super) fn cc_index_at(x: f32, y: f32, menu: (f32, f32, f32, f32)) -> Option<i32> {
        let col = ((x - menu.0 - 4.0) / CC_CELL_W).floor() as i32;
        let row = ((y - menu.1 - 4.0) / CC_CELL_H).floor() as i32;
        if col >= 0 && row >= 0 && col < CC_COLS as i32 && row < CC_ROWS as i32 {
            Some(row * CC_COLS as i32 + col)
        } else {
            None
        }
    }

    pub(super) fn hit(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
        (x..=x + w).contains(&px) && (y..=y + h).contains(&py)
    }

    pub(super) fn fader_pos_at(y: f32) -> f32 {
        let travel = (FADER_H - FADER_HANDLE_H).max(1.0);
        let t = (y - FADER_Y - FADER_HANDLE_H * 0.5) / travel;
        (1.0 - t).clamp(0.0, 1.0)
    }

    pub(super) fn slider_norm_at(x: f32, strip_x: f32) -> f32 {
        let sx = strip_x + 5.0;
        let sw = STRIP_W - 10.0;
        ((x - sx) / sw).clamp(0.0, 1.0)
    }

    pub(super) fn strip_x(kit_piece: KitPieceId) -> f32 {
        Self::mixer_x() + kit_piece as usize as f32 * STRIP_W
    }

    pub(super) fn cc_menu_rect() -> (f32, f32, f32, f32) {
        let w = CC_COLS as f32 * CC_CELL_W + 8.0;
        let h = CC_ROWS as f32 * CC_CELL_H + 8.0;
        (WINDOW_W - w - 8.0, HEADER_H + 4.0, w, h)
    }
}
