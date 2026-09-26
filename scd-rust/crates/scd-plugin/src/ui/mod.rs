mod drawing;
use drawing::*;
mod events;
mod fader_law;
mod geometry;
use geometry::*;
mod mapping;
use mapping::*;
mod mixer;
mod presets;
use presets::*;
mod render;
mod sub_kick;
mod velocity_map;
#[cfg(test)]
use velocity_map::midi_hit_alpha;

use crate::dsp::kick_sine::length_to_decay_ms;
use crate::params::{
    peak_to_meter, ScdParams, EDITOR_HEIGHT, EDITOR_WIDTH, FADER_MAX_DB, FADER_MIN_DB,
    VU_PEAK_COUNT,
};
use crate::presets::{
    default_send_db, FactoryPreset, PresetScope, UserPreset, DRUMMER_PANS, MAX_USER_PRESETS,
};
use crate::vel_map::{VelCurve, VelMapState, ALL_ART};
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
    widgets::{util::ModifiersExt, ParamEvent, RawParamEvent},
    ViziaTheming,
};
use pleasant_ui::{draw::Draw, theme::rgb, typed_char, ValueEdit};
use scd_core::kit::stonehouse;
use scd_core::{KitPieceId, MicChannel};
use std::cell::Cell;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const WINDOW_W: f32 = EDITOR_WIDTH as f32;
pub const WINDOW_H: f32 = EDITOR_HEIGHT as f32;

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
    vel_art_menu_open: bool,
    vel_selected_node: Option<usize>,
    vel_just_inserted: bool,
    vel_ignore_up: bool,
    vel_hits: Vec<(KitPieceId, usize, u8, Instant)>,
    note_edit: Option<ValueEdit<NoteTarget>>,
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
    bg_img: Cell<Option<ImageId>>,
    logo_img: Cell<Option<ImageId>>,
    stone_img: Cell<Option<ImageId>>,
    blur_shot: Cell<Option<ImageId>>,
    blur_src: Cell<Option<ImageId>>,
    blur_dst: Cell<Option<ImageId>>,
    blur_dirty: Cell<bool>,
    last_draw_size: Cell<(f32, f32)>,
    /// Peak-held decaying display values.
    /// Layout: [mic0_L, mic0_R, mic1_L, mic1_R, ... mic5_R, master_L, master_R] = 14
    vu_display: Cell<[f32; VU_PEAK_COUNT]>,
    kit_vu: Cell<[f32; KitPieceId::COUNT]>,
}

impl ScdEditorView {
    pub fn new(cx: &mut Context, params: Arc<ScdParams>) -> Handle<'_, Self> {
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
            vel_art: ALL_ART,
            vel_art_menu_open: false,
            vel_selected_node: None,
            vel_just_inserted: false,
            vel_ignore_up: false,
            vel_hits: Vec::new(),
            note_edit: None,
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
            bg_img: Cell::new(None),
            logo_img: Cell::new(None),
            stone_img: Cell::new(None),
            blur_shot: Cell::new(None),
            blur_src: Cell::new(None),
            blur_dst: Cell::new(None),
            blur_dirty: Cell::new(true),
            last_draw_size: Cell::new((0.0, 0.0)),
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

    fn begin_one(&mut self, cx: &mut EventContext, ptr: ParamPtr) {
        self.begin_ptr(cx, ptr, 0);
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
}

impl View for ScdEditorView {
    fn element(&self) -> Option<&'static str> {
        Some("scd-editor")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let was_editing = self.note_edit.is_some();
        self.handle_event(cx, event);
        pleasant_ui::value_edit::sync_text_input(cx, was_editing, self.note_edit.is_some());
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        self.draw_content(cx, canvas);
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
mod tests;
