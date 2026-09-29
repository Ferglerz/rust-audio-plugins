mod chords;
mod events;
mod layout;
mod mapping;
mod memories;
use memories::MemoryDrag;
mod performance;
mod render;
use crate::{
    bridge::Bridge,
    engine::{Command, Snapshot, POINTER_KEY_OFFSET},
    harmony::{self, SavedChord, KEY_COLUMNS, KEY_COUNT},
    params::{ChordboardParams, Control},
};
use layout::*;
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{
        prelude::*,
        vg::{Color, FontId},
    },
    widgets::{util::ModifiersExt, RawParamEvent},
    ViziaTheming,
};
use pleasant_ui::{
    theme::{BG, COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT},
    AppearanceStore, Draw, ValueEdit, FONT_JETBRAINS_MONO,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc, OnceLock},
    time::{Duration, Instant},
};
#[derive(Clone, Copy, PartialEq, Eq)]
enum Menu {
    Key,
    Scale,
    YTarget,
    StrumRate,
    MappingKind,
    MappingChannel(bool),
    MappingCc(bool),
}
impl Menu {
    fn items(self) -> Vec<String> {
        match self {
            Self::StrumRate => ARP_RATES
                .iter()
                .map(|(label, _)| label.to_string())
                .collect(),
            Self::YTarget => [
                "Velocity",
                "Gate",
                "Pressure",
                "Timbre",
                "Bend",
                "Custom CC",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            Self::Key => harmony::NOTE_NAMES.iter().map(|s| s.to_string()).collect(),
            Self::Scale => harmony::SCALE_NAMES.iter().map(|s| s.to_string()).collect(),
            Self::MappingKind => ["OFF", "CC 7-BIT", "CC 14-BIT", "PITCH BEND"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            Self::MappingChannel(bend) => (1..=16)
                .map(|n| n.to_string())
                .chain((!bend).then(|| "ANY".into()))
                .collect(),
            Self::MappingCc(wide) => (0..if wide { 32 } else { 128 })
                .map(|n| n.to_string())
                .collect(),
        }
    }
    fn option_rect(self, index: usize) -> Rect {
        let (x, y, w, h) = match self {
            Self::StrumRate => (852.0, 238.0, 110.0, 30.0),
            Self::YTarget => (612.0, 310.0, 224.0, 30.0),
            Self::Key => (32.0, 544.0, 64.0, 30.0),
            Self::Scale => (198.0, 544.0, 114.0, 30.0),
            Self::MappingKind => (616.0, 380.0, 114.0, 30.0),
            Self::MappingChannel(_) => (824.0, 380.0, 38.0, 30.0),
            Self::MappingCc(_) => (600.0, 380.0, 30.0, 30.0),
        };
        let columns = self.columns();
        (
            x + (index % columns) as f32 * w,
            y + (index / columns) as f32 * h,
            w,
            h,
        )
    }
    fn columns(self) -> usize {
        match self {
            Self::Scale | Self::MappingKind | Self::StrumRate => 2,
            Self::MappingChannel(_) => 6,
            Self::MappingCc(_) => 16,
            Self::Key => 3,
            Self::YTarget => 1,
        }
    }
    fn trigger_rect(self) -> Rect {
        match self {
            Self::StrumRate => (852.0, 342.0, 220.0, 42.0),
            Self::YTarget => (612.0, 494.0, 224.0, 30.0),
            Self::Key => (32.0, 510.0, 158.0, 28.0),
            Self::Scale => (198.0, 510.0, 228.0, 28.0),
            Self::MappingKind => (616.0, 346.0, 200.0, 30.0),
            Self::MappingChannel(_) => (824.0, 346.0, 110.0, 30.0),
            Self::MappingCc(_) => (942.0, 346.0, 130.0, 30.0),
        }
    }
}
const CODES: [Code; KEY_COUNT] = [
    Code::Digit1,
    Code::Digit2,
    Code::Digit3,
    Code::Digit4,
    Code::Digit5,
    Code::Digit6,
    Code::Digit7,
    Code::Digit8,
    Code::Digit9,
    Code::Digit0,
    Code::Minus,
    Code::Equal,
    Code::KeyQ,
    Code::KeyW,
    Code::KeyE,
    Code::KeyR,
    Code::KeyT,
    Code::KeyY,
    Code::KeyU,
    Code::KeyI,
    Code::KeyO,
    Code::KeyP,
    Code::BracketLeft,
    Code::BracketRight,
    Code::KeyA,
    Code::KeyS,
    Code::KeyD,
    Code::KeyF,
    Code::KeyG,
    Code::KeyH,
    Code::KeyJ,
    Code::KeyK,
    Code::KeyL,
    Code::Semicolon,
    Code::Quote,
    Code::Enter,
];

const MEMORY_CODES: [Code; 8] = [
    Code::KeyZ,
    Code::KeyX,
    Code::KeyC,
    Code::KeyV,
    Code::KeyB,
    Code::KeyN,
    Code::KeyM,
    Code::Comma,
];
const MEMORY_HINTS: [&str; 8] = ["Z", "X", "C", "V", "B", "N", "M", ","];

static PREFS: OnceLock<AppearanceStore> = OnceLock::new();
fn prefs() -> &'static AppearanceStore {
    PREFS.get_or_init(|| AppearanceStore::new("Chordboard"))
}
fn hit(r: Rect, x: f32, y: f32) -> bool {
    (r.0..=r.0 + r.2).contains(&x) && (r.1..=r.1 + r.3).contains(&y)
}
fn alpha(mut color: Color, a: f32) -> Color {
    color.a = a;
    color
}
#[derive(Clone, Copy)]
struct Tick;
#[derive(Clone, Copy)]
enum Drag {
    Pad,
    StrumBound(bool, bool),
    Control(ControlDrag),
    Key(u8),
    Memory(MemoryDrag),
}
#[derive(Clone, Copy)]
struct ControlDrag {
    press: pleasant_ui::pointer::ValuePress<&'static str>,
    ptr: ParamPtr,
    norm: f32,
    editable: bool,
    started: bool,
}
struct ChordboardView {
    params: Arc<ChordboardParams>,
    bridge: Arc<Bridge>,
    context: Arc<dyn GuiContext>,
    font: Cell<Option<FontId>>,
    snapshot: Snapshot,
    pending_learn: Option<(u8, i32, u32)>,
    pressed: [bool; KEY_COUNT],
    special: [bool; 2],
    memory_held: [bool; 8],
    focused: bool,
    pointer: Option<(f32, f32)>,
    drag: Option<Drag>,
    panel: Option<Panel>,
    edit: Option<ValueEdit<&'static str>>,
    menu: Option<Menu>,
    menu_cursor: usize,
    status: String,
    key_anim: [f32; KEY_COUNT],
    string_anim: [f32; 12],
    trail: Vec<(f32, f32, Instant)>,
    last_frame: Instant,
    meters: [f32; 3],
    page_position: f32,
    page_start: f32,
    page_target: i32,
    page_elapsed: f32,
    controls_signature: Vec<f32>,
    slot_signature: [u64; 8],
    mapping_axis: usize,
    mapping_signature: [u32; 2],
}
impl Drop for ChordboardView {
    fn drop(&mut self) {
        self.bridge.send(Command::ReleaseKeyboard);
        self.bridge.send(Command::EndGesture);
        self.bridge.visible.store(false, Ordering::Relaxed);
    }
}
impl ChordboardView {
    fn new(
        params: Arc<ChordboardParams>,
        bridge: Arc<Bridge>,
        context: Arc<dyn GuiContext>,
    ) -> Self {
        Self {
            params: params.clone(),
            bridge: bridge.clone(),
            context,
            font: Cell::new(None),
            snapshot: Snapshot::default(),
            pending_learn: None,
            pressed: [false; KEY_COUNT],
            special: [false; 2],
            memory_held: [false; 8],
            focused: false,
            pointer: None,
            drag: None,
            panel: None,
            edit: None,
            menu: None,
            menu_cursor: 0,
            status: String::new(),
            key_anim: [0.0; KEY_COUNT],
            string_anim: [0.0; 12],
            trail: Vec::with_capacity(24),
            last_frame: Instant::now(),
            meters: [0.0, 0.5, 0.5],
            page_position: params.mode.value().max(1) as f32,
            page_start: params.mode.value().max(1) as f32,
            page_target: params.mode.value().max(1),
            page_elapsed: pleasant_ui::page_slide::DURATION,
            controls_signature: Vec::new(),
            slot_signature: [0; 8],
            mapping_axis: 0,
            mapping_signature: [0; 2],
        }
    }
    fn control(&self, id: &str) -> Option<Control> {
        self.params.control(id)
    }
    fn arp_main(&self) -> bool {
        self.params.mode.value() == 3
    }
    fn set_panel(&mut self, panel: Option<Panel>) {
        self.release_keys();
        self.request_learn(0);
        self.panel = panel;
        self.menu = None;
    }
    fn learning(&self) -> u8 {
        self.pending_learn
            .map_or(self.snapshot.learning, |pending| pending.0)
    }
    fn request_learn(&mut self, target: u8) {
        let mapping = if (1..=2).contains(&target) {
            self.params
                .mapping(target as usize - 1)
                .load(Ordering::Relaxed)
        } else {
            0
        };
        self.pending_learn = Some((
            target,
            self.params.control_base.load(Ordering::Relaxed),
            mapping,
        ));
        self.bridge.send(Command::Learn(target));
    }
    fn key_active(&self, key: usize) -> bool {
        self.pressed[key]
            || matches!(self.drag, Some(Drag::Key(token)) if token == key as u8 + POINTER_KEY_OFFSET)
            || self.snapshot.accepted & (1 << key) != 0
    }
    fn base_controls(&self) -> Vec<(Control, Rect)> {
        self.controls_for_mode(self.params.mode.value())
    }
    fn controls_for_mode(&self, mode: i32) -> Vec<(Control, Rect)> {
        let mut positions = Vec::new();
        let mode = mode.max(1);
        positions.extend(voicing_controls());
        if mode == 3 {
            positions.extend(arp_controls());
        } else if mode == 1 {
            positions.extend(strum_controls());
        }
        positions.extend(output_controls(mode));
        if !self.params.tempo_sync.value() {
            positions.push(("tempo", TEMPO_CONTROL));
        }
        if self.params.strum_sync.value() {
            positions.retain(|(id, _)| *id != "strum_ms");
        }
        positions
            .into_iter()
            .filter_map(|(id, r)| self.control(id).map(|c| (c, r)))
            .collect()
    }
    fn panel_controls(&self) -> Vec<(Control, Rect)> {
        let Some(panel) = self.panel else {
            return Vec::new();
        };
        let ids: &[&str] = match panel {
            Panel::Mapping => match self.mapping_axis {
                0 => &["x_reverse"],
                1 if self.params.y_target.value() == 5 => &["y", "y_reverse", "y_cc"],
                1 => &["y", "y_reverse"],
                _ => &[],
            },
            Panel::Output if self.params.mpe_enabled() => {
                &["upper", "members", "bend_range", "master_range", "filter"]
            }
            Panel::Output if self.params.split_channels.value() => &[
                "output_channel",
                "split_channels",
                "bass_channel",
                "upper_channel",
                "filter",
            ],
            Panel::Output => &["output_channel", "split_channels", "filter"],
        };
        ids.iter()
            .enumerate()
            .filter_map(|(i, id)| {
                let r = if panel == Panel::Mapping {
                    mapping_control_rect(if *id == "y_cc" { 5 } else { i })
                } else {
                    local_control_rect(panel, i)
                };
                self.control(id).map(|c| (c, r))
            })
            .collect()
    }
    // Popovers own their pointer targets; covered controls cannot react underneath.
    fn placed_controls(&self) -> Vec<(Control, Rect)> {
        if self.panel.is_some() {
            self.panel_controls()
        } else {
            self.base_controls()
        }
    }
    fn mapping(&self) -> crate::engine::Mapping {
        crate::engine::Mapping::decode(
            self.params
                .mapping(self.mapping_axis)
                .load(Ordering::Relaxed),
        )
    }
    fn emit(cx: &mut EventContext, ptr: ParamPtr, norm: f32) {
        cx.emit(RawParamEvent::BeginSetParameter(ptr));
        cx.emit(RawParamEvent::SetParameterNormalized(
            ptr,
            norm.clamp(0.0, 1.0),
        ));
        cx.emit(RawParamEvent::EndSetParameter(ptr));
    }
    fn set(&self, cx: &mut EventContext, id: &str, norm: f32) {
        if let Some(c) = self.control(id) {
            Self::emit(cx, c.ptr, norm);
        }
    }
    fn open_menu(&mut self, menu: Menu) {
        self.release_keys();
        self.menu_cursor = match menu {
            Menu::YTarget => self.params.y_target.value() as usize,
            Menu::StrumRate => ARP_RATES
                .iter()
                .position(|(_, beats)| (*beats - self.params.strum_beats.value()).abs() < 0.0001)
                .unwrap_or(3),
            Menu::Key => self.params.key.value() as usize,
            Menu::Scale => self.params.scale.value() as usize,
            Menu::MappingKind => self.mapping().kind as usize,
            Menu::MappingChannel(_) => self.mapping().channel as usize,
            Menu::MappingCc(_) => self.mapping().number as usize,
        };
        self.menu = Some(menu);
    }
    fn select_menu(&mut self, cx: &mut EventContext, menu: Menu, index: usize) {
        self.menu = None;
        match menu {
            Menu::StrumRate => Self::emit(
                cx,
                self.params.strum_beats.as_ptr(),
                self.params
                    .strum_beats
                    .preview_normalized(ARP_RATES[index].1),
            ),
            Menu::YTarget => Self::emit(
                cx,
                self.params.y_target.as_ptr(),
                self.params.y_target.preview_normalized(index as i32),
            ),
            Menu::Key => Self::emit(
                cx,
                self.params.key.as_ptr(),
                self.params.key.preview_normalized(index as i32),
            ),
            Menu::Scale => Self::emit(
                cx,
                self.params.scale.as_ptr(),
                self.params.scale.preview_normalized(index as i32),
            ),
            _ => {
                let mut m = self.mapping();
                match menu {
                    Menu::MappingKind => {
                        m.kind = index as u8;
                        if m.kind == 2 {
                            m.number %= 32;
                        }
                        if m.kind == 3 && m.channel == 16 {
                            m.channel = 0;
                        }
                    }
                    Menu::MappingChannel(_) => m.channel = index as u8,
                    Menu::MappingCc(_) => m.number = index as u8,
                    _ => {}
                }
                // Choosing an X/Y CC manually clears the learned channel lock.
                // MIDI Learn writes its captured channel directly to the mapping.
                if self.mapping_axis < 2
                    && matches!(m.kind, 1 | 2)
                    && matches!(menu, Menu::MappingKind | Menu::MappingCc(_))
                {
                    m.channel = 16;
                }
                self.params
                    .mapping(self.mapping_axis)
                    .store(m.encode(), Ordering::Relaxed);
            }
        }
    }
    fn inversion(&self, cx: &mut EventContext, step: i8) {
        let n = self.snapshot.full_notes.len.max(1) as i16;
        let value = (self.snapshot.inversion as i16 + step as i16).rem_euclid(n) as i32;
        Self::emit(
            cx,
            self.params.inversion.as_ptr(),
            self.params.inversion.preview_normalized(value),
        );
    }
    fn release_keys(&mut self) {
        self.pressed.fill(false);
        self.special.fill(false);
        self.bridge.send(Command::ReleaseKeyboard);
    }
    fn keyboard_root(&self, column: usize) -> Option<u8> {
        harmony::keyboard_root(self.params.fifths.value(), column)
            .map(|root| (root + self.params.key.value() as u8) % 12)
    }
    fn play_key(&mut self, key: usize) {
        if self.pressed[key] {
            return;
        }
        if let Some(root) = self.keyboard_root(key % KEY_COLUMNS) {
            self.pressed[key] = true;
            self.bridge.send(Command::KeyDown(
                key as u8,
                (self.params.keyboard_octave.value() + root as i32).min(127) as u8,
                harmony::row_quality(key / KEY_COLUMNS),
            ));
        }
    }
    fn bound_param(&self, y_axis: bool, maximum: bool) -> &FloatParam {
        match (y_axis, maximum) {
            (false, false) => &self.params.x_min,
            (false, true) => &self.params.x_max,
            (true, false) => &self.params.y_min,
            (true, true) => &self.params.y_max,
        }
    }
    fn end_drag(&mut self, cx: &mut EventContext) {
        if let Some(drag) = self.drag.take() {
            match drag {
                Drag::Pad => {
                    for ptr in [self.params.x.as_ptr(), self.params.y.as_ptr()] {
                        cx.emit(RawParamEvent::EndSetParameter(ptr));
                    }
                    self.bridge.send(Command::EndGesture);
                }
                Drag::StrumBound(y_axis, right) => {
                    cx.emit(RawParamEvent::EndSetParameter(
                        self.bound_param(y_axis, right).as_ptr(),
                    ));
                }
                Drag::Memory(_) => {}
                Drag::Control(drag) => {
                    if drag.started {
                        cx.emit(RawParamEvent::EndSetParameter(drag.ptr));
                    }
                }
                Drag::Key(key) => {
                    if (key as usize) < KEY_COUNT {
                        self.pressed[key as usize] = false;
                    }
                    self.bridge.send(Command::KeyUp(key));
                }
            }
            cx.release();
        }
    }
    fn commit_edit(&mut self, cx: &mut EventContext) {
        let Some(mut edit) = self.edit.take() else {
            return;
        };
        if let Some(control) = self.control(edit.target) {
            // ParamPtr is kept alive by this view's Arc<ChordboardParams>.
            let value = unsafe { control.ptr.string_to_normalized_value(&edit.text) };
            if let Some(value) = value {
                Self::emit(cx, control.ptr, value);
            } else {
                edit.invalid = true;
                self.edit = Some(edit);
            }
        }
    }
    fn tick(&mut self, cx: &mut EventContext) {
        if !self.params.editor_state.is_open() && self.context.plugin_api() != PluginApi::Standalone
        {
            return;
        }
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        let was_sliding = self.page_elapsed < pleasant_ui::page_slide::DURATION;
        let mode = self.params.mode.value().max(1);
        if mode != self.page_target {
            self.page_start = self.page_position;
            self.page_target = mode;
            self.page_elapsed = 0.0;
            self.end_drag(cx);
            self.edit = None;
        }
        self.page_elapsed = (self.page_elapsed + dt).min(pleasant_ui::page_slide::DURATION);
        self.page_position = pleasant_ui::page_slide::position(
            self.page_start,
            self.page_target as f32,
            self.page_elapsed,
        );
        if was_sliding || self.page_elapsed < pleasant_ui::page_slide::DURATION {
            cx.needs_redraw();
        }
        let old = self.snapshot;
        while let Some(s) = self.bridge.snapshots.pop() {
            self.snapshot = s;
        }
        if let Some((target, base, mapping)) = self.pending_learn {
            let completed = if target == 4 {
                self.params.control_base.load(Ordering::Relaxed) != base
            } else if (1..=2).contains(&target) {
                self.params
                    .mapping(target as usize - 1)
                    .load(Ordering::Relaxed)
                    != mapping
            } else {
                false
            };
            if self.snapshot.learning == target || completed {
                self.pending_learn = None;
            }
        }
        let mut changed = old != self.snapshot;
        for i in 0..KEY_COUNT {
            let target = if self.key_active(i) { 1.0 } else { 0.0 };
            let previous = self.key_anim[i];
            self.key_anim[i] = previous + (target - previous) * (1.0 - (-dt / 0.07).exp());
            changed |= (self.key_anim[i] - previous).abs() > 0.001;
        }
        for i in 0..12 {
            if old.strikes[i] != self.snapshot.strikes[i] {
                self.string_anim[i] = 1.0;
            }
            if self.string_anim[i] > 0.001 {
                changed = true;
                self.string_anim[i] = (self.string_anim[i] - dt * 3.5).max(0.0);
            }
        }
        if (old.x, old.y) != (self.snapshot.x, self.snapshot.y) {
            if self.trail.len() >= 24 {
                self.trail.remove(0);
            }
            self.trail.push((self.snapshot.x, self.snapshot.y, now));
        }
        self.trail
            .retain(|p| now.duration_since(p.2).as_secs_f32() < 0.35);
        changed |= !self.trail.is_empty();
        let targets = [
            self.snapshot.pressure,
            self.snapshot.timbre,
            self.snapshot.bend,
        ];
        for (i, &target) in targets.iter().enumerate() {
            let prev = self.meters[i];
            self.meters[i] = prev + (target - prev) * (1.0 - (-dt / 0.055).exp());
            changed |= (prev - self.meters[i]).abs() > 0.001;
        }
        let signature: Vec<f32> = self.params.controls().iter().map(|c| c.norm).collect();
        if signature != self.controls_signature {
            self.controls_signature = signature;
            changed = true;
        }
        for i in 0..2 {
            let value = self.params.mapping(i).load(Ordering::Relaxed);
            if value != self.mapping_signature[i] {
                self.mapping_signature[i] = value;
                changed = true;
            }
        }
        for i in 0..8 {
            let v = self.params.slot(i).load(Ordering::Relaxed);
            if v != self.slot_signature[i] {
                self.slot_signature[i] = v;
                changed = true;
            }
        }
        if changed {
            cx.needs_redraw();
        }
    }
}
impl View for ChordboardView {
    fn element(&self) -> Option<&'static str> {
        Some("chordboard")
    }
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let was_editing = self.edit.is_some();
        event.map(|_: &Tick, _| self.tick(cx));
        event.map(|param: &RawParamEvent, _| {
            if let RawParamEvent::SetParameterNormalized(ptr, value) = param {
                if *ptr == self.params.quality.as_ptr() {
                    self.bridge.send(Command::SetQuality(
                        self.params.quality.preview_plain(*value) as u8,
                    ));
                }
                if *ptr == self.params.inversion.as_ptr() {
                    self.bridge.send(Command::SetInversion(
                        self.params.inversion.preview_plain(*value) as u8,
                    ));
                }
            }
        });
        event.map(|window: &WindowEvent, meta| {
            if self.window_event(cx, window) {
                nih_plug_vizia::consume_window_event(cx, window, meta);
                cx.needs_redraw();
            }
        });
        pleasant_ui::value_edit::sync_text_input(cx, was_editing, self.edit.is_some());
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        self.render(cx, canvas);
    }
}
pub fn create(params: Arc<ChordboardParams>, bridge: Arc<Bridge>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, context| {
            nih_plug_vizia::assets::register_noto_sans_light(cx);
            bridge.visible.store(true, Ordering::Relaxed);
            ChordboardView::new(params.clone(), bridge.clone(), context)
                .build(cx, |cx| {
                    let timer = cx.add_timer(Duration::from_millis(16), None, |cx, action| {
                        if let TimerAction::Tick(_) = action {
                            cx.emit(Tick);
                        }
                    });
                    cx.start_timer(timer);
                })
                .width(Stretch(1.0))
                .height(Stretch(1.0))
                .focusable(true);
        },
    )
}

#[cfg(test)]
struct PreviewContext;
#[cfg(test)]
impl GuiContext for PreviewContext {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Standalone
    }
    fn request_resize(&self) -> bool {
        true
    }
    unsafe fn raw_begin_set_parameter(&self, _: ParamPtr) {}
    unsafe fn raw_set_parameter_normalized(&self, _: ParamPtr, _: f32) {}
    unsafe fn raw_end_set_parameter(&self, _: ParamPtr) {}
    fn get_state(&self) -> PluginState {
        PluginState {
            version: "0.1.0".into(),
            params: Default::default(),
            fields: Default::default(),
        }
    }
    fn set_state(&self, _: PluginState) {}
}

/// Interactive preview using the real plugin wrapper and processing loop.
#[cfg(feature = "ui-preview")]
pub fn preview() {
    let mut args: Vec<String> = std::env::args().collect();
    // Exercise the plugin without opening audio/MIDI devices by default.
    if !args
        .iter()
        .any(|arg| arg == "--backend" || arg.starts_with("--backend="))
    {
        args.extend(["--backend".into(), "dummy".into()]);
    }
    nih_export_standalone_with_args::<crate::Chordboard, _>(args);
}

#[cfg(test)]
mod tests;
