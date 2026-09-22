mod fader_law;

use crate::dsp::kick_sine::length_to_decay_ms;
use crate::params::{
    peak_to_meter, ScdParams, EDITOR_HEIGHT, EDITOR_WIDTH, FADER_MAX_DB, FADER_MIN_DB,
    VU_PEAK_COUNT,
};
use crate::presets::{
    default_send_db, FactoryPreset, PresetScope, UserPreset, DRUMMER_PANS, MAX_USER_PRESETS,
};
use crate::vel_map::{VelCurve, VelMapState};
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{
        prelude::*,
        vg::{
            Color, FontId, ImageFilter, ImageFlags, ImageId, LineCap, LineJoin, Paint, Path,
            PixelFormat, RenderTarget, Solidity,
        },
    },
    widgets::{util::ModifiersExt, GuiContextEvent, ParamEvent, RawParamEvent},
    ViziaTheming,
};
use pleasant_ui::{draw::Draw, theme::rgb, typed_char, ValueEdit};
use scd_core::kit::{art_switch_groups, stonehouse};
use scd_core::{KitPieceId, MicChannel};
use std::cell::Cell;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const WINDOW_W: f32 = EDITOR_WIDTH as f32;
pub const WINDOW_H: f32 = EDITOR_HEIGHT as f32;

// Original HISE layout (UI_MakeInterface.js at 85% zoom of 1390×719).
const HEADER_H: f32 = 30.0;
const STRIP_W: f32 = 72.0;
const MIXER_Y: f32 = 120.0;
const FADER_Y: f32 = MIXER_Y + 55.0;
const FADER_H: f32 = 250.0;
const PITCH_Y: f32 = MIXER_Y;
const PAN_Y: f32 = MIXER_Y + 22.0;
const PUNCH_Y: f32 = MIXER_Y + 322.0;
const SLIDER_H: f32 = 14.0;
const PUNCH_SIZE: f32 = 50.0;
const SOF_Y: f32 = MIXER_Y + 55.0 + FADER_H + 98.0;
const SOF_W: f32 = 65.0;
const SOF_H: f32 = 30.0;
const SOF_PAD: f32 = 14.0;
const LOCK_SIZE: f32 = 26.0;
const SUB_KICK: (f32, f32, f32, f32) = (40.0, 400.0, 70.0, 20.0);
const ZOOM_X: f32 = 8.0;
const ZOOM_Y: f32 = 0.0;
const ZOOM_W: f32 = 64.0;
const ZOOM_H: f32 = HEADER_H;
const ZOOM_MENU_W: f32 = 72.0;
const ZOOM_LEVELS: [i32; 7] = [50, 75, 85, 100, 125, 150, 200];
const PRESET_X: f32 = 78.0;
const PRESET_Y: f32 = 0.0;
const PRESET_W: f32 = 82.0;
const PRESET_H: f32 = HEADER_H;
const PRESET_MENU_W: f32 = 210.0;
const PRESET_ITEM_H: f32 = 22.0;
const PRESET_RULE_H: f32 = 8.0;
const RESET_FLYOUT_W: f32 = 140.0;
const SAMPLES_X: f32 = PRESET_X + PRESET_W + 6.0;
const SAMPLES_Y: f32 = 0.0;
const SAMPLES_W: f32 = 92.0;
const SAMPLES_H: f32 = HEADER_H;
const SAMPLES_MENU_W: f32 = 252.0;
const MAP_NOTE_W: f32 = 36.0;
const MASTER_VU: (f32, f32, f32, f32) = (WINDOW_W - 330.0, 3.0, 110.0, HEADER_H - 6.0);
const CC_INVERT: (f32, f32, f32, f32) = (WINDOW_W - 200.0, 0.0, 60.0, HEADER_H);
const CC_LABEL: (f32, f32, f32, f32) = (WINDOW_W - 124.0, 0.0, 60.0, HEADER_H);
const CC_SELECT: (f32, f32, f32, f32) = (WINDOW_W - 65.0, 0.0, 43.0, HEADER_H);
const CC_DISPLAY: (f32, f32, f32, f32) = (WINDOW_W - 30.0, 0.0, 30.0, HEADER_H);
const CC_CELL_W: f32 = 28.0;
const CC_CELL_H: f32 = 18.0;
const CC_COLS: usize = 8;
const CC_ROWS: usize = 16;
const LOGO_SC: (f32, f32, f32, f32) = (0.0, MIXER_Y + 55.0 + FADER_H + 75.0, 150.0, 60.0);
const LOGO_SH: (f32, f32, f32, f32) = (
    0.0,
    MIXER_Y + 55.0 + FADER_H + 100.0,
    1074.0 / 7.0,
    175.0 / 7.0,
);
const LABEL_X: f32 = 68.0;
const FADER_HANDLE_H: f32 = 26.0;
const FADER_HANDLE_INSET: f32 = 10.0;
const HEADER: Color = rgb(23, 18, 24);
const SCRIM: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.62,
};
const GLASS_TINT: Color = Color {
    r: 23.0 / 255.0,
    g: 18.0 / 255.0,
    b: 24.0 / 255.0,
    a: 0.48,
};
const GLASS_SHEEN: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.14,
};
const GLASS_EDGE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.32,
};
const GLASS_INNER: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.10,
};
const GLASS_SHADOW: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.40,
};
const GLASS_BLUR_SIGMA: f32 = 18.0;
const GLASS_BLUR_PAD: f32 = 48.0;
const THEME: Color = rgb(235, 235, 235);
const THEME_DIM: Color = Color {
    r: 235.0 / 255.0,
    g: 235.0 / 255.0,
    b: 235.0 / 255.0,
    a: 0.2,
};
const FADER_LINE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.13,
};
const FADER_MASTER: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.8,
};
const FADER_SHADOW: Color = Color {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 0.6,
};
const FADER_TEXT: Color = rgb(0, 0, 0);
const SOF_OFF: Color = Color {
    r: 21.0 / 255.0,
    g: 21.0 / 255.0,
    b: 21.0 / 255.0,
    a: 0.47,
};
const SOF_HOVER: Color = Color {
    r: 21.0 / 255.0,
    g: 21.0 / 255.0,
    b: 21.0 / 255.0,
    a: 0.3,
};
// HISE pan/pitch LAF wash: 0x17FFFFFF.
const SLIDER_WASH: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0x17 as f32 / 255.0,
};
// HISE AHDSRGraph tile: itemColour 0x1AFFFFFF, itemColour2 0x80FFFFFF.
const ENV_FILL: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0x1A as f32 / 255.0,
};
const ENV_LINE: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0x80 as f32 / 255.0,
};
const VU_FILL: Color = Color {
    r: 235.0 / 255.0,
    g: 235.0 / 255.0,
    b: 235.0 / 255.0,
    a: 0.73,
};
const NOTE_YELLOW: Color = rgb(244, 214, 92);
const STAGE_HOVER: Color = Color {
    r: 235.0 / 255.0,
    g: 235.0 / 255.0,
    b: 235.0 / 255.0,
    a: 0.10,
};
const STAGE_DOWN: Color = Color {
    r: 235.0 / 255.0,
    g: 235.0 / 255.0,
    b: 235.0 / 255.0,
    a: 0.38,
};
const STAGE_ON: Color = Color {
    r: 235.0 / 255.0,
    g: 235.0 / 255.0,
    b: 235.0 / 255.0,
    a: 0.20,
};
const STAGE_ON_HOVER: Color = Color {
    r: 235.0 / 255.0,
    g: 235.0 / 255.0,
    b: 235.0 / 255.0,
    a: 0.30,
};
// HISE UI_Meters.js vertical mono meter: 0x00B4A284 → 0xFFa1c6e8.
const CHAN_METER_TOP: Color = Color {
    r: 180.0 / 255.0,
    g: 162.0 / 255.0,
    b: 132.0 / 255.0,
    a: 0.0,
};
const CHAN_METER_BOT: Color = Color {
    r: 161.0 / 255.0,
    g: 198.0 / 255.0,
    b: 232.0 / 255.0,
    a: 1.0,
};

const BG_PNG: &[u8] = include_bytes!("../../assets/bg.png");
const LOGO_PNG: &[u8] = include_bytes!("../../assets/SoundchefSmall.png");
const STONE_PNG: &[u8] = include_bytes!("../../assets/Stonehouse.png");

pub const MIC_COLORS: [Color; MicChannel::COUNT] = [
    rgb(184, 244, 171), // Close (soft green)
    rgb(244, 171, 184), // XY (soft pink)
    rgb(171, 220, 244), // Mono (soft blue)
    rgb(231, 171, 244), // Wide (soft violet)
    rgb(244, 195, 171), // Front MS (soft peach)
    rgb(244, 231, 171), // Room (soft yellow)
];

const VU_DECAY: f32 = 0.97;
const VU_IDLE_EPS: f32 = 0.002;

#[derive(Clone, Copy, PartialEq, Eq)]
enum PresetPick {
    Factory(FactoryPreset),
    User(usize),
}

#[derive(Clone, Copy)]
enum PresetRow {
    Resets,
    Rule,
    Factory(FactoryPreset),
    User(usize),
    Add,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum DragTarget {
    Pitch(KitPieceId),
    Pan(KitPieceId),
    Fader(KitPieceId),
    Punch(KitPieceId),
    SubKickVol,
    SubKickLength,
    SubKickDive,
    SubKickSpeed,
    SubKickOffset,
    VelMapNode,
    VelMapHandleIn,
    VelMapHandleOut,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct NoteTarget {
    kit_piece: KitPieceId,
    art: usize,
    note2: bool,
}

#[derive(Clone, Copy)]
struct NoteLearnPoll;

pub struct ScdEditorView {
    params: Arc<ScdParams>,
    active_sof: Option<MicChannel>,
    sub_kick_open: bool,
    preset_open: bool,
    resets_open: bool,
    add_preset_open: bool,
    add_name: String,
    add_scope: PresetScope,
    add_name_focus: bool,
    samples_open: bool,
    vel_map_open: Option<KitPieceId>,
    vel_art: usize,
    vel_selected_node: Option<usize>,
    vel_just_inserted: bool,
    vel_ignore_up: bool,
    vel_hits: Vec<(KitPieceId, usize, u8, Instant)>,
    note_edit: Option<ValueEdit<NoteTarget>>,
    zoom_open: bool,
    zoom_pct: i32,
    cc_menu_open: bool,
    current_preset: PresetPick,
    drag: Option<DragTarget>,
    hover_sof: Option<MicChannel>,
    hover_lock: bool,
    hover_invert: bool,
    hover_note: Option<NoteTarget>,
    press_invert: bool,
    press_note: Option<NoteTarget>,
    gesture: [Option<ParamPtr>; KitPieceId::COUNT],
    mouse: (f32, f32),
    font: Cell<Option<FontId>>,
    icon_font: Cell<Option<FontId>>,
    bg_img: Cell<Option<ImageId>>,
    logo_img: Cell<Option<ImageId>>,
    stone_img: Cell<Option<ImageId>>,
    blur_shot: Cell<Option<ImageId>>,
    blur_src: Cell<Option<ImageId>>,
    blur_dst: Cell<Option<ImageId>>,
    blur_dirty: Cell<bool>,
    /// Peak-held decaying display values.
    /// Layout: [mic0_L, mic0_R, mic1_L, mic1_R, ... mic5_R, master_L, master_R] = 14
    vu_display: Cell<[f32; VU_PEAK_COUNT]>,
    kit_vu: Cell<[f32; KitPieceId::COUNT]>,
}

impl ScdEditorView {
    pub fn new(cx: &mut Context, params: Arc<ScdParams>) -> Handle<'_, Self> {
        let zoom_pct = initial_zoom_pct(&params);
        Self {
            params,
            active_sof: None,
            sub_kick_open: false,
            preset_open: false,
            resets_open: false,
            add_preset_open: false,
            add_name: String::new(),
            add_scope: PresetScope::all(),
            add_name_focus: false,
            samples_open: false,
            vel_map_open: None,
            vel_art: 0,
            vel_selected_node: None,
            vel_just_inserted: false,
            vel_ignore_up: false,
            vel_hits: Vec::new(),
            note_edit: None,
            zoom_open: false,
            zoom_pct,
            cc_menu_open: false,
            current_preset: PresetPick::Factory(FactoryPreset::Init),
            drag: None,
            hover_sof: None,
            hover_lock: false,
            hover_invert: false,
            hover_note: None,
            press_invert: false,
            press_note: None,
            gesture: std::array::from_fn(|_| None),
            mouse: (0.0, 0.0),
            font: Cell::new(None),
            icon_font: Cell::new(None),
            bg_img: Cell::new(None),
            logo_img: Cell::new(None),
            stone_img: Cell::new(None),
            blur_shot: Cell::new(None),
            blur_src: Cell::new(None),
            blur_dst: Cell::new(None),
            blur_dirty: Cell::new(true),
            vu_display: Cell::new([0.0f32; VU_PEAK_COUNT]),
            kit_vu: Cell::new([0.0f32; KitPieceId::COUNT]),
        }
        .build(cx, |cx| {
            let timer = cx.add_timer(Duration::from_millis(16), None, |cx, action| {
                if let TimerAction::Tick(_) = action {
                    let dirty = cx
                        .get_view::<ScdEditorView>()
                        .is_some_and(ScdEditorView::needs_idle_redraw);
                    if dirty {
                        cx.emit(NoteLearnPoll);
                        cx.needs_redraw();
                    }
                }
            });
            cx.start_timer(timer);
        })
    }

    fn needs_idle_redraw(&self) -> bool {
        if self.sub_kick_open
            || self.preset_open
            || self.resets_open
            || self.add_preset_open
            || self.samples_open
            || self.vel_map_open.is_some()
            || self.zoom_open
            || self.cc_menu_open
            || self.drag.is_some()
            || self.hover_sof.is_some()
            || self.hover_lock
            || self.hover_invert
            || self.hover_note.is_some()
            || self.press_invert
            || self.press_note.is_some()
            || self.note_edit.is_some()
            || self.gesture.iter().any(Option::is_some)
        {
            return true;
        }
        if self.vu_display.get().iter().any(|v| *v > VU_IDLE_EPS)
            || self.kit_vu.get().iter().any(|v| *v > VU_IDLE_EPS)
        {
            return true;
        }
        self.params.vu.any_above(1.0e-5)
    }

    fn emit_norm(&self, cx: &mut EventContext, ptr: ParamPtr, norm: f32) {
        let norm = norm.clamp(0.0, 1.0);
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
        cx.emit(RawParamEvent::SetParameterNormalized(ptr, norm));
        cx.emit(RawParamEvent::EndSetParameter(ptr));
    }

    fn emit_plain(&self, cx: &mut EventContext, param: &FloatParam, plain: f32) {
        self.emit_norm(cx, param.as_ptr(), param.preview_normalized(plain));
    }

    fn emit_default(&self, cx: &mut EventContext, param: &FloatParam) {
        self.emit_norm(cx, param.as_ptr(), param.default_normalized_value());
    }

    fn mixer_x() -> f32 {
        (WINDOW_W - KitPieceId::COUNT as f32 * STRIP_W) * 0.5
    }

    fn lock_rect() -> (f32, f32, f32, f32) {
        (
            Self::mixer_x() - 35.0,
            MIXER_Y + 120.0,
            LOCK_SIZE,
            LOCK_SIZE,
        )
    }

    fn logo_sc_rect() -> (f32, f32, f32, f32) {
        (
            Self::mixer_x() + STRIP_W * KitPieceId::COUNT as f32 - LOGO_SC.2,
            LOGO_SC.1,
            LOGO_SC.2,
            LOGO_SC.3,
        )
    }

    fn logo_sh_rect() -> (f32, f32, f32, f32) {
        (Self::mixer_x(), LOGO_SH.1, LOGO_SH.2, LOGO_SH.3)
    }

    fn sof_origin() -> (f32, f32) {
        let n = MicChannel::COUNT as f32;
        let total = n * SOF_W + (n - 1.0) * SOF_PAD;
        ((WINDOW_W - total) * 0.5, SOF_Y)
    }

    fn sof_rect(i: usize) -> (f32, f32, f32, f32) {
        let (x, y) = Self::sof_origin();
        (x + i as f32 * (SOF_W + SOF_PAD), y, SOF_W, SOF_H)
    }

    fn sof_at(x: f32, y: f32) -> Option<MicChannel> {
        MicChannel::ALL.iter().enumerate().find_map(|(i, mic)| {
            let (bx, by, w, h) = Self::sof_rect(i);
            Self::hit(x, y, bx, by, w, h).then_some(*mic)
        })
    }

    const SUB_MODAL_W: f32 = 420.0;
    const SUB_MODAL_H: f32 = 160.0;

    fn sub_kick_modal() -> (f32, f32, f32, f32) {
        (
            WINDOW_W * 0.5 - Self::SUB_MODAL_W * 0.5,
            WINDOW_H * 0.5 - Self::SUB_MODAL_H * 0.5,
            Self::SUB_MODAL_W,
            Self::SUB_MODAL_H,
        )
    }

    fn sub_kick_knob_x(modal_x: f32, i: usize) -> f32 {
        modal_x + 60.0 + i as f32 * 75.0
    }

    fn zoom_combo_rect() -> (f32, f32, f32, f32) {
        (ZOOM_X, ZOOM_Y, ZOOM_W, ZOOM_H)
    }

    fn preset_combo_rect() -> (f32, f32, f32, f32) {
        (PRESET_X, PRESET_Y, PRESET_W, PRESET_H)
    }

    fn samples_combo_rect() -> (f32, f32, f32, f32) {
        (SAMPLES_X, SAMPLES_Y, SAMPLES_W, SAMPLES_H)
    }

    fn samples_item_rect(i: usize) -> (f32, f32, f32, f32) {
        let menu_y = SAMPLES_Y + SAMPLES_H + 4.0;
        (
            SAMPLES_X,
            menu_y + 4.0 + (i + 1) as f32 * PRESET_ITEM_H,
            SAMPLES_MENU_W,
            PRESET_ITEM_H,
        )
    }

    fn mapping_header_rect() -> (f32, f32, f32, f32) {
        let menu_y = SAMPLES_Y + SAMPLES_H + 4.0;
        (SAMPLES_X, menu_y + 4.0, SAMPLES_MENU_W, PRESET_ITEM_H)
    }

    fn mapping_note_rect(i: usize, note2: bool) -> (f32, f32, f32, f32) {
        let (x, y, _, h) = Self::samples_item_rect(i);
        let ox = if note2 { MAP_NOTE_W } else { 0.0 };
        (x + 4.0 + ox, y + 2.0, MAP_NOTE_W - 4.0, h - 4.0)
    }

    fn mapping_name_rect(i: usize) -> (f32, f32, f32, f32) {
        let (x, y, w, h) = Self::samples_item_rect(i);
        let left = 4.0 + MAP_NOTE_W * 2.0;
        (x + left, y, w - left, h)
    }

    fn preset_menu_rows(&self) -> Vec<(PresetRow, (f32, f32, f32, f32))> {
        let users = self.params.user_presets.list();
        let mut y = PRESET_Y + PRESET_H + 8.0;
        let mut rows = Vec::new();
        rows.push((
            PresetRow::Resets,
            (PRESET_X, y, PRESET_MENU_W, PRESET_ITEM_H),
        ));
        y += PRESET_ITEM_H;
        rows.push((
            PresetRow::Rule,
            (
                PRESET_X + 10.0,
                y + PRESET_RULE_H * 0.5 - 0.5,
                PRESET_MENU_W - 20.0,
                1.0,
            ),
        ));
        y += PRESET_RULE_H;
        for preset in FactoryPreset::SNAPSHOTS {
            rows.push((
                PresetRow::Factory(preset),
                (PRESET_X, y, PRESET_MENU_W, PRESET_ITEM_H),
            ));
            y += PRESET_ITEM_H;
        }
        for i in 0..users.len() {
            rows.push((
                PresetRow::User(i),
                (PRESET_X, y, PRESET_MENU_W, PRESET_ITEM_H),
            ));
            y += PRESET_ITEM_H;
        }
        rows.push((
            PresetRow::Rule,
            (
                PRESET_X + 10.0,
                y + PRESET_RULE_H * 0.5 - 0.5,
                PRESET_MENU_W - 20.0,
                1.0,
            ),
        ));
        y += PRESET_RULE_H;
        rows.push((PresetRow::Add, (PRESET_X, y, PRESET_MENU_W, PRESET_ITEM_H)));
        rows
    }

    fn preset_menu_rect(&self) -> (f32, f32, f32, f32) {
        let rows = self.preset_menu_rows();
        let top = PRESET_Y + PRESET_H + 4.0;
        let bottom = rows.last().map(|(_, r)| r.1 + r.3).unwrap_or(top) + 4.0;
        (
            PRESET_X,
            top,
            PRESET_MENU_W,
            (bottom - top).max(PRESET_ITEM_H),
        )
    }

    fn reset_flyout_item(i: usize) -> (f32, f32, f32, f32) {
        let menu_y = PRESET_Y + PRESET_H + 8.0;
        (
            PRESET_X + PRESET_MENU_W - 4.0,
            menu_y + 4.0 + i as f32 * PRESET_ITEM_H,
            RESET_FLYOUT_W,
            PRESET_ITEM_H,
        )
    }

    fn reset_flyout_rect() -> (f32, f32, f32, f32) {
        let (x, y, _, _) = Self::reset_flyout_item(0);
        let n = FactoryPreset::RESETS.len() as f32;
        (x, y - 4.0, RESET_FLYOUT_W, n * PRESET_ITEM_H + 8.0)
    }

    const ADD_MODAL_W: f32 = 360.0;
    const ADD_MODAL_H: f32 = 248.0;

    fn add_preset_modal() -> (f32, f32, f32, f32) {
        (
            WINDOW_W * 0.5 - Self::ADD_MODAL_W * 0.5,
            WINDOW_H * 0.5 - Self::ADD_MODAL_H * 0.5,
            Self::ADD_MODAL_W,
            Self::ADD_MODAL_H,
        )
    }

    fn add_name_rect(modal: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        (modal.0 + 20.0, modal.1 + 52.0, modal.2 - 40.0, 28.0)
    }

    fn add_scope_rect(modal: (f32, f32, f32, f32), i: usize) -> (f32, f32, f32, f32) {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        (
            modal.0 + 24.0 + col * 160.0,
            modal.1 + 96.0 + row * 28.0,
            150.0,
            24.0,
        )
    }

    fn add_save_rect(modal: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        (
            modal.0 + modal.2 - 168.0,
            modal.1 + modal.3 - 44.0,
            70.0,
            26.0,
        )
    }

    fn add_cancel_rect(modal: (f32, f32, f32, f32)) -> (f32, f32, f32, f32) {
        (
            modal.0 + modal.2 - 90.0,
            modal.1 + modal.3 - 44.0,
            70.0,
            26.0,
        )
    }

    fn zoom_factor(&self) -> f32 {
        self.zoom_pct as f32 / 100.0
    }

    fn content_scale(&self, dpi: f32) -> f32 {
        dpi * self.zoom_factor()
    }

    fn apply_zoom(&mut self, cx: &mut EventContext, pct: i32) {
        self.zoom_pct = pct;
        self.zoom_open = false;
        self.params
            .editor_zoom_pct
            .store(pct.max(0) as u32, Ordering::Relaxed);
        // Keep Vizia user-scale at 1 so DPI stays host HiDPI only. Window size
        // comes from `ViziaState`'s size_fn, which reads `editor_zoom_pct`.
        cx.set_user_scale_factor(1.0);
        cx.emit(GuiContextEvent::Resize);
        cx.needs_redraw();
    }

    fn close_preset_menus(&mut self) {
        self.preset_open = false;
        self.resets_open = false;
    }

    fn open_add_preset(&mut self, cx: &mut EventContext) {
        self.close_preset_menus();
        self.add_preset_open = true;
        self.add_name = next_preset_name(&self.params.user_presets.list());
        self.add_scope = PresetScope::all();
        self.add_name_focus = true;
        self.blur_dirty.set(true);
        cx.focus();
        cx.needs_redraw();
    }

    fn save_user_preset(&mut self, cx: &mut EventContext) {
        let name = self.add_name.trim();
        if name.is_empty() || !self.add_scope.any() {
            return;
        }
        if self.params.user_presets.list().len() >= MAX_USER_PRESETS {
            return;
        }
        let preset = UserPreset::capture(name.to_string(), self.add_scope, &self.params);
        if let Some(idx) = self.params.user_presets.push(preset) {
            self.current_preset = PresetPick::User(idx);
        }
        self.add_preset_open = false;
        self.add_name_focus = false;
        cx.needs_redraw();
    }

    fn apply_user_preset(&mut self, cx: &mut EventContext, index: usize) {
        let Some(preset) = self.params.user_presets.get(index) else {
            return;
        };
        for (ptr, norm) in preset.apply_values(&self.params) {
            self.emit_norm(cx, ptr, norm);
        }
        if let Some(bank) = preset.vel_maps {
            self.params.vel_maps.load_bank(bank);
        }
        if let Some(bank) = preset.note_maps {
            self.params.note_maps.load_bank(bank);
        }
        self.current_preset = PresetPick::User(index);
    }

    const VEL_MODAL_W: f32 = 1040.0;
    const VEL_MODAL_H: f32 = 448.0;
    const VEL_HEADER_H: f32 = 26.0;
    const VEL_SWITCH_Y: f32 = 134.0;

    fn vel_map_modal() -> (f32, f32, f32, f32) {
        (
            WINDOW_W * 0.5 - Self::VEL_MODAL_W * 0.5,
            WINDOW_H * 0.5 - Self::VEL_MODAL_H * 0.5,
            Self::VEL_MODAL_W,
            Self::VEL_MODAL_H,
        )
    }

    fn vel_map_graph() -> (f32, f32, f32, f32) {
        let (x, y, w, h) = Self::vel_map_modal();
        (x + 24.0, y + 186.0, w - 48.0, h - 210.0)
    }

    fn vel_switch_track() -> (f32, f32, f32, f32) {
        let (x, y, w, _) = Self::vel_map_modal();
        (x + 24.0, y + Self::VEL_SWITCH_Y, w - 48.0, 32.0)
    }

    fn vel_group_header_rect(start: usize, count: usize, n: usize) -> (f32, f32, f32, f32) {
        let (x, y, w, _) = Self::vel_switch_track();
        let pad = 4.0;
        let inner_w = (w - pad * 2.0).max(1.0);
        let seg_w = inner_w / n.max(1) as f32;
        (
            x + pad + start as f32 * seg_w,
            y - Self::VEL_HEADER_H,
            seg_w * count.max(1) as f32,
            Self::VEL_HEADER_H - 1.0,
        )
    }

    /// Choices shown in the velocity mapping switch. Hi-hat shoulder and tip
    /// are UI groups; their curves are still stored per underlying articulation.
    fn vel_art_options(kit_piece: KitPieceId) -> Vec<(usize, String)> {
        if kit_piece == KitPieceId::Hihat {
            vec![
                (0, "Splash".to_string()),
                (1, "Stomp".to_string()),
                (2, "Shoulder".to_string()),
                (7, "Tip".to_string()),
            ]
        } else {
            let names: Vec<String> = stonehouse()
                .arts(kit_piece)
                .iter()
                .map(|art| art.short_name())
                .collect();
            let groups = art_switch_groups(&names);
            let headers: Vec<_> = groups.iter().filter_map(|g| g.header.as_ref()).collect();
            groups
                .clone()
                .into_iter()
                .flat_map(|group| {
                    let start = group.start;
                    group
                        .labels
                        .into_iter()
                        .enumerate()
                        .map(move |(offset, label)| (start + offset, label))
                })
                .filter(|(art, _)| !headers.iter().any(|header| **header == names[*art]))
                .collect()
        }
    }

    /// Header selection refers to its own articulation, not its children.
    fn vel_headers(kit_piece: KitPieceId) -> Vec<(String, Option<usize>, usize, usize)> {
        if kit_piece == KitPieceId::Hihat {
            return Vec::new();
        }
        let names: Vec<_> = stonehouse()
            .arts(kit_piece)
            .iter()
            .map(|a| a.short_name())
            .collect();
        let options = Self::vel_art_options(kit_piece);
        art_switch_groups(&names)
            .into_iter()
            .filter_map(|group| {
                let header = group.header?;
                let start = options.iter().position(|(art, _)| *art == group.start)?;
                let art = names.iter().position(|name| *name == header);
                Some((header, art, start, group.labels.len()))
            })
            .collect()
    }

    fn vel_art_members(kit_piece: KitPieceId, art: usize) -> Vec<usize> {
        match (kit_piece, art) {
            (KitPieceId::Hihat, 2) => (2..7).collect(),
            (KitPieceId::Hihat, 7) => (7..12).collect(),
            _ => vec![art],
        }
    }

    fn mapping_art(kit_piece: KitPieceId) -> usize {
        stonehouse()
            .arts(kit_piece)
            .iter()
            .enumerate()
            .max_by_key(|(_, art)| art.layers)
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn vel_seg_rect(i: usize, n: usize) -> (f32, f32, f32, f32) {
        let (x, y, w, h) = Self::vel_switch_track();
        let pad = 4.0;
        let inner_w = (w - pad * 2.0).max(1.0);
        let seg_w = inner_w / n.max(1) as f32;
        (
            x + pad + i as f32 * seg_w + 7.0,
            y + pad,
            (seg_w - 14.0).max(1.0),
            h - pad * 2.0,
        )
    }

    fn graph_to_xy(x: f32, y: f32) -> (f32, f32) {
        let (gx, gy, gw, gh) = Self::vel_map_graph();
        let nx = (x - gx) / gw.max(1.0);
        let ny = 1.0 - (y - gy) / gh.max(1.0);
        (nx, ny)
    }

    fn graph_to_norm(x: f32, y: f32) -> (f32, f32) {
        let (nx, ny) = Self::graph_to_xy(x, y);
        (nx.clamp(0.0, 1.0), ny.clamp(0.0, 1.0))
    }

    fn norm_to_graph(nx: f32, ny: f32) -> (f32, f32) {
        let (gx, gy, gw, gh) = Self::vel_map_graph();
        (gx + nx * gw, gy + (1.0 - ny) * gh)
    }

    fn zoom_item_rect(i: usize) -> (f32, f32, f32, f32) {
        let menu_y = ZOOM_Y + ZOOM_H + 4.0;
        (
            ZOOM_X,
            menu_y + 4.0 + i as f32 * PRESET_ITEM_H,
            ZOOM_MENU_W,
            PRESET_ITEM_H,
        )
    }

    fn cc_cell_rect(menu: (f32, f32, f32, f32), n: usize) -> (f32, f32, f32, f32) {
        let col = (n % CC_COLS) as f32;
        let row = (n / CC_COLS) as f32;
        (
            menu.0 + 4.0 + col * CC_CELL_W,
            menu.1 + 4.0 + row * CC_CELL_H,
            CC_CELL_W,
            CC_CELL_H,
        )
    }

    fn cc_index_at(x: f32, y: f32, menu: (f32, f32, f32, f32)) -> Option<i32> {
        let col = ((x - menu.0 - 4.0) / CC_CELL_W).floor() as i32;
        let row = ((y - menu.1 - 4.0) / CC_CELL_H).floor() as i32;
        if col >= 0 && row >= 0 && col < CC_COLS as i32 && row < CC_ROWS as i32 {
            Some(row * CC_COLS as i32 + col)
        } else {
            None
        }
    }

    fn hit(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32) -> bool {
        (x..=x + w).contains(&px) && (y..=y + h).contains(&py)
    }

    fn omitted(&self, kit_piece: KitPieceId) -> bool {
        self.active_sof
            .map(|mic| kit_piece.omits_mic(mic))
            .unwrap_or(false)
    }

    fn apply_preset(&self, cx: &mut EventContext, preset: FactoryPreset) {
        match preset {
            FactoryPreset::Init => {
                self.emit_default(cx, &self.params.master_gain);
                self.emit_default(cx, &self.params.sub_kick.vol);
                self.emit_default(cx, &self.params.sub_kick.length);
                self.emit_default(cx, &self.params.sub_kick.dive);
                self.emit_default(cx, &self.params.sub_kick.speed);
                self.emit_default(cx, &self.params.sub_kick.offset);
                for kit_piece in KitPieceId::ALL {
                    let strip = self.params.get_strip(kit_piece);
                    self.emit_default(cx, &strip.gain);
                    self.emit_default(cx, &strip.pan);
                    self.emit_default(cx, &strip.pitch);
                    self.emit_default(cx, &strip.punch);
                    for mic in MicChannel::ALL {
                        self.emit_default(cx, strip.send_for(mic));
                    }
                }
                self.params.vel_maps.reset_all();
                self.params.note_maps.reset_all();
            }
            FactoryPreset::ResetPitch => {
                for kit_piece in KitPieceId::ALL {
                    self.emit_plain(cx, &self.params.get_strip(kit_piece).pitch, 0.0);
                }
            }
            FactoryPreset::ResetPan => {
                for kit_piece in KitPieceId::ALL {
                    self.emit_plain(cx, &self.params.get_strip(kit_piece).pan, 0.0);
                }
            }
            FactoryPreset::ResetPunch => {
                for kit_piece in KitPieceId::ALL {
                    self.emit_plain(cx, &self.params.get_strip(kit_piece).punch, 0.0);
                }
            }
            FactoryPreset::ResetVolumes => {
                let unity = util::db_to_gain(0.0);
                self.emit_plain(cx, &self.params.master_gain, unity);
                for kit_piece in KitPieceId::ALL {
                    let strip = self.params.get_strip(kit_piece);
                    self.emit_plain(cx, &strip.gain, unity);
                    for mic in MicChannel::ALL {
                        self.emit_plain(
                            cx,
                            strip.send_for(mic),
                            util::db_to_gain(default_send_db(mic)),
                        );
                    }
                }
            }
            FactoryPreset::DrummerPerspective => {
                for (kit_piece, pan) in DRUMMER_PANS {
                    self.emit_plain(cx, &self.params.get_strip(kit_piece).pan, pan);
                }
            }
        }
    }
    fn set_live(&self, cx: &mut EventContext, ptr: ParamPtr, norm: f32) {
        cx.emit(RawParamEvent::SetParameterNormalized(
            ptr,
            norm.clamp(0.0, 1.0),
        ));
    }

    fn begin_ptr(&mut self, cx: &mut EventContext, ptr: ParamPtr, slot: usize) {
        if slot < self.gesture.len() {
            self.gesture[slot] = Some(ptr);
        }
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
    }

    fn end_gesture(&mut self, cx: &mut EventContext) {
        for slot in &mut self.gesture {
            if let Some(ptr) = slot.take() {
                cx.emit(RawParamEvent::EndSetParameter(ptr));
            }
        }
    }

    fn fader_pos_at(y: f32) -> f32 {
        let travel = (FADER_H - FADER_HANDLE_H).max(1.0);
        let t = (y - FADER_Y - FADER_HANDLE_H * 0.5) / travel;
        (1.0 - t).clamp(0.0, 1.0)
    }

    fn slider_norm_at(x: f32, strip_x: f32) -> f32 {
        let sx = strip_x + 5.0;
        let sw = STRIP_W - 10.0;
        ((x - sx) / sw).clamp(0.0, 1.0)
    }

    fn strip_x(kit_piece: KitPieceId) -> f32 {
        Self::mixer_x() + kit_piece as usize as f32 * STRIP_W
    }

    fn cc_menu_rect() -> (f32, f32, f32, f32) {
        let w = CC_COLS as f32 * CC_CELL_W + 8.0;
        let h = CC_ROWS as f32 * CC_CELL_H + 8.0;
        (WINDOW_W - w - 8.0, HEADER_H + 4.0, w, h)
    }

    fn apply_fader(&self, cx: &mut EventContext, dragged: KitPieceId, hise_pos: f32) {
        if self.omitted(dragged) {
            return;
        }
        let sof = self.active_sof;
        let dragged_param = self.params.get_strip(dragged).fader_param(sof);
        let new_db = fader_law::pos_to_db(hise_pos);
        if self.params.fader_lock.value() {
            let old_db = util::gain_to_db(dragged_param.unmodulated_plain_value());
            let delta = new_db - old_db;
            for kit_piece in KitPieceId::ALL {
                if self.omitted(kit_piece) {
                    continue;
                }
                let param = self.params.get_strip(kit_piece).fader_param(sof);
                let db = (util::gain_to_db(param.unmodulated_plain_value()) + delta)
                    .clamp(FADER_MIN_DB, FADER_MAX_DB);
                self.set_live(
                    cx,
                    param.as_ptr(),
                    param.preview_normalized(util::db_to_gain(db)),
                );
            }
        } else {
            self.set_live(
                cx,
                dragged_param.as_ptr(),
                dragged_param.preview_normalized(util::db_to_gain(new_db)),
            );
        }
    }

    fn begin_fader_gesture(&mut self, cx: &mut EventContext, dragged: KitPieceId) {
        let sof = self.active_sof;
        if self.params.fader_lock.value() {
            for kit_piece in KitPieceId::ALL {
                if self.omitted(kit_piece) {
                    continue;
                }
                let ptr = self.params.get_strip(kit_piece).fader_param(sof).as_ptr();
                self.begin_ptr(cx, ptr, kit_piece as usize);
            }
        } else {
            let ptr = self.params.get_strip(dragged).fader_param(sof).as_ptr();
            self.begin_ptr(cx, ptr, dragged as usize);
        }
    }

    fn begin_one(&mut self, cx: &mut EventContext, ptr: ParamPtr) {
        self.begin_ptr(cx, ptr, 0);
    }

    fn set_cc_number(&self, cx: &mut EventContext, value: i32) {
        let param = &self.params.cc_number;
        let value = value.clamp(0, 127);
        self.emit_norm(cx, param.as_ptr(), param.preview_normalized(value));
    }

    fn update_hover(&mut self, cx: &mut EventContext) {
        let (x, y) = self.mouse;
        let hover_sof = Self::sof_at(x, y);
        let lock = Self::lock_rect();
        let hover_lock = Self::hit(x, y, lock.0, lock.1, lock.2, lock.3);
        let hover_invert = Self::hit(x, y, CC_INVERT.0, CC_INVERT.1, CC_INVERT.2, CC_INVERT.3);
        let hover_note = self.note_target_at(x, y);
        if hover_sof != self.hover_sof
            || hover_lock != self.hover_lock
            || hover_invert != self.hover_invert
            || hover_note != self.hover_note
        {
            self.hover_sof = hover_sof;
            self.hover_lock = hover_lock;
            self.hover_invert = hover_invert;
            self.hover_note = hover_note;
            cx.needs_redraw();
        }
    }

    fn note_target_at(&self, x: f32, y: f32) -> Option<NoteTarget> {
        if self.samples_open {
            for (i, kit_piece) in KitPieceId::ALL.iter().enumerate() {
                for note2 in [false, true] {
                    let (nx, ny, nw, nh) = Self::mapping_note_rect(i, note2);
                    if Self::hit(x, y, nx, ny, nw, nh) {
                        return Some(NoteTarget {
                            kit_piece: *kit_piece,
                            art: Self::mapping_art(*kit_piece),
                            note2,
                        });
                    }
                }
            }
        }
        None
    }

    fn open_vel_map(&mut self, kit_piece: KitPieceId) {
        self.samples_open = false;
        self.close_preset_menus();
        self.sub_kick_open = false;
        self.vel_hits.clear();
        for arts in &self.params.midi_velocities {
            for velocity in arts {
                velocity.store(0, Ordering::Relaxed);
            }
        }
        self.vel_map_open = Some(kit_piece);
        self.vel_art = 0;
        self.vel_selected_node = None;
        self.vel_just_inserted = false;
        self.blur_dirty.set(true);
    }

    fn vel_curve(&self) -> Option<VelCurve> {
        self.vel_map_open
            .map(|kit_piece| self.params.vel_maps.curve(kit_piece, self.vel_art))
    }

    fn commit_vel_curve(&self, curve: VelCurve) {
        if let Some(kit_piece) = self.vel_map_open {
            Self::set_vel_curve_group(&self.params.vel_maps, kit_piece, self.vel_art, curve);
        }
    }

    fn reset_vel_art(&self, kit_piece: KitPieceId) {
        Self::reset_vel_curve_group(&self.params.vel_maps, kit_piece, self.vel_art);
    }

    fn set_vel_curve_group(
        state: &VelMapState,
        kit_piece: KitPieceId,
        art: usize,
        curve: VelCurve,
    ) {
        for member in Self::vel_art_members(kit_piece, art) {
            state.set_curve(kit_piece, member, curve.clone());
        }
    }

    fn reset_vel_curve_group(state: &VelMapState, kit_piece: KitPieceId, art: usize) {
        for member in Self::vel_art_members(kit_piece, art) {
            state.reset_art(kit_piece, member);
        }
    }

    fn node_delete_pos(curve: &VelCurve, i: usize) -> (f32, f32) {
        let node = &curve.nodes[i];
        let (nx, ny) = Self::norm_to_graph(node.x, node.y);
        let (ix, iy) = Self::norm_to_graph(node.in_handle.x, node.in_handle.y);
        let (ox, oy) = Self::norm_to_graph(node.out_handle.x, node.out_handle.y);
        let (mut dx, mut dy) = (ox - ix, oy - iy);
        if dx.hypot(dy) < 1.0 {
            let (ax, ay) = Self::norm_to_graph(curve.nodes[i - 1].x, curve.nodes[i - 1].y);
            let (bx, by) = Self::norm_to_graph(curve.nodes[i + 1].x, curve.nodes[i + 1].y);
            dx = bx - ax;
            dy = by - ay;
        }
        let len = dx.hypot(dy).max(1.0);
        let offset = (-dy / len * 20.0, dx / len * 20.0);
        let (gx, gy, gw, gh) = Self::vel_map_graph();
        let candidate = (nx + offset.0, ny + offset.1);
        if Self::hit(
            candidate.0,
            candidate.1,
            gx + 8.0,
            gy + 8.0,
            gw - 16.0,
            gh - 16.0,
        ) {
            candidate
        } else {
            (nx - offset.0, ny - offset.1)
        }
    }

    fn hit_vel_delete_x(&self, curve: &VelCurve, x: f32, y: f32) -> Option<usize> {
        let i = self.vel_selected_node?;
        if i == 0 || i + 1 >= curve.nodes.len() {
            return None;
        }
        let (px, py) = Self::node_delete_pos(curve, i);
        ((px - x).hypot(py - y) <= 8.0).then_some(i)
    }

    fn hit_vel_node(&self, curve: &VelCurve, x: f32, y: f32) -> Option<usize> {
        const R: f32 = 8.0;
        curve
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                let (px, py) = Self::norm_to_graph(n.x, n.y);
                let d = (px - x).hypot(py - y);
                (d <= R).then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    fn hit_vel_handle(&self, curve: &VelCurve, x: f32, y: f32) -> Option<(usize, bool)> {
        let i = self.vel_selected_node?;
        let node = curve.nodes.get(i)?;
        const R: f32 = 6.0;
        let last = curve.nodes.len().saturating_sub(1);
        let (ox, oy) = Self::norm_to_graph(node.out_handle.x, node.out_handle.y);
        let (ix, iy) = Self::norm_to_graph(node.in_handle.x, node.in_handle.y);
        let out_d = (ox - x).hypot(oy - y);
        let in_d = (ix - x).hypot(iy - y);
        let out_ok = i < last && out_d <= R;
        let in_ok = i > 0 && in_d <= R;
        if out_ok && (!in_ok || out_d <= in_d) {
            Some((i, true))
        } else if in_ok {
            Some((i, false))
        } else {
            None
        }
    }

    fn note_label(&self, kit_piece: KitPieceId, art: usize, note2: bool) -> String {
        if let Some(edit) = &self.note_edit {
            if edit.target.kit_piece == kit_piece
                && edit.target.art == art
                && edit.target.note2 == note2
            {
                return if edit.text.is_empty() {
                    "_".to_string()
                } else {
                    edit.text.clone()
                };
            }
        }
        let over = self.params.note_maps.art_notes(kit_piece, art);
        if note2 {
            NoteMapStateDisplay::n2(kit_piece, art, over)
        } else {
            format!("{}", NoteMapStateDisplay::n1(kit_piece, art, over))
        }
    }

    fn note_altered(&self, kit_piece: KitPieceId, art: usize, note2: bool) -> bool {
        let over = self.params.note_maps.art_notes(kit_piece, art);
        if note2 {
            crate::note_map::NoteMapState::n2_altered(kit_piece, art, over)
        } else {
            crate::note_map::NoteMapState::n1_altered(kit_piece, art, over)
        }
    }

    fn begin_note_edit(&mut self, cx: &mut EventContext, target: NoteTarget) {
        cx.focus();
        let over = self
            .params
            .note_maps
            .art_notes(target.kit_piece, target.art);
        let buf = if target.note2 {
            NoteMapStateDisplay::n2(target.kit_piece, target.art, over)
        } else {
            format!(
                "{}",
                NoteMapStateDisplay::n1(target.kit_piece, target.art, over)
            )
        };
        let buf = if buf == "-" { String::new() } else { buf };
        let rect = self.note_rect(target);
        self.params.note_maps.arm_learn();
        self.note_edit = Some(ValueEdit::new(target, rect, buf));
    }

    fn activate_note(&mut self, cx: &mut EventContext, target: NoteTarget) {
        if cx.modifiers().alt() {
            self.cancel_note_edit();
            self.params
                .note_maps
                .reset_slot(target.kit_piece, target.art, target.note2);
        } else {
            self.commit_note_edit();
            self.begin_note_edit(cx, target);
        }
        self.press_note = Some(target);
    }

    fn note_edit_hits(edit: &ValueEdit<NoteTarget>, x: f32, y: f32) -> bool {
        Self::hit(x, y, edit.rect.0, edit.rect.1, edit.rect.2, edit.rect.3)
    }

    fn note_rect(&self, target: NoteTarget) -> (f32, f32, f32, f32) {
        KitPieceId::ALL
            .iter()
            .position(|p| *p == target.kit_piece)
            .map(|i| Self::mapping_note_rect(i, target.note2))
            .unwrap_or(Self::mapping_note_rect(0, target.note2))
    }

    fn apply_note_value(&self, target: NoteTarget, raw: &str) {
        if target.note2 {
            if raw.trim().is_empty() {
                let has_factory = stonehouse()
                    .arts(target.kit_piece)
                    .get(target.art)
                    .and_then(|a| a.note2)
                    .is_some();
                self.params
                    .note_maps
                    .set_n2(target.kit_piece, target.art, None, has_factory);
            } else if let Ok(n) = raw.trim().parse::<u8>() {
                if n <= 127 {
                    self.params
                        .note_maps
                        .set_n2(target.kit_piece, target.art, Some(n), false);
                }
            }
        } else if let Ok(n) = raw.trim().parse::<u8>() {
            if n <= 127 {
                let def = stonehouse()
                    .arts(target.kit_piece)
                    .get(target.art)
                    .map(|a| a.note1)
                    .unwrap_or(0);
                if n == def {
                    self.params
                        .note_maps
                        .set_n1(target.kit_piece, target.art, None);
                } else {
                    self.params
                        .note_maps
                        .set_n1(target.kit_piece, target.art, Some(n));
                }
            }
        }
    }

    fn commit_note_edit(&mut self) {
        let Some(edit) = self.note_edit.take() else {
            return;
        };
        self.params.note_maps.disarm_learn();
        self.apply_note_value(edit.target, &edit.text);
    }

    fn cancel_note_edit(&mut self) {
        self.note_edit = None;
        self.params.note_maps.disarm_learn();
    }

    fn apply_note_learn(&mut self) -> bool {
        let Some(edit) = &self.note_edit else {
            return false;
        };
        let Some(n) = self.params.note_maps.take_learn() else {
            return false;
        };
        let target = edit.target;
        self.note_edit = None;
        self.apply_note_value(target, &n.to_string());
        true
    }

    fn note_stage(&self, target: NoteTarget) -> (bool, bool, bool) {
        let on = self.note_edit.as_ref().is_some_and(|e| e.target == target);
        let hover = self.hover_note == Some(target);
        let press = self.press_note == Some(target);
        (on, hover, press)
    }
}

struct NoteMapStateDisplay;

impl NoteMapStateDisplay {
    fn n1(kit_piece: KitPieceId, art: usize, over: crate::note_map::ArtNotes) -> u8 {
        crate::note_map::NoteMapState::effective_n1(kit_piece, art, over)
    }

    fn n2(kit_piece: KitPieceId, art: usize, over: crate::note_map::ArtNotes) -> String {
        crate::note_map::NoteMapState::effective_n2(kit_piece, art, over)
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string())
    }
}

fn nearest_zoom_pct(factor: f64) -> i32 {
    let pct = (factor * 100.0).round() as i32;
    *ZOOM_LEVELS
        .iter()
        .min_by_key(|level| (**level - pct).abs())
        .unwrap_or(&100)
}

fn initial_zoom_pct(params: &ScdParams) -> i32 {
    let mut pct = params.editor_zoom_pct.load(Ordering::Relaxed) as i32;
    if pct == 100 {
        let vizia = params.editor_state.user_scale_factor();
        if (vizia - 1.0).abs() > 1e-6 {
            pct = nearest_zoom_pct(vizia);
            params
                .editor_zoom_pct
                .store(pct.max(0) as u32, Ordering::Relaxed);
        }
    }
    nearest_zoom_pct(pct as f64 / 100.0)
}

fn next_preset_name(existing: &[UserPreset]) -> String {
    for i in 1..=MAX_USER_PRESETS {
        let name = format!("Preset {i}");
        if !existing.iter().any(|p| p.name == name) {
            return name;
        }
    }
    "Preset".to_string()
}

fn draw_spinner(draw: &mut Draw<'_>, cx: f32, cy: f32, color: Color) {
    draw.fill_poly(
        &[(cx, cy - 4.6), (cx - 3.4, cy - 1.1), (cx + 3.4, cy - 1.1)],
        color,
    );
    draw.fill_poly(
        &[(cx, cy + 4.6), (cx - 3.4, cy + 1.1), (cx + 3.4, cy + 1.1)],
        color,
    );
}

fn draw_chevron_right(draw: &mut Draw<'_>, cx: f32, cy: f32, color: Color) {
    draw.fill_poly(
        &[(cx + 3.2, cy), (cx - 1.6, cy - 4.0), (cx - 1.6, cy + 4.0)],
        color,
    );
}

fn draw_magnifier(draw: &mut Draw<'_>, cx: f32, cy: f32, color: Color) {
    draw.circle(cx, cy, 5.2, color, false);
    draw.line(cx + 3.6, cy + 3.6, cx + 7.4, cy + 7.4, color, 1.4);
}

fn ensure_img(slot: &Cell<Option<ImageId>>, canvas: &mut Canvas, bytes: &[u8]) {
    if slot.get().is_none() {
        slot.set(
            canvas
                .load_image_mem(bytes, ImageFlags::GENERATE_MIPMAPS)
                .ok(),
        );
    }
}

fn blit(draw: &mut Draw<'_>, id: ImageId, x: f32, y: f32, w: f32, h: f32) {
    let px = draw.ox + (x + draw.offset_x) * draw.s;
    let py = draw.oy + y * draw.s;
    let pw = w * draw.s;
    let ph = h * draw.s;
    let mut path = Path::new();
    path.rect(px, py, pw, ph);
    draw.c
        .fill_path(&path, &Paint::image(id, px, py, pw, ph, 0.0, 1.0));
}

fn ensure_empty_image(
    canvas: &mut Canvas,
    slot: &Cell<Option<ImageId>>,
    w: usize,
    h: usize,
) -> Option<ImageId> {
    let w = w.max(1);
    let h = h.max(1);
    if let Some(id) = slot.get() {
        if canvas.image_size(id).ok() == Some((w, h)) {
            return Some(id);
        }
        canvas.delete_image(id);
    }
    let flags = ImageFlags::FLIP_Y | ImageFlags::PREMULTIPLIED;
    let id = canvas
        .create_image_empty(w, h, PixelFormat::Rgba8, flags)
        .ok()?;
    slot.set(Some(id));
    Some(id)
}

#[allow(clippy::too_many_arguments)]
fn capture_backdrop_blur(
    draw: &mut Draw<'_>,
    shot_slot: &Cell<Option<ImageId>>,
    src_slot: &Cell<Option<ImageId>>,
    dst_slot: &Cell<Option<ImageId>>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Option<ImageId> {
    let s = draw.s;
    let pad = (GLASS_BLUR_PAD * s).ceil();
    let px = draw.ox + (x + draw.offset_x) * s - pad;
    let py = draw.oy + y * s - pad;
    let pw = w * s + pad * 2.0;
    let ph = h * s + pad * 2.0;
    let iw = pw.ceil().max(1.0) as usize;
    let ih = ph.ceil().max(1.0) as usize;

    let shot = draw.c.screenshot().ok()?;
    let shot_id = if let Some(id) = shot_slot.get() {
        if draw.c.image_size(id).ok() == Some((shot.width(), shot.height())) {
            if draw.c.update_image(id, shot.as_ref(), 0, 0).is_err() {
                return None;
            }
            id
        } else {
            draw.c.delete_image(id);
            let id = draw
                .c
                .create_image(shot.as_ref(), ImageFlags::empty())
                .ok()?;
            shot_slot.set(Some(id));
            id
        }
    } else {
        let id = draw
            .c
            .create_image(shot.as_ref(), ImageFlags::empty())
            .ok()?;
        shot_slot.set(Some(id));
        id
    };

    let src = ensure_empty_image(draw.c, src_slot, iw, ih)?;
    let dst = ensure_empty_image(draw.c, dst_slot, iw, ih)?;
    let shot_w = shot.width() as f32;
    let shot_h = shot.height() as f32;

    draw.c.save();
    draw.c.set_render_target(RenderTarget::Image(src));
    draw.c.reset_scissor();
    draw.c.reset_transform();
    draw.c
        .clear_rect(0, 0, iw as u32, ih as u32, Color::rgba(0, 0, 0, 0));
    let mut region = Path::new();
    region.rect(0.0, 0.0, iw as f32, ih as f32);
    draw.c.fill_path(
        &region,
        &Paint::image(shot_id, -px, -py, shot_w, shot_h, 0.0, 1.0),
    );
    draw.c.filter_image(
        dst,
        ImageFilter::GaussianBlur {
            sigma: (GLASS_BLUR_SIGMA * s).clamp(10.0, 40.0),
        },
        src,
    );
    draw.c.restore();
    draw.c.set_render_target(RenderTarget::Screen);
    Some(dst)
}

fn fill_rounded_image(
    draw: &mut Draw<'_>,
    id: ImageId,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
) {
    let s = draw.s;
    let pad = (GLASS_BLUR_PAD * s).ceil();
    let px = draw.ox + (x + draw.offset_x) * s;
    let py = draw.oy + y * s;
    let pw = w * s;
    let ph = h * s;
    let mut path = Path::new();
    path.rounded_rect(px, py, pw, ph, radius * s);
    draw.c.fill_path(
        &path,
        &Paint::image(
            id,
            px - pad,
            py - pad,
            pw + pad * 2.0,
            ph + pad * 2.0,
            0.0,
            1.0,
        ),
    );
}

#[allow(clippy::too_many_arguments)]
fn fill_rounded_vertical_gradient(
    draw: &mut Draw<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    top: Color,
    bottom: Color,
) {
    let s = draw.s;
    let px = draw.ox + (x + draw.offset_x) * s;
    let py = draw.oy + y * s;
    let pw = w * s;
    let ph = h * s;
    let mut path = Path::new();
    path.rounded_rect(px, py, pw, ph, radius * s);
    draw.c.fill_path(
        &path,
        &Paint::linear_gradient(
            px,
            py,
            px,
            py + ph * 0.55,
            draw.color(top),
            draw.color(bottom),
        ),
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_glass_modal(
    draw: &mut Draw<'_>,
    blur_shot: &Cell<Option<ImageId>>,
    blur_src: &Cell<Option<ImageId>>,
    blur_dst: &Cell<Option<ImageId>>,
    blur_dirty: &Cell<bool>,
    modal_x: f32,
    modal_y: f32,
    modal_w: f32,
    modal_h: f32,
) {
    let radius = 8.0;
    if blur_dirty.get()
        && capture_backdrop_blur(
            draw, blur_shot, blur_src, blur_dst, modal_x, modal_y, modal_w, modal_h,
        )
        .is_some()
    {
        blur_dirty.set(false);
    }

    draw.rect(0.0, 0.0, WINDOW_W, WINDOW_H, SCRIM);
    draw.rounded_rect(
        modal_x + 1.0,
        modal_y + 8.0,
        modal_w,
        modal_h,
        radius,
        GLASS_SHADOW,
    );
    if let Some(blurred) = blur_dst.get() {
        fill_rounded_image(draw, blurred, modal_x, modal_y, modal_w, modal_h, radius);
    }
    draw.rounded_rect(modal_x, modal_y, modal_w, modal_h, radius, GLASS_TINT);
    fill_rounded_vertical_gradient(
        draw,
        modal_x,
        modal_y,
        modal_w,
        modal_h,
        radius,
        GLASS_SHEEN,
        Color {
            a: 0.0,
            ..GLASS_SHEEN
        },
    );
    draw.outline_rounded(
        modal_x + 1.0,
        modal_y + 1.0,
        modal_w - 2.0,
        modal_h - 2.0,
        radius - 1.0,
        GLASS_INNER,
        1.0,
    );
    draw.outline_rounded(modal_x, modal_y, modal_w, modal_h, radius, GLASS_EDGE, 1.25);
}

fn midi_hit_alpha(age: f32) -> f32 {
    (1.0 - (age - 0.5).max(0.0) / 2.0).clamp(0.0, 1.0)
}

fn draw_vel_map_graph(draw: &mut Draw<'_>, curve: &VelCurve, selected: Option<usize>) {
    let (gx, gy, gw, gh) = ScdEditorView::vel_map_graph();
    draw.rounded_rect(gx, gy, gw, gh, 4.0, THEME_DIM);
    let to_pt = |nx: f32, ny: f32| ScdEditorView::norm_to_graph(nx, ny);
    let pts: Vec<(f32, f32)> = curve
        .sample_points(24)
        .into_iter()
        .map(|(nx, ny)| to_pt(nx, ny))
        .collect();
    if pts.len() >= 2 {
        draw.area(&pts, gy + gh, ENV_FILL);
        draw.poly(&pts, ENV_LINE, 1.4);
    }
    if let Some(i) = selected {
        if let Some(node) = curve.nodes.get(i) {
            let (nx, ny) = to_pt(node.x, node.y);
            if i + 1 < curve.nodes.len() {
                let (ox, oy) = to_pt(node.out_handle.x, node.out_handle.y);
                draw.line(nx, ny, ox, oy, THEME_DIM, 1.0);
                draw.circle(ox, oy, 4.0, THEME, true);
            }
            if i > 0 {
                let (ix, iy) = to_pt(node.in_handle.x, node.in_handle.y);
                draw.line(nx, ny, ix, iy, THEME_DIM, 1.0);
                draw.circle(ix, iy, 4.0, THEME, true);
            }
        }
    }
    if let Some(i) = selected.filter(|i| *i > 0 && *i + 1 < curve.nodes.len()) {
        let (dx, dy) = ScdEditorView::node_delete_pos(curve, i);
        draw_delete_x(draw, dx, dy);
    }
    for (i, node) in curve.nodes.iter().enumerate() {
        let (nx, ny) = to_pt(node.x, node.y);
        let r = if selected == Some(i) { 6.0 } else { 5.0 };
        draw.circle(nx, ny, r, THEME, true);
    }
}

fn draw_delete_x(draw: &mut Draw<'_>, cx: f32, cy: f32) {
    let r = 4.5;
    draw.line(cx - r, cy - r, cx + r, cy + r, THEME, 1.3);
    draw.line(cx - r, cy + r, cx + r, cy - r, THEME, 1.3);
}

/// Channel meter from `UI_Meters.js`: a thin beam whose width grows with level,
/// fading from transparent at the peak to opaque blue at the bottom.
fn draw_all_chan_meters(draw: &mut Draw<'_>, kit_vu: &[f32; KitPieceId::COUNT]) {
    let mixer_x = ScdEditorView::mixer_x();
    for (idx, _) in KitPieceId::ALL.iter().enumerate() {
        draw_chan_meter(draw, mixer_x + idx as f32 * STRIP_W, kit_vu[idx]);
    }
}

fn draw_chan_meter(draw: &mut Draw<'_>, strip_x: f32, level: f32) {
    let n = level.clamp(0.0, 1.0);
    if n < 0.008 {
        return;
    }
    let left_h = n * FADER_H;
    let core_w = (0.02 * left_h).max(1.2);
    let peak_y = FADER_Y + FADER_H - left_h;
    let cx = strip_x + STRIP_W * 0.5;

    let glow_w = (core_w * 5.0).min(STRIP_W * 0.55);
    fill_meter_beam(
        draw,
        cx - glow_w * 0.5,
        peak_y,
        glow_w,
        left_h,
        CHAN_METER_TOP,
        Color {
            a: 0.28,
            ..CHAN_METER_BOT
        },
    );
    fill_meter_beam(
        draw,
        cx - core_w * 0.5,
        peak_y,
        core_w,
        left_h,
        CHAN_METER_TOP,
        CHAN_METER_BOT,
    );
}

fn fill_meter_beam(draw: &mut Draw<'_>, x: f32, y: f32, w: f32, h: f32, top: Color, bottom: Color) {
    let s = draw.s;
    let px = draw.ox + (x + draw.offset_x) * s;
    let py = draw.oy + y * s;
    let pw = w * s;
    let ph = h * s;
    let mut path = Path::new();
    path.rect(px, py, pw, ph);
    draw.c.fill_path(
        &path,
        &Paint::linear_gradient(px, py, px, py + ph, draw.color(top), draw.color(bottom)),
    );
}

fn stroke_arc(
    draw: &mut Draw<'_>,
    center: (f32, f32),
    radius: f32,
    angles: (f32, f32),
    color: Color,
    width: f32,
) {
    let px = draw.ox + (center.0 + draw.offset_x) * draw.s;
    let py = draw.oy + center.1 * draw.s;
    let mut path = Path::new();
    path.arc(px, py, radius * draw.s, angles.0, angles.1, Solidity::Hole);
    let mut paint = Paint::color(draw.color(color));
    paint.set_line_width(width * draw.s);
    paint.set_line_cap(LineCap::Butt);
    paint.set_line_join(LineJoin::Round);
    draw.c.stroke_path(&path, &paint);
}

/// HISE rotary: 126° from 12 o'clock, screen-space y-down.
fn hise_knob(
    draw: &mut Draw<'_>,
    cx: f32,
    cy: f32,
    size: f32,
    norm: f32,
    bipolar: bool,
    label: Option<&str>,
) {
    let start_offset = 126.0_f32.to_radians();
    let min_arc = 4.0_f32.to_radians();
    let start_angle = 144.0_f32.to_radians();
    let sweep = 252.0_f32.to_radians();
    let radius = size * 0.42;
    let n = norm.clamp(0.0, 1.0);

    stroke_arc(
        draw,
        (cx, cy),
        radius,
        (start_angle, start_angle + sweep),
        THEME,
        1.35,
    );

    let (arc_start, arc_end) = if bipolar {
        let center = start_angle + sweep * 0.5;
        if n <= 0.5 {
            let scale = 1.0 - (n / 0.5);
            let left = min_arc + scale * (start_offset - min_arc);
            (center - left, center - min_arc)
        } else {
            let scale = (n - 0.5) / 0.5;
            let right = min_arc + scale * (start_offset - min_arc);
            (center + min_arc, center + right)
        }
    } else {
        let sa = start_angle;
        let mut ea = sa + n * sweep;
        ea = ea.max(sa + 2.0 * min_arc);
        (sa, ea)
    };
    if !bipolar || (n - 0.5).abs() > f32::EPSILON {
        stroke_arc(draw, (cx, cy), radius, (arc_start, arc_end), THEME, 5.0);
    }

    if let Some(label) = label {
        let font_size = size * 2.0 / (3.4 * 3.4);
        draw.text_centered(cx, cy + font_size * 0.35, label, font_size, THEME);
    }
}

fn bipolar_slider(draw: &mut Draw<'_>, x: f32, y: f32, w: f32, h: f32, norm: f32) {
    draw.rounded_rect(x, y, w, h, 4.0, THEME_DIM);
    draw.rect(x + w * 0.5 - 0.75, y, 1.5, h, THEME_DIM);
    draw.rounded_rect(x, y, w, h, 4.0, SLIDER_WASH);
    let n = norm.clamp(0.0, 1.0);
    let center = x + w * 0.5;
    let (fill_x, fill_w) = if n < 0.5 {
        let width = (0.5 - n) * w;
        (center - width, width)
    } else {
        (center, (n - 0.5) * w)
    };
    if fill_w > 0.4 {
        let radius = 4.0_f32;
        let left_r = if fill_x <= x + 0.51 {
            radius.min(fill_w * 0.5)
        } else {
            0.0
        };
        let right_r = if fill_x + fill_w >= x + w - 0.51 {
            radius.min(fill_w * 0.5)
        } else {
            0.0
        };
        fill_rounded_varying(
            draw,
            fill_x,
            y,
            fill_w,
            h,
            (left_r, right_r, right_r, left_r),
            THEME,
        );
    }
}

fn fill_rounded_varying(
    draw: &mut Draw<'_>,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radii: (f32, f32, f32, f32),
    color: Color,
) {
    let s = draw.s;
    let px = draw.ox + (x + draw.offset_x) * s;
    let py = draw.oy + y * s;
    let mut path = Path::new();
    path.rounded_rect_varying(
        px,
        py,
        w * s,
        h * s,
        radii.0 * s,
        radii.1 * s,
        radii.2 * s,
        radii.3 * s,
    );
    draw.c.fill_path(&path, &Paint::color(draw.color(color)));
}

fn draw_value_popup(draw: &mut Draw<'_>, cx: f32, y: f32, text: &str, above: bool) {
    let w = (text.len() as f32 * 6.6 + 14.0).max(40.0);
    let h = 18.0;
    let x = cx - w * 0.5;
    let py = if above {
        (y - h - 4.0).max(0.0)
    } else {
        y + 4.0
    };
    draw.rounded_rect(x, py, w, h, 3.0, HEADER);
    draw.outline_rounded(x, py, w, h, 3.0, THEME_DIM, 1.0);
    draw.text_centered(cx, py + h * 0.5 + 4.0, text, 10.0, THEME);
}

fn format_db(gain: f32) -> String {
    let db = util::gain_to_db(gain);
    if db <= -71.5 {
        "-inf dB".to_string()
    } else {
        format!("{db:.1} dB")
    }
}

fn insert_note_digit(edit: &mut ValueEdit<NoteTarget>, ch: char) -> bool {
    if !ch.is_ascii_digit() {
        return false;
    }
    if edit.text.len().saturating_sub(edit.selection().len()) >= 3 {
        return false;
    }
    edit.insert(&ch.to_string());
    true
}

fn five_stage_fill(on: bool, hover: bool, press: bool) -> Option<Color> {
    match (on, hover, press) {
        (_, _, true) => Some(STAGE_DOWN),
        (true, true, false) => Some(STAGE_ON_HOVER),
        (true, false, false) => Some(STAGE_ON),
        (false, true, false) => Some(STAGE_HOVER),
        (false, false, false) => None,
    }
}

fn draw_five_stage(
    draw: &mut Draw<'_>,
    r: (f32, f32, f32, f32),
    radius: f32,
    on: bool,
    hover: bool,
    press: bool,
) {
    if let Some(fill) = five_stage_fill(on, hover, press) {
        draw.rounded_rect(r.0, r.1, r.2, r.3, radius, fill);
    }
}

fn draw_lock(
    draw: &mut Draw<'_>,
    icon_font: Option<FontId>,
    x: f32,
    y: f32,
    size: f32,
    locked: bool,
    hovered: bool,
) {
    let mut color = THEME;
    color.a = if hovered {
        1.0
    } else if locked {
        0.75
    } else {
        0.35
    };
    if let Some(icon_font) = icon_font {
        // Lock and lock-open glyphs from Vizia's bundled Tabler icon font.
        let glyph = if locked { "\u{eb0e}" } else { "\u{eb0f}" };
        let old_font = draw.font.replace(icon_font);
        draw.text_centered(
            x + size * 0.5,
            y + size * 0.5 + size * 0.34,
            glyph,
            size,
            color,
        );
        draw.font = old_font;
    }
}

fn draw_kick_env(
    draw: &mut Draw<'_>,
    rect: (f32, f32, f32, f32),
    vol_gain: f32,
    length: f32,
    offset_ms: f32,
) {
    let (x, y, w, h) = rect;
    let attack_ms = 45.0;
    let hold_ms = (length * 6.0).max(0.0);
    let decay_ms = length_to_decay_ms(length).max(1.0);
    let release_ms = decay_ms;
    let peak_db = util::gain_to_db(vol_gain).clamp(-100.0, 0.0);
    let sustain_db = -100.0;
    let total_ms = attack_ms + hold_ms + decay_ms + release_ms;
    let gx = x + 15.0 + offset_ms;
    let gw = (w - 20.0 - offset_ms).max(8.0);
    let gy = y;
    let gh = h;

    let env_y = |db: f32| gy + gh * (1.0 - ((db + 100.0) / 100.0).clamp(0.0, 1.0));
    let env_x = |ms: f32| gx + gw * (ms / total_ms);

    let mut pts = Vec::with_capacity(20);
    pts.push((env_x(0.0), env_y(sustain_db)));
    const ATTACK_STEPS: usize = 8;
    for i in 1..=ATTACK_STEPS {
        let t = i as f32 / ATTACK_STEPS as f32;
        let shaped = t.powf(0.65);
        let db = sustain_db + (peak_db - sustain_db) * shaped;
        pts.push((env_x(t * attack_ms), env_y(db)));
    }
    let hold_end = attack_ms + hold_ms;
    pts.push((env_x(hold_end), env_y(peak_db)));
    pts.push((env_x(hold_end + decay_ms), env_y(sustain_db)));
    pts.push((env_x(total_ms), env_y(sustain_db)));

    draw.area(&pts, gy + gh, ENV_FILL);
    draw.poly(&pts, ENV_LINE, 1.4);
}

impl View for ScdEditorView {
    fn element(&self) -> Option<&'static str> {
        Some("scd-editor")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|param_event: &RawParamEvent, _| {
            if matches!(param_event, RawParamEvent::ParametersChanged) {
                cx.needs_redraw();
            }
        });
        event.map(|_: &NoteLearnPoll, _| {
            let now = Instant::now();
            self.vel_hits
                .retain(|(_, _, _, at)| now.duration_since(*at).as_secs_f32() < 2.5);
            for piece in KitPieceId::ALL {
                for art in 0..crate::vel_map::MAX_ARTS {
                    let velocity =
                        self.params.midi_velocities[piece as usize][art].swap(0, Ordering::Relaxed);
                    if velocity != 0
                        && self.vel_map_open == Some(piece)
                        && Self::vel_art_members(piece, self.vel_art).contains(&art)
                    {
                        // One trail per velocity keeps memory bounded even under dense MIDI.
                        self.vel_hits
                            .retain(|(p, a, v, _)| (*p, *a, *v) != (piece, art, velocity));
                        self.vel_hits.push((piece, art, velocity, now));
                    }
                }
            }
            if self.apply_note_learn() {
                cx.needs_redraw();
            }
        });
        event.map(|window_event, meta| {
            let scale = self.content_scale(cx.scale_factor()).max(1.0e-6);
            let mouse_x = cx.mouse().cursorx / scale;
            let mouse_y = cx.mouse().cursory / scale;
            let prev_y = self.mouse.1;
            self.mouse = (mouse_x, mouse_y);

            if self.note_edit.is_some() {
                match window_event {
                    WindowEvent::CharInput(ch)
                        if ch.is_ascii_digit() && !cx.modifiers().command() =>
                    {
                        if let Some(edit) = self.note_edit.as_mut() {
                            insert_note_digit(edit, *ch);
                        }
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::KeyDown(code, key) => {
                        match *code {
                            Code::Enter | Code::NumpadEnter => self.commit_note_edit(),
                            Code::Escape => self.cancel_note_edit(),
                            _ => {
                                if let Some(edit) = self.note_edit.as_mut() {
                                    edit.handle_key(cx, *code);
                                    let has_char = matches!(key, Some(Key::Character(_)));
                                    if !has_char && !cx.modifiers().command() {
                                        if let Some(c) = typed_char(*code, cx.modifiers().shift()) {
                                            insert_note_digit(edit, c);
                                        }
                                    }
                                }
                            }
                        }
                        meta.consume();
                        cx.needs_redraw();
                        return;
                    }
                    WindowEvent::MouseDown(MouseButton::Left) => {
                        if self
                            .note_edit
                            .as_ref()
                            .is_some_and(|edit| Self::note_edit_hits(edit, mouse_x, mouse_y))
                        {
                            if let Some(edit) = self.note_edit.as_mut() {
                                edit.handle_mouse_down(mouse_x);
                            }
                            meta.consume();
                            cx.needs_redraw();
                            return;
                        }
                    }
                    WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                        if self
                            .note_edit
                            .as_ref()
                            .is_some_and(|edit| Self::note_edit_hits(edit, mouse_x, mouse_y))
                        {
                            if let Some(edit) = self.note_edit.as_mut() {
                                edit.select_all();
                            }
                            meta.consume();
                            cx.needs_redraw();
                            return;
                        }
                    }
                    WindowEvent::FocusOut => {
                        self.commit_note_edit();
                        cx.needs_redraw();
                    }
                    _ => {}
                }
            }

            match window_event {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    let x = mouse_x;
                    let y = mouse_y;

                    if self.add_preset_open {
                        let (mx, my, mw, mh) = Self::add_preset_modal();
                        if Self::hit(x, y, mx, my, mw, mh) {
                            let name_r = Self::add_name_rect((mx, my, mw, mh));
                            self.add_name_focus =
                                Self::hit(x, y, name_r.0, name_r.1, name_r.2, name_r.3);
                            for i in 0..PresetScope::names().len() {
                                let r = Self::add_scope_rect((mx, my, mw, mh), i);
                                if Self::hit(x, y, r.0, r.1, r.2, r.3) {
                                    self.add_scope.toggle(i);
                                    cx.needs_redraw();
                                    meta.consume();
                                    return;
                                }
                            }
                            let save = Self::add_save_rect((mx, my, mw, mh));
                            if Self::hit(x, y, save.0, save.1, save.2, save.3) {
                                self.save_user_preset(cx);
                                meta.consume();
                                return;
                            }
                            let cancel = Self::add_cancel_rect((mx, my, mw, mh));
                            if Self::hit(x, y, cancel.0, cancel.1, cancel.2, cancel.3) {
                                self.add_preset_open = false;
                                self.add_name_focus = false;
                                cx.needs_redraw();
                                meta.consume();
                                return;
                            }
                            meta.consume();
                            return;
                        } else {
                            self.add_preset_open = false;
                            self.add_name_focus = false;
                            cx.needs_redraw();
                            meta.consume();
                            return;
                        }
                    }

                    if self.cc_menu_open {
                        let (mx, my, mw, mh) = Self::cc_menu_rect();
                        if Self::hit(x, y, mx, my, mw, mh) {
                            if let Some(n) = Self::cc_index_at(x, y, (mx, my, mw, mh)) {
                                self.set_cc_number(cx, n);
                            }
                            self.cc_menu_open = false;
                            cx.needs_redraw();
                            meta.consume();
                            return;
                        }
                        self.cc_menu_open = false;
                        cx.needs_redraw();
                    }

                    if self.zoom_open {
                        for (i, level) in ZOOM_LEVELS.iter().enumerate() {
                            let (ix, iy, iw, ih) = Self::zoom_item_rect(i);
                            if Self::hit(x, y, ix, iy, iw, ih) {
                                self.apply_zoom(cx, *level);
                                meta.consume();
                                return;
                            }
                        }
                    }

                    if self.preset_open {
                        if self.resets_open {
                            let (fx, fy, fw, fh) = Self::reset_flyout_rect();
                            if Self::hit(x, y, fx, fy, fw, fh) {
                                for (i, preset) in FactoryPreset::RESETS.iter().enumerate() {
                                    let (ix, iy, iw, ih) = Self::reset_flyout_item(i);
                                    if Self::hit(x, y, ix, iy, iw, ih) {
                                        self.apply_preset(cx, *preset);
                                        self.close_preset_menus();
                                        cx.needs_redraw();
                                        meta.consume();
                                        return;
                                    }
                                }
                                meta.consume();
                                return;
                            }
                        }
                        for (row, (rx, ry, rw, rh)) in self.preset_menu_rows() {
                            if matches!(row, PresetRow::Rule) {
                                continue;
                            }
                            if !Self::hit(x, y, rx, ry, rw, rh) {
                                continue;
                            }
                            match row {
                                PresetRow::Resets => {
                                    self.resets_open = !self.resets_open;
                                    cx.needs_redraw();
                                }
                                PresetRow::Factory(preset) => {
                                    self.apply_preset(cx, preset);
                                    self.current_preset = PresetPick::Factory(preset);
                                    self.close_preset_menus();
                                    cx.needs_redraw();
                                }
                                PresetRow::User(i) => {
                                    self.apply_user_preset(cx, i);
                                    self.close_preset_menus();
                                    cx.needs_redraw();
                                }
                                PresetRow::Add => {
                                    self.open_add_preset(cx);
                                }
                                PresetRow::Rule => {}
                            }
                            meta.consume();
                            return;
                        }
                    }

                    if self.samples_open {
                        for (i, kit_piece) in KitPieceId::ALL.iter().enumerate() {
                            for note2 in [false, true] {
                                let (nx, ny, nw, nh) = Self::mapping_note_rect(i, note2);
                                if Self::hit(x, y, nx, ny, nw, nh) {
                                    self.activate_note(
                                        cx,
                                        NoteTarget {
                                            kit_piece: *kit_piece,
                                            art: Self::mapping_art(*kit_piece),
                                            note2,
                                        },
                                    );
                                    cx.needs_redraw();
                                    meta.consume();
                                    return;
                                }
                            }
                            let (ix, iy, iw, ih) = Self::mapping_name_rect(i);
                            if Self::hit(x, y, ix, iy, iw, ih) {
                                self.commit_note_edit();
                                self.open_vel_map(*kit_piece);
                                cx.needs_redraw();
                                meta.consume();
                                return;
                            }
                        }
                        let menu_y = SAMPLES_Y + SAMPLES_H + 4.0;
                        let menu_h = (KitPieceId::COUNT + 1) as f32 * PRESET_ITEM_H + 8.0;
                        if Self::hit(x, y, SAMPLES_X, menu_y, SAMPLES_MENU_W, menu_h) {
                            meta.consume();
                            return;
                        }
                    }

                    let (zx, zy, zw, zh) = Self::zoom_combo_rect();
                    if Self::hit(x, y, zx, zy, zw, zh) {
                        self.zoom_open = !self.zoom_open;
                        self.close_preset_menus();
                        self.samples_open = false;
                        self.cc_menu_open = false;
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    } else if self.zoom_open {
                        self.zoom_open = false;
                        cx.needs_redraw();
                    }

                    let (px, py, pw, ph) = Self::preset_combo_rect();
                    if Self::hit(x, y, px, py, pw, ph) {
                        self.preset_open = !self.preset_open;
                        self.resets_open = false;
                        self.zoom_open = false;
                        self.samples_open = false;
                        self.cc_menu_open = false;
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    } else if self.preset_open {
                        self.close_preset_menus();
                        cx.needs_redraw();
                    }

                    let (sx, sy, sw, sh) = Self::samples_combo_rect();
                    if Self::hit(x, y, sx, sy, sw, sh) {
                        self.samples_open = !self.samples_open;
                        self.zoom_open = false;
                        self.close_preset_menus();
                        self.cc_menu_open = false;
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    } else if self.samples_open {
                        self.commit_note_edit();
                        self.samples_open = false;
                        cx.needs_redraw();
                    }

                    if Self::hit(x, y, CC_INVERT.0, CC_INVERT.1, CC_INVERT.2, CC_INVERT.3) {
                        self.cc_menu_open = false;
                        self.close_preset_menus();
                        self.samples_open = false;
                        self.zoom_open = false;
                        self.press_invert = true;
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    }

                    if Self::hit(x, y, CC_SELECT.0, CC_SELECT.1, CC_SELECT.2, CC_SELECT.3) {
                        self.cc_menu_open = !self.cc_menu_open;
                        self.close_preset_menus();
                        self.samples_open = false;
                        self.zoom_open = false;
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    }

                    if let Some(mic) = Self::sof_at(x, y) {
                        if self.active_sof == Some(mic) {
                            self.active_sof = None;
                        } else {
                            self.active_sof = Some(mic);
                        }
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    }

                    if Self::hit(x, y, SUB_KICK.0, SUB_KICK.1, SUB_KICK.2, SUB_KICK.3) {
                        self.sub_kick_open = !self.sub_kick_open;
                        if self.sub_kick_open {
                            self.vel_map_open = None;
                            self.blur_dirty.set(true);
                        }
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    }

                    let lock = Self::lock_rect();
                    if Self::hit(x, y, lock.0, lock.1, lock.2, lock.3) {
                        let cur = self.params.fader_lock.value();
                        self.emit_norm(
                            cx,
                            self.params.fader_lock.as_ptr(),
                            if !cur { 1.0 } else { 0.0 },
                        );
                        cx.needs_redraw();
                        meta.consume();
                        return;
                    }

                    if self.sub_kick_open {
                        let (modal_x, modal_y, modal_w, modal_h) = Self::sub_kick_modal();

                        if Self::hit(x, y, modal_x, modal_y, modal_w, modal_h) {
                            let knob_y = modal_y + 55.0;
                            let knob_radius: f32 = 30.0;

                            for k_idx in 0..5 {
                                let kx = Self::sub_kick_knob_x(modal_x, k_idx);
                                let dist_sq = (x - kx).powi(2) + (y - knob_y).powi(2);
                                if dist_sq <= knob_radius.powi(2) {
                                    let ptr = match k_idx {
                                        0 => self.params.sub_kick.vol.as_ptr(),
                                        1 => self.params.sub_kick.length.as_ptr(),
                                        2 => self.params.sub_kick.dive.as_ptr(),
                                        3 => self.params.sub_kick.speed.as_ptr(),
                                        _ => self.params.sub_kick.offset.as_ptr(),
                                    };
                                    self.drag = Some(match k_idx {
                                        0 => DragTarget::SubKickVol,
                                        1 => DragTarget::SubKickLength,
                                        2 => DragTarget::SubKickDive,
                                        3 => DragTarget::SubKickSpeed,
                                        _ => DragTarget::SubKickOffset,
                                    });
                                    self.begin_one(cx, ptr);
                                    cx.capture();
                                    meta.consume();
                                    return;
                                }
                            }
                        } else {
                            self.sub_kick_open = false;
                            cx.needs_redraw();
                            meta.consume();
                            return;
                        }
                    }

                    if self.vel_map_open.is_some() {
                        let (modal_x, modal_y, modal_w, modal_h) = Self::vel_map_modal();
                        if Self::hit(x, y, modal_x, modal_y, modal_w, modal_h) {
                            if let Some(kit_piece) = self.vel_map_open {
                                let options = Self::vel_art_options(kit_piece);
                                for (_, art, start, count) in Self::vel_headers(kit_piece) {
                                    let (hx, hy, hw, hh) =
                                        Self::vel_group_header_rect(start, count, options.len());
                                    if let Some(art) =
                                        art.filter(|_| Self::hit(x, y, hx, hy, hw, hh))
                                    {
                                        self.commit_note_edit();
                                        self.vel_art = art;
                                        self.vel_selected_node = None;
                                        self.vel_ignore_up = true;
                                        cx.needs_redraw();
                                        meta.consume();
                                        return;
                                    }
                                }
                                for (i, (art, _)) in options.iter().enumerate() {
                                    let (sx, sy, sw, sh) = Self::vel_seg_rect(i, options.len());
                                    if Self::hit(x, y, sx, sy, sw, sh) {
                                        self.commit_note_edit();
                                        self.vel_art = *art;
                                        self.vel_selected_node = None;
                                        cx.needs_redraw();
                                        meta.consume();
                                        return;
                                    }
                                }
                            }
                            if let Some(curve) = self.vel_curve() {
                                if let Some(i) = self.hit_vel_delete_x(&curve, x, y) {
                                    self.vel_ignore_up = true;
                                    let mut curve = curve;
                                    if curve.delete(i) {
                                        self.vel_selected_node = None;
                                        self.commit_vel_curve(curve);
                                    }
                                    cx.needs_redraw();
                                    meta.consume();
                                    return;
                                }
                                if let Some((i, is_out)) = self.hit_vel_handle(&curve, x, y) {
                                    self.drag = Some(if is_out {
                                        DragTarget::VelMapHandleOut
                                    } else {
                                        DragTarget::VelMapHandleIn
                                    });
                                    self.vel_selected_node = Some(i);
                                    self.vel_just_inserted = false;
                                    cx.capture();
                                    meta.consume();
                                    return;
                                }
                                if let Some(i) = self.hit_vel_node(&curve, x, y) {
                                    self.drag = Some(DragTarget::VelMapNode);
                                    self.vel_selected_node = Some(i);
                                    self.vel_just_inserted = false;
                                    cx.capture();
                                    meta.consume();
                                    return;
                                }
                            }
                            self.commit_note_edit();
                            meta.consume();
                            return;
                        } else {
                            self.commit_note_edit();
                            self.vel_map_open = None;
                            self.vel_selected_node = None;
                            cx.needs_redraw();
                            meta.consume();
                            return;
                        }
                    }

                    let mixer_x = Self::mixer_x();
                    for (idx, &kit_piece) in KitPieceId::ALL.iter().enumerate() {
                        let sx = mixer_x + idx as f32 * STRIP_W;
                        if !(sx..=sx + STRIP_W).contains(&x) {
                            continue;
                        }
                        if (PITCH_Y..=PITCH_Y + SLIDER_H).contains(&y) {
                            let ptr = self.params.get_strip(kit_piece).pitch.as_ptr();
                            self.drag = Some(DragTarget::Pitch(kit_piece));
                            self.begin_one(cx, ptr);
                            self.set_live(cx, ptr, Self::slider_norm_at(x, sx));
                            cx.capture();
                            meta.consume();
                            return;
                        }
                        if (PAN_Y..=PAN_Y + SLIDER_H).contains(&y) {
                            let ptr = self.params.get_strip(kit_piece).pan.as_ptr();
                            self.drag = Some(DragTarget::Pan(kit_piece));
                            self.begin_one(cx, ptr);
                            self.set_live(cx, ptr, Self::slider_norm_at(x, sx));
                            cx.capture();
                            meta.consume();
                            return;
                        }
                        if (FADER_Y..=FADER_Y + FADER_H).contains(&y) {
                            if self.omitted(kit_piece) {
                                meta.consume();
                                return;
                            }
                            self.drag = Some(DragTarget::Fader(kit_piece));
                            self.begin_fader_gesture(cx, kit_piece);
                            self.apply_fader(cx, kit_piece, Self::fader_pos_at(y));
                            cx.capture();
                            meta.consume();
                            return;
                        }
                        if (PUNCH_Y..=PUNCH_Y + PUNCH_SIZE).contains(&y) {
                            let ptr = self.params.get_strip(kit_piece).punch.as_ptr();
                            self.drag = Some(DragTarget::Punch(kit_piece));
                            self.begin_one(cx, ptr);
                            cx.capture();
                            meta.consume();
                            return;
                        }
                    }
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    if self.press_invert {
                        self.press_invert = false;
                        if Self::hit(
                            mouse_x,
                            mouse_y,
                            CC_INVERT.0,
                            CC_INVERT.1,
                            CC_INVERT.2,
                            CC_INVERT.3,
                        ) {
                            let on = self.params.invert_cc.unmodulated_plain_value();
                            cx.emit(ParamEvent::BeginSetParameter(&self.params.invert_cc).upcast());
                            cx.emit(ParamEvent::SetParameter(&self.params.invert_cc, !on).upcast());
                            cx.emit(ParamEvent::EndSetParameter(&self.params.invert_cc).upcast());
                        }
                        cx.needs_redraw();
                        meta.consume();
                    }
                    if self.press_note.take().is_some() {
                        cx.needs_redraw();
                        meta.consume();
                    }
                    let was_vel_drag = matches!(
                        self.drag,
                        Some(
                            DragTarget::VelMapNode
                                | DragTarget::VelMapHandleIn
                                | DragTarget::VelMapHandleOut
                        )
                    );
                    if self.drag.is_some() {
                        self.drag = None;
                        self.end_gesture(cx);
                        cx.release();
                        cx.needs_redraw();
                        meta.consume();
                    }
                    if self.vel_ignore_up {
                        self.vel_ignore_up = false;
                    } else if !was_vel_drag && self.vel_map_open.is_some() {
                        let (gx, gy, gw, gh) = Self::vel_map_graph();
                        if Self::hit(mouse_x, mouse_y, gx, gy, gw, gh) {
                            if let Some(mut curve) = self.vel_curve() {
                                if self.hit_vel_node(&curve, mouse_x, mouse_y).is_none() {
                                    let (nx, _) = Self::graph_to_norm(mouse_x, mouse_y);
                                    if let Some(idx) = curve.insert_at(nx) {
                                        self.vel_selected_node = Some(idx);
                                        self.vel_just_inserted = true;
                                        self.commit_vel_curve(curve);
                                        cx.needs_redraw();
                                        meta.consume();
                                    }
                                }
                            }
                        }
                    }
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    if self.vel_map_open.is_some() {
                        let (modal_x, modal_y, modal_w, modal_h) = Self::vel_map_modal();
                        if Self::hit(mouse_x, mouse_y, modal_x, modal_y, modal_w, modal_h) {
                            if let Some(kit_piece) = self.vel_map_open {
                                if let Some(mut curve) = self.vel_curve() {
                                    if let Some(i) = self.hit_vel_node(&curve, mouse_x, mouse_y) {
                                        if self.vel_just_inserted || !curve.delete(i) {
                                            self.reset_vel_art(kit_piece);
                                            self.vel_selected_node = None;
                                        } else {
                                            self.vel_selected_node = None;
                                            self.commit_vel_curve(curve);
                                        }
                                    } else {
                                        self.reset_vel_art(kit_piece);
                                        self.vel_selected_node = None;
                                    }
                                }
                            }
                            self.vel_just_inserted = false;
                            self.vel_ignore_up = true;
                            self.drag = None;
                            cx.needs_redraw();
                            meta.consume();
                        }
                    }
                }
                WindowEvent::MouseMove(..) => {
                    if let Some(drag) = self.drag {
                        let vdelta = -(mouse_y - prev_y) * 0.005;
                        match drag {
                            DragTarget::Pitch(kp) => {
                                let p = &self.params.get_strip(kp).pitch;
                                self.set_live(
                                    cx,
                                    p.as_ptr(),
                                    Self::slider_norm_at(mouse_x, Self::strip_x(kp)),
                                );
                            }
                            DragTarget::Pan(kp) => {
                                let p = &self.params.get_strip(kp).pan;
                                self.set_live(
                                    cx,
                                    p.as_ptr(),
                                    Self::slider_norm_at(mouse_x, Self::strip_x(kp)),
                                );
                            }
                            DragTarget::Punch(kp) => {
                                let p = &self.params.get_strip(kp).punch;
                                self.set_live(
                                    cx,
                                    p.as_ptr(),
                                    p.unmodulated_normalized_value() + vdelta,
                                );
                            }
                            DragTarget::Fader(kp) => {
                                self.apply_fader(cx, kp, Self::fader_pos_at(mouse_y));
                            }
                            DragTarget::SubKickVol => {
                                let p = &self.params.sub_kick.vol;
                                self.set_live(
                                    cx,
                                    p.as_ptr(),
                                    p.unmodulated_normalized_value() + vdelta,
                                );
                            }
                            DragTarget::SubKickLength => {
                                let p = &self.params.sub_kick.length;
                                self.set_live(
                                    cx,
                                    p.as_ptr(),
                                    p.unmodulated_normalized_value() + vdelta,
                                );
                            }
                            DragTarget::SubKickDive => {
                                let p = &self.params.sub_kick.dive;
                                self.set_live(
                                    cx,
                                    p.as_ptr(),
                                    p.unmodulated_normalized_value() + vdelta,
                                );
                            }
                            DragTarget::SubKickSpeed => {
                                let p = &self.params.sub_kick.speed;
                                self.set_live(
                                    cx,
                                    p.as_ptr(),
                                    p.unmodulated_normalized_value() + vdelta,
                                );
                            }
                            DragTarget::SubKickOffset => {
                                let p = &self.params.sub_kick.offset;
                                self.set_live(
                                    cx,
                                    p.as_ptr(),
                                    p.unmodulated_normalized_value() + vdelta,
                                );
                            }
                            DragTarget::VelMapNode => {
                                if let Some(i) = self.vel_selected_node {
                                    if let Some(mut curve) = self.vel_curve() {
                                        let (nx, ny) = Self::graph_to_norm(mouse_x, mouse_y);
                                        curve.move_node(i, nx, ny);
                                        self.commit_vel_curve(curve);
                                    }
                                }
                            }
                            DragTarget::VelMapHandleOut => {
                                if let Some(i) = self.vel_selected_node {
                                    if let Some(mut curve) = self.vel_curve() {
                                        let (nx, ny) = Self::graph_to_xy(mouse_x, mouse_y);
                                        curve.drag_out_handle(i, nx, ny);
                                        self.commit_vel_curve(curve);
                                    }
                                }
                            }
                            DragTarget::VelMapHandleIn => {
                                if let Some(i) = self.vel_selected_node {
                                    if let Some(mut curve) = self.vel_curve() {
                                        let (nx, ny) = Self::graph_to_xy(mouse_x, mouse_y);
                                        curve.drag_in_handle(i, nx, ny);
                                        self.commit_vel_curve(curve);
                                    }
                                }
                            }
                        }
                        cx.needs_redraw();
                        meta.consume();
                    } else {
                        self.update_hover(cx);
                    }
                }
                WindowEvent::MouseOut => {
                    if self.hover_sof.is_some()
                        || self.hover_lock
                        || self.hover_invert
                        || self.hover_note.is_some()
                    {
                        self.hover_sof = None;
                        self.hover_lock = false;
                        self.hover_invert = false;
                        self.hover_note = None;
                        cx.needs_redraw();
                    }
                }
                WindowEvent::MouseScroll(_, dy)
                    if *dy != 0.0
                        && (Self::hit(
                            mouse_x,
                            mouse_y,
                            CC_SELECT.0,
                            CC_SELECT.1,
                            CC_SELECT.2,
                            CC_SELECT.3,
                        ) || self.cc_menu_open) =>
                {
                    let delta = if *dy > 0.0 { 1 } else { -1 };
                    self.set_cc_number(cx, self.params.cc_number.value() + delta);
                    cx.needs_redraw();
                    meta.consume();
                }
                WindowEvent::CharInput(ch)
                    if self.add_preset_open && self.add_name_focus && !ch.is_control() =>
                {
                    if self.add_name.len() < 32 {
                        self.add_name.push(*ch);
                        cx.needs_redraw();
                    }
                    meta.consume();
                }
                WindowEvent::KeyDown(code, _) if self.add_preset_open => {
                    match *code {
                        Code::Backspace if self.add_name_focus => {
                            self.add_name.pop();
                            cx.needs_redraw();
                        }
                        Code::Enter | Code::NumpadEnter => self.save_user_preset(cx),
                        Code::Escape => {
                            self.add_preset_open = false;
                            self.add_name_focus = false;
                            cx.needs_redraw();
                        }
                        _ => {}
                    }
                    meta.consume();
                }
                _ => {}
            }
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        if self.font.get().is_none() {
            self.font.set(
                canvas
                    .add_font_mem(nih_plug_vizia::assets::fonts::NOTO_SANS_REGULAR)
                    .ok(),
            );
        }
        if self.icon_font.get().is_none() {
            self.icon_font.set(
                canvas
                    .add_font_mem(nih_plug_vizia::vizia_assets::fonts::TABLER_ICONS)
                    .ok(),
            );
        }
        ensure_img(&self.bg_img, canvas, BG_PNG);
        ensure_img(&self.logo_img, canvas, LOGO_PNG);
        ensure_img(&self.stone_img, canvas, STONE_PNG);

        let s = self.content_scale(cx.scale_factor());
        let pw = (WINDOW_W * s).round().max(1.0) as u32;
        let ph = (WINDOW_H * s).round().max(1.0) as u32;
        canvas.save();
        canvas.reset_transform();
        canvas.set_size(pw, ph, 1.0);
        let font = self.font.get();
        let mut draw = Draw::new(canvas, false, s, 0.0, 0.0, font);

        let new_peaks = self.params.vu.read_and_clear();
        let mut vu = self.vu_display.get();
        for (display, peak) in vu.iter_mut().zip(new_peaks) {
            *display = display.max(peak_to_meter(peak));
        }
        let new_kit = self.params.vu.read_and_clear_kit();
        let mut kit_vu = self.kit_vu.get();
        for (display, peak) in kit_vu.iter_mut().zip(new_kit) {
            *display = display.max(peak_to_meter(peak));
        }

        if let Some(id) = self.bg_img.get() {
            blit(&mut draw, id, 0.0, 0.0, WINDOW_W, WINDOW_H);
        } else {
            draw.rect(0.0, 0.0, WINDOW_W, WINDOW_H, HEADER);
        }

        draw.rect(0.0, 0.0, WINDOW_W, HEADER_H, HEADER);

        let (zx, zy, zw, zh) = Self::zoom_combo_rect();
        if self.zoom_open {
            draw.rounded_rect(zx, zy + 3.0, zw, zh - 6.0, 4.0, THEME_DIM);
        }
        draw_magnifier(&mut draw, zx + 11.0, HEADER_H * 0.5, THEME);
        let zoom_label = format!("{}", self.zoom_pct);
        draw.text(zx + 22.0, HEADER_H * 0.5 + 4.0, &zoom_label, 12.0, THEME);
        draw_spinner(&mut draw, zx + zw - 11.0, HEADER_H * 0.5, THEME);

        let (px, py, pw, ph) = Self::preset_combo_rect();
        if self.preset_open {
            draw.rounded_rect(px, py + 3.0, pw, ph - 6.0, 4.0, THEME_DIM);
        }
        draw.text(px + 8.0, HEADER_H * 0.5 + 4.0, "Presets", 12.0, THEME);
        draw_spinner(&mut draw, px + pw - 11.0, HEADER_H * 0.5, THEME);

        let (sx, sy, sw, sh) = Self::samples_combo_rect();
        if self.samples_open {
            draw.rounded_rect(sx, sy + 3.0, sw, sh - 6.0, 4.0, THEME_DIM);
        }
        draw.text(sx + 8.0, HEADER_H * 0.5 + 4.0, "Mapping", 12.0, THEME);
        draw_spinner(&mut draw, sx + sw - 11.0, HEADER_H * 0.5, THEME);

        let (vu_x, vu_y, vu_w, vu_h) = MASTER_VU;
        let master_l = vu[12].clamp(0.0, 1.0);
        let master_r = vu[13].clamp(0.0, 1.0);
        if master_l > 0.002 {
            draw.rect(vu_x, vu_y, vu_w * master_l, vu_h * 0.5 - 1.0, VU_FILL);
        }
        if master_r > 0.002 {
            draw.rect(
                vu_x,
                vu_y + vu_h * 0.5 + 1.0,
                vu_w * master_r,
                vu_h * 0.5 - 1.0,
                VU_FILL,
            );
        }

        let invert_on = self.params.invert_cc.value();
        draw_five_stage(
            &mut draw,
            (CC_INVERT.0, 3.0, CC_INVERT.2, HEADER_H - 6.0),
            4.0,
            invert_on,
            self.hover_invert,
            self.press_invert,
        );
        draw.text_centered(
            CC_INVERT.0 + CC_INVERT.2 * 0.5,
            HEADER_H * 0.5 + 4.0,
            "Invert",
            12.0,
            THEME,
        );
        draw.text(CC_LABEL.0, HEADER_H * 0.5 + 4.0, "Hihat CC:", 12.0, THEME);
        if self.cc_menu_open {
            draw.rounded_rect(
                CC_SELECT.0,
                3.0,
                CC_SELECT.2,
                HEADER_H - 6.0,
                4.0,
                THEME_DIM,
            );
        }
        let cc_text = format!("{}", self.params.cc_number.value());
        draw.text(
            CC_SELECT.0 + 4.0,
            HEADER_H * 0.5 + 4.0,
            &cc_text,
            12.0,
            THEME,
        );
        draw_spinner(
            &mut draw,
            CC_SELECT.0 + CC_SELECT.2 - 10.0,
            HEADER_H * 0.5,
            THEME,
        );
        let cc_shown = self.params.cc_display.load(Ordering::Relaxed);
        let cc_live = if cc_shown == 255 {
            "–".to_string()
        } else {
            format!("{cc_shown}")
        };
        draw.text_centered(
            CC_DISPLAY.0 + CC_DISPLAY.2 * 0.5,
            HEADER_H * 0.5 + 4.0,
            &cc_live,
            12.0,
            THEME,
        );

        let mixer_x = Self::mixer_x();
        draw.text(LABEL_X, MIXER_Y + 12.0, "PITCH:", 12.0, THEME);
        draw.text(LABEL_X + 12.0, MIXER_Y + 32.0, "PAN:", 12.0, THEME);
        draw.text(LABEL_X, MIXER_Y + 352.0, "PUNCH:", 12.0, THEME);

        let sc = Self::logo_sc_rect();
        if let Some(id) = self.logo_img.get() {
            blit(&mut draw, id, sc.0, sc.1, sc.2, sc.3);
        }
        let sh = Self::logo_sh_rect();
        if let Some(id) = self.stone_img.get() {
            blit(&mut draw, id, sh.0, sh.1, sh.2, sh.3);
        }

        let lock = Self::lock_rect();
        draw_lock(
            &mut draw,
            self.icon_font.get(),
            lock.0,
            lock.1,
            lock.2,
            self.params.fader_lock.value(),
            self.hover_lock,
        );

        draw.text(SUB_KICK.0, SUB_KICK.1 + 14.0, "Sub Kick", 12.0, THEME);

        for (idx, &kit_piece) in KitPieceId::ALL.iter().enumerate() {
            let sx = mixer_x + idx as f32 * STRIP_W;
            let strip = self.params.get_strip(kit_piece);
            let omitted = self.omitted(kit_piece);
            let slider_x = sx + 5.0;
            let slider_w = STRIP_W - 10.0;

            bipolar_slider(
                &mut draw,
                slider_x,
                PITCH_Y,
                slider_w,
                SLIDER_H,
                strip.pitch.unmodulated_normalized_value(),
            );
            bipolar_slider(
                &mut draw,
                slider_x,
                PAN_Y,
                slider_w,
                SLIDER_H,
                strip.pan.unmodulated_normalized_value(),
            );

            draw_chan_meter(&mut draw, sx, kit_vu[idx]);
            draw.rect(sx + STRIP_W * 0.5 - 1.0, FADER_Y, 2.0, FADER_H, FADER_LINE);

            let gain_db =
                util::gain_to_db(strip.fader_param(self.active_sof).unmodulated_plain_value());
            let gain_pos = fader_law::db_to_pos(gain_db);
            let handle_w = STRIP_W - FADER_HANDLE_INSET * 2.0;
            let handle_x = sx + FADER_HANDLE_INSET;
            let handle_y = FADER_Y + (1.0 - gain_pos) * (FADER_H - FADER_HANDLE_H);
            let handle_color = if omitted {
                THEME_DIM
            } else {
                match self.active_sof {
                    None => FADER_MASTER,
                    Some(mic) => MIC_COLORS[mic as usize],
                }
            };
            if !omitted {
                draw.rounded_rect(
                    handle_x + 1.5,
                    handle_y + 3.0,
                    handle_w - 3.0,
                    FADER_HANDLE_H,
                    5.0,
                    FADER_SHADOW,
                );
            }
            draw.rounded_rect(
                handle_x,
                handle_y,
                handle_w,
                FADER_HANDLE_H,
                5.0,
                handle_color,
            );

            let name = kit_piece.name();
            let split = name.split_once(' ').filter(|_| name.len() > 6);
            if let Some((a, b)) = split {
                draw.text_centered(sx + STRIP_W * 0.5, handle_y + 11.0, a, 10.0, FADER_TEXT);
                draw.text_centered(sx + STRIP_W * 0.5, handle_y + 21.0, b, 10.0, FADER_TEXT);
            } else {
                draw.text_centered(sx + STRIP_W * 0.5, handle_y + 17.0, name, 11.0, FADER_TEXT);
            }

            hise_knob(
                &mut draw,
                sx + STRIP_W * 0.5,
                PUNCH_Y + PUNCH_SIZE * 0.5,
                PUNCH_SIZE,
                strip.punch.unmodulated_normalized_value(),
                true,
                None,
            );
        }

        match self.drag {
            Some(DragTarget::Pitch(kp)) => {
                let text = format!(
                    "{:.1} st",
                    self.params.get_strip(kp).pitch.unmodulated_plain_value()
                );
                draw_value_popup(
                    &mut draw,
                    Self::strip_x(kp) + STRIP_W * 0.5,
                    PITCH_Y,
                    &text,
                    true,
                );
            }
            Some(DragTarget::Pan(kp)) => {
                let text = format!(
                    "{:.0}%",
                    self.params.get_strip(kp).pan.unmodulated_plain_value()
                );
                draw_value_popup(
                    &mut draw,
                    Self::strip_x(kp) + STRIP_W * 0.5,
                    PAN_Y + SLIDER_H,
                    &text,
                    false,
                );
            }
            Some(DragTarget::Fader(kp)) => {
                let param = self.params.get_strip(kp).fader_param(self.active_sof);
                let pos = fader_law::db_to_pos(util::gain_to_db(param.unmodulated_plain_value()));
                let handle_y = FADER_Y + (1.0 - pos) * (FADER_H - FADER_HANDLE_H);
                draw_value_popup(
                    &mut draw,
                    Self::strip_x(kp) + STRIP_W * 0.5,
                    handle_y,
                    &format_db(param.unmodulated_plain_value()),
                    true,
                );
            }
            _ => {}
        }

        for (i, mic) in MicChannel::ALL.iter().enumerate() {
            let (bx, by, bw, bh) = Self::sof_rect(i);
            let is_active = self.active_sof == Some(*mic);
            let bg = if is_active {
                MIC_COLORS[*mic as usize]
            } else if self.hover_sof == Some(*mic) {
                SOF_HOVER
            } else {
                SOF_OFF
            };
            draw.rounded_rect(bx, by, bw, bh, 5.0, bg);
            draw.text_centered(
                bx + bw * 0.5,
                by + bh * 0.5 + 4.0,
                mic.name(),
                11.0,
                if is_active { FADER_TEXT } else { THEME },
            );
        }

        if self.sub_kick_open {
            let (modal_x, modal_y, modal_w, modal_h) = Self::sub_kick_modal();
            paint_glass_modal(
                &mut draw,
                &self.blur_shot,
                &self.blur_src,
                &self.blur_dst,
                &self.blur_dirty,
                modal_x,
                modal_y,
                modal_w,
                modal_h,
            );
            draw_all_chan_meters(&mut draw, &kit_vu);

            let knobs = [
                (
                    Self::sub_kick_knob_x(modal_x, 0),
                    "VOL",
                    self.params.sub_kick.vol.unmodulated_normalized_value(),
                    false,
                ),
                (
                    Self::sub_kick_knob_x(modal_x, 1),
                    "LENGTH",
                    self.params.sub_kick.length.unmodulated_normalized_value(),
                    false,
                ),
                (
                    Self::sub_kick_knob_x(modal_x, 2),
                    "DIVE",
                    self.params.sub_kick.dive.unmodulated_normalized_value(),
                    true,
                ),
                (
                    Self::sub_kick_knob_x(modal_x, 3),
                    "SPEED",
                    self.params.sub_kick.speed.unmodulated_normalized_value(),
                    false,
                ),
                (
                    Self::sub_kick_knob_x(modal_x, 4),
                    "OFFSET",
                    self.params.sub_kick.offset.unmodulated_normalized_value(),
                    false,
                ),
            ];
            for (kx, label, norm, bipolar) in knobs {
                hise_knob(
                    &mut draw,
                    kx,
                    modal_y + 55.0,
                    60.0,
                    norm,
                    bipolar,
                    Some(label),
                );
            }
            draw_kick_env(
                &mut draw,
                (modal_x, modal_y + 90.0, modal_w, 60.0),
                self.params.sub_kick.vol.unmodulated_plain_value(),
                self.params.sub_kick.length.unmodulated_plain_value(),
                self.params.sub_kick.offset.unmodulated_plain_value(),
            );
        }

        if let Some(kit_piece) = self.vel_map_open {
            let (modal_x, modal_y, modal_w, modal_h) = Self::vel_map_modal();
            paint_glass_modal(
                &mut draw,
                &self.blur_shot,
                &self.blur_src,
                &self.blur_dst,
                &self.blur_dirty,
                modal_x,
                modal_y,
                modal_w,
                modal_h,
            );
            draw_all_chan_meters(&mut draw, &kit_vu);
            draw.text_centered(
                modal_x + modal_w * 0.5,
                modal_y + 26.0,
                &format!("{} velocity map", kit_piece.name()),
                14.0,
                THEME,
            );
            draw.text(
                modal_x + 24.0,
                modal_y + 48.0,
                "Maps incoming MIDI velocity to sample layers.",
                11.0,
                THEME,
            );
            draw.text(
                modal_x + 24.0,
                modal_y + 64.0,
                "Click the curve to add a point. Drag nodes and handles to shape it.",
                11.0,
                THEME,
            );
            let arts = stonehouse().arts(kit_piece);
            let art_i = self.vel_art.min(arts.len().saturating_sub(1));
            let members = Self::vel_art_members(kit_piece, self.vel_art);
            let art_layers: u32 = members
                .iter()
                .filter_map(|i| arts.get(*i))
                .map(|a| a.layers)
                .sum();
            let piece_layers: u32 = arts.iter().map(|a| a.layers).sum();
            let art_label = Self::vel_art_options(kit_piece)
                .into_iter()
                .find(|(art, _)| *art == self.vel_art)
                .map(|(_, label)| label)
                .or_else(|| arts.get(art_i).map(|a| a.short_name()))
                .unwrap_or_else(|| "Hit".to_string());
            draw.text(
                modal_x + 24.0,
                modal_y + 88.0,
                &format!(
                    "{art_layers} samples in {art_label}  ·  {piece_layers} in {}",
                    kit_piece.name()
                ),
                11.0,
                THEME,
            );
            let options = Self::vel_art_options(kit_piece);
            let n = options.len();
            let (tx, ty, tw, th) = Self::vel_switch_track();
            draw.rounded_rect(tx, ty, tw, th, th * 0.5, SOF_OFF);
            for (header, art, start, count) in Self::vel_headers(kit_piece) {
                let (hx, hy, hw, hh) = Self::vel_group_header_rect(start, count, n);
                let (hx, hw) = (hx + 2.0, hw - 4.0);
                let fill = if art == Some(self.vel_art) {
                    THEME_DIM
                } else {
                    SOF_OFF
                };
                fill_rounded_varying(&mut draw, hx, hy, hw, hh, (6.0, 6.0, 0.0, 0.0), fill);
                draw.line(hx, hy + 6.0, hx, ty + th, THEME_DIM, 1.0);
                draw.line(hx + hw, hy + 6.0, hx + hw, ty + th, THEME_DIM, 1.0);
                draw.text_centered(hx + hw * 0.5, hy + hh * 0.5 + 3.5, &header, 10.0, THEME);
            }
            if let Some(selected) = options.iter().position(|(art, _)| *art == self.vel_art) {
                let (sx, sy, sw, sh) = Self::vel_seg_rect(selected, n);
                draw.rounded_rect(sx, sy, sw, sh, sh * 0.5, THEME_DIM);
            }
            let font = if n > 8 { 9.0 } else { 10.0 };
            for (i, (_, label)) in options.iter().enumerate() {
                let (sx, sy, sw, sh) = Self::vel_seg_rect(i, n);
                draw.text_centered(sx + sw * 0.5, sy + sh * 0.5 + 4.0, label, font, THEME);
            }
            let curve = self.params.vel_maps.curve(kit_piece, self.vel_art);
            draw_vel_map_graph(&mut draw, &curve, self.vel_selected_node);
            let (gx, gy, gw, gh) = Self::vel_map_graph();
            for &(piece, art, velocity, at) in &self.vel_hits {
                if piece == kit_piece && members.contains(&art) {
                    let alpha = midi_hit_alpha(at.elapsed().as_secs_f32());
                    let x = gx + (velocity as f32 - 1.0) / 126.0 * gw;
                    draw.line(x, gy, x, gy + gh, Color::rgbaf(1.0, 0.85, 0.4, alpha), 1.5);
                }
            }
        }

        if self.zoom_open {
            let (_, menu_y, _, _) = Self::zoom_item_rect(0);
            let menu_y = menu_y - 4.0;
            let menu_h = ZOOM_LEVELS.len() as f32 * PRESET_ITEM_H + 8.0;
            draw.rounded_rect(ZOOM_X, menu_y, ZOOM_MENU_W, menu_h, 4.0, HEADER);
            draw.outline_rounded(ZOOM_X, menu_y, ZOOM_MENU_W, menu_h, 4.0, THEME_DIM, 1.0);
            for (i, level) in ZOOM_LEVELS.iter().enumerate() {
                let (ix, iy, iw, ih) = Self::zoom_item_rect(i);
                if *level == self.zoom_pct {
                    draw.rounded_rect(ix + 4.0, iy, iw - 8.0, ih, 3.0, THEME_DIM);
                }
                draw.text(ix + 12.0, iy + 15.0, &format!("{level}%"), 11.0, THEME);
            }
        }

        if self.preset_open {
            let (mx, my, mw, mh) = self.preset_menu_rect();
            draw.rounded_rect(mx, my, mw, mh, 4.0, HEADER);
            draw.outline_rounded(mx, my, mw, mh, 4.0, THEME_DIM, 1.0);
            let users = self.params.user_presets.list();
            for (row, (ix, iy, iw, ih)) in self.preset_menu_rows() {
                match row {
                    PresetRow::Rule => {
                        draw.rect(ix, iy, iw, ih, THEME_DIM);
                    }
                    PresetRow::Resets => {
                        if self.resets_open {
                            draw.rounded_rect(ix + 4.0, iy, iw - 8.0, ih, 3.0, THEME_DIM);
                        }
                        draw.text(ix + 12.0, iy + 15.0, "Resets", 11.0, THEME);
                        draw_chevron_right(&mut draw, ix + iw - 14.0, iy + ih * 0.5, THEME);
                    }
                    PresetRow::Factory(preset) => {
                        if self.current_preset == PresetPick::Factory(preset) {
                            draw.rounded_rect(ix + 4.0, iy, iw - 8.0, ih, 3.0, THEME_DIM);
                        }
                        draw.text(ix + 12.0, iy + 15.0, preset.name(), 11.0, THEME);
                    }
                    PresetRow::User(i) => {
                        if self.current_preset == PresetPick::User(i) {
                            draw.rounded_rect(ix + 4.0, iy, iw - 8.0, ih, 3.0, THEME_DIM);
                        }
                        let label = users.get(i).map(|p| p.name.as_str()).unwrap_or("Preset");
                        draw.text(ix + 12.0, iy + 15.0, label, 11.0, THEME);
                    }
                    PresetRow::Add => {
                        draw.text(ix + 12.0, iy + 15.0, "+ Add Preset", 11.0, THEME);
                    }
                }
            }
            if self.resets_open {
                let (fx, fy, fw, fh) = Self::reset_flyout_rect();
                draw.rounded_rect(fx, fy, fw, fh, 4.0, HEADER);
                draw.outline_rounded(fx, fy, fw, fh, 4.0, THEME_DIM, 1.0);
                for (i, preset) in FactoryPreset::RESETS.iter().enumerate() {
                    let (ix, iy, _, _) = Self::reset_flyout_item(i);
                    draw.text(ix + 12.0, iy + 15.0, preset.name(), 11.0, THEME);
                }
            }
        }

        if self.samples_open {
            let (_, menu_y, _, _) = Self::mapping_header_rect();
            let menu_y = menu_y - 4.0;
            let menu_h = (KitPieceId::COUNT + 1) as f32 * PRESET_ITEM_H + 8.0;
            draw.rounded_rect(SAMPLES_X, menu_y, SAMPLES_MENU_W, menu_h, 4.0, HEADER);
            draw.outline_rounded(
                SAMPLES_X,
                menu_y,
                SAMPLES_MENU_W,
                menu_h,
                4.0,
                THEME_DIM,
                1.0,
            );
            let (hx, hy, _, _) = Self::mapping_header_rect();
            draw.text(hx + 8.0, hy + 15.0, "N1", 10.0, THEME);
            draw.text(hx + 8.0 + MAP_NOTE_W, hy + 15.0, "N2", 10.0, THEME);
            draw.text(hx + 8.0 + MAP_NOTE_W * 2.0, hy + 15.0, "Piece", 10.0, THEME);
            for (i, kit_piece) in KitPieceId::ALL.iter().enumerate() {
                let (ix, iy, iw, ih) = Self::samples_item_rect(i);
                if self.vel_map_open == Some(*kit_piece) {
                    draw.rounded_rect(ix + 4.0, iy, iw - 8.0, ih, 3.0, THEME_DIM);
                }
                for note2 in [false, true] {
                    let (nx, ny, nw, nh) = Self::mapping_note_rect(i, note2);
                    let art = Self::mapping_art(*kit_piece);
                    let target = NoteTarget {
                        kit_piece: *kit_piece,
                        art,
                        note2,
                    };
                    let color = if self.note_altered(*kit_piece, art, note2) {
                        NOTE_YELLOW
                    } else {
                        THEME
                    };
                    if let Some(edit) = self.note_edit.as_ref().filter(|e| e.target == target) {
                        draw.value_edit(edit, color);
                        continue;
                    }
                    let (on, hover, press) = self.note_stage(target);
                    draw_five_stage(&mut draw, (nx, ny, nw, nh), 5.0, on, hover, press);
                    let label = self.note_label(*kit_piece, art, note2);
                    draw.text_centered(nx + nw * 0.5, ny + nh * 0.5 + 4.0, &label, 10.0, color);
                }
                draw.text(
                    ix + 8.0 + MAP_NOTE_W * 2.0,
                    iy + 15.0,
                    kit_piece.name(),
                    11.0,
                    THEME,
                );
            }
        }

        if self.cc_menu_open {
            let (mx, my, mw, mh) = Self::cc_menu_rect();
            draw.rounded_rect(mx, my, mw, mh, 4.0, HEADER);
            draw.outline_rounded(mx, my, mw, mh, 4.0, THEME_DIM, 1.0);
            let current = self.params.cc_number.value();
            for n in 0..128 {
                let (cx_cell, cy_cell, cw, ch) = Self::cc_cell_rect((mx, my, mw, mh), n);
                if n as i32 == current {
                    draw.rounded_rect(cx_cell, cy_cell, cw - 2.0, ch - 2.0, 2.0, THEME_DIM);
                }
                draw.text_centered(
                    cx_cell + cw * 0.5 - 1.0,
                    cy_cell + ch * 0.5 + 4.0,
                    &format!("{n}"),
                    9.0,
                    THEME,
                );
            }
        }

        if self.add_preset_open {
            let (mx, my, mw, mh) = Self::add_preset_modal();
            paint_glass_modal(
                &mut draw,
                &self.blur_shot,
                &self.blur_src,
                &self.blur_dst,
                &self.blur_dirty,
                mx,
                my,
                mw,
                mh,
            );
            draw_all_chan_meters(&mut draw, &kit_vu);
            draw.text(mx + 20.0, my + 28.0, "Add Preset", 14.0, THEME);
            let name_r = Self::add_name_rect((mx, my, mw, mh));
            draw.rounded_rect(name_r.0, name_r.1, name_r.2, name_r.3, 4.0, THEME_DIM);
            if self.add_name_focus {
                draw.outline_rounded(name_r.0, name_r.1, name_r.2, name_r.3, 4.0, THEME, 1.0);
            }
            let name_label = if self.add_name.is_empty() {
                "Name"
            } else {
                self.add_name.as_str()
            };
            draw.text(
                name_r.0 + 8.0,
                name_r.1 + 19.0,
                name_label,
                12.0,
                if self.add_name.is_empty() {
                    THEME_DIM
                } else {
                    THEME
                },
            );
            for i in 0..PresetScope::names().len() {
                let r = Self::add_scope_rect((mx, my, mw, mh), i);
                let on = self.add_scope.get(i);
                draw.rounded_rect(
                    r.0,
                    r.1 + 4.0,
                    16.0,
                    16.0,
                    3.0,
                    if on { THEME } else { THEME_DIM },
                );
                if on {
                    draw.rounded_rect(r.0 + 4.0, r.1 + 8.0, 8.0, 8.0, 2.0, FADER_TEXT);
                }
                draw.text(r.0 + 24.0, r.1 + 17.0, PresetScope::names()[i], 12.0, THEME);
            }
            let save = Self::add_save_rect((mx, my, mw, mh));
            let cancel = Self::add_cancel_rect((mx, my, mw, mh));
            let can_save = !self.add_name.trim().is_empty() && self.add_scope.any();
            draw.rounded_rect(
                save.0,
                save.1,
                save.2,
                save.3,
                4.0,
                if can_save { THEME_DIM } else { SOF_OFF },
            );
            draw.text_centered(save.0 + save.2 * 0.5, save.1 + 18.0, "Save", 12.0, THEME);
            draw.rounded_rect(cancel.0, cancel.1, cancel.2, cancel.3, 4.0, THEME_DIM);
            draw.text_centered(
                cancel.0 + cancel.2 * 0.5,
                cancel.1 + 18.0,
                "Cancel",
                12.0,
                THEME,
            );
        }

        for v in vu.iter_mut() {
            *v *= VU_DECAY;
        }
        self.vu_display.set(vu);
        for v in kit_vu.iter_mut() {
            *v *= VU_DECAY;
        }
        self.kit_vu.set(kit_vu);
        draw.c.restore();
    }
}

pub fn create(params: Arc<ScdParams>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            nih_plug_vizia::assets::register_noto_sans_regular(cx);
            ScdEditorView::new(cx, params.clone())
                .width(Stretch(1.0))
                .height(Stretch(1.0))
                .focusable(true);
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::editor_logical_size;

    #[test]
    fn mapping_headers_select_base_samples_without_duplicate_choices() {
        let options = ScdEditorView::vel_art_options(KitPieceId::OpenSnare);
        assert_eq!(
            options.iter().map(|(art, _)| *art).collect::<Vec<_>>(),
            [0, 1, 2, 3, 4, 7, 8]
        );
        let headers = ScdEditorView::vel_headers(KitPieceId::OpenSnare);
        assert_eq!(
            headers,
            vec![
                ("Center".into(), Some(5), 1, 2),
                ("Edge".into(), Some(6), 3, 2)
            ]
        );
        for (_, _, start, count) in headers {
            let (hx, _, hw, _) = ScdEditorView::vel_group_header_rect(start, count, options.len());
            let (left, _, _, _) = ScdEditorView::vel_seg_rect(start, options.len());
            let (right, _, width, _) =
                ScdEditorView::vel_seg_rect(start + count - 1, options.len());
            assert!(left > hx + 2.0 && right + width < hx + hw - 2.0);
        }
    }

    #[test]
    fn mapping_delete_is_near_node_and_perpendicular_to_tangent() {
        let mut curve = VelCurve::identity();
        let i = curve.insert_at(0.5).unwrap();
        let node = &curve.nodes[i];
        let (nx, ny) = ScdEditorView::norm_to_graph(node.x, node.y);
        let (ix, iy) = ScdEditorView::norm_to_graph(node.in_handle.x, node.in_handle.y);
        let (ox, oy) = ScdEditorView::norm_to_graph(node.out_handle.x, node.out_handle.y);
        let (dx, dy) = ScdEditorView::node_delete_pos(&curve, i);
        assert!(((dx - nx).hypot(dy - ny) - 20.0).abs() < 0.001);
        assert!(((dx - nx) * (ox - ix) + (dy - ny) * (oy - iy)).abs() < 0.02);
    }

    #[test]
    fn mapping_midi_holds_then_fades_for_two_seconds() {
        assert_eq!(midi_hit_alpha(0.0), 1.0);
        assert_eq!(midi_hit_alpha(0.5), 1.0);
        assert_eq!(midi_hit_alpha(1.5), 0.5);
        assert_eq!(midi_hit_alpha(2.5), 0.0);
        assert_eq!(midi_hit_alpha(3.0), 0.0);
    }

    #[test]
    fn nearest_zoom_snaps_to_preset_levels() {
        assert_eq!(nearest_zoom_pct(1.0), 100);
        assert_eq!(nearest_zoom_pct(0.85), 85);
        assert_eq!(nearest_zoom_pct(1.5), 150);
        assert_eq!(nearest_zoom_pct(1.4), 150);
        assert_eq!(nearest_zoom_pct(0.6), 50);
    }

    #[test]
    fn note_digit_replaces_selection_and_caps_at_three() {
        let target = NoteTarget {
            kit_piece: KitPieceId::Kick,
            art: 0,
            note2: false,
        };
        let mut edit = ValueEdit::new(target, (0.0, 0.0, 36.0, 28.0), "36".into());
        assert!(insert_note_digit(&mut edit, '4'));
        assert_eq!(edit.text, "4");
        assert!(insert_note_digit(&mut edit, '2'));
        assert!(insert_note_digit(&mut edit, '0'));
        assert!(!insert_note_digit(&mut edit, '1'));
        assert_eq!(edit.text, "420");
        assert!(!insert_note_digit(&mut edit, 'a'));
    }

    #[test]
    fn five_stage_covers_hise_filmstrip() {
        assert_eq!(five_stage_fill(false, false, false), None);
        assert_eq!(five_stage_fill(false, true, false), Some(STAGE_HOVER));
        assert_eq!(five_stage_fill(false, true, true), Some(STAGE_DOWN));
        assert_eq!(five_stage_fill(true, false, false), Some(STAGE_ON));
        assert_eq!(five_stage_fill(true, true, false), Some(STAGE_ON_HOVER));
        assert_eq!(five_stage_fill(true, false, true), Some(STAGE_DOWN));
    }

    #[test]
    fn hihat_mapping_groups_route_to_all_underlying_articulations() {
        let options = ScdEditorView::vel_art_options(KitPieceId::Hihat);
        assert_eq!(
            options.iter().map(|(art, _)| *art).collect::<Vec<_>>(),
            [0, 1, 2, 7]
        );
        assert_eq!(
            ScdEditorView::vel_art_members(KitPieceId::Hihat, 2),
            vec![2, 3, 4, 5, 6]
        );
        assert_eq!(
            ScdEditorView::vel_art_members(KitPieceId::Hihat, 7),
            vec![7, 8, 9, 10, 11]
        );
        assert_eq!(
            ScdEditorView::vel_art_members(KitPieceId::Hihat, 1),
            vec![1]
        );
    }

    #[test]
    fn hihat_mapping_group_edit_and_reset_fan_out() {
        let state = VelMapState::identity();
        let mut curve = VelCurve::identity();
        curve.move_node(0, 0.0, 1.0);
        ScdEditorView::set_vel_curve_group(&state, KitPieceId::Hihat, 2, curve);
        for art in 2..7 {
            assert_eq!(state.lookup(KitPieceId::Hihat, art, 64), 127);
        }
        assert_eq!(state.lookup(KitPieceId::Hihat, 1, 64), 64);
        ScdEditorView::reset_vel_curve_group(&state, KitPieceId::Hihat, 2);
        for art in 2..7 {
            assert_eq!(state.curve(KitPieceId::Hihat, art), VelCurve::identity());
        }
        assert_eq!(state.lookup(KitPieceId::Hihat, 1, 64), 64);
    }

    #[test]
    fn initial_zoom_migrates_legacy_vizia_scale() {
        let params = ScdParams::default();
        // `ViziaState::scale_factor` is private; spawn-time user scale defaults to 1.
        assert_eq!(initial_zoom_pct(&params), 100);
        params.editor_zoom_pct.store(125, Ordering::Relaxed);
        assert_eq!(initial_zoom_pct(&params), 125);
        assert_eq!(
            params.editor_state.inner_logical_size(),
            editor_logical_size(1.25)
        );
    }
}
