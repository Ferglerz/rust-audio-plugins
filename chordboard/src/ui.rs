mod events;
mod render;
use crate::{
    bridge::Bridge,
    engine::{Command, Snapshot},
    harmony::{self, SavedChord},
    params::{ChordboardParams, Control},
};
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
const W: f32 = 1120.0;
const H: f32 = 780.0;
type Rect = (f32, f32, f32, f32);
const PAD: Rect = (600.0, 190.0, 488.0, 310.0);
const GROUPS: [&str; 6] = [
    "VOICING",
    "RHYTHM",
    "MPE",
    "EXPRESSION",
    "INPUT",
    "DISCOVERY",
];
#[derive(Clone, Copy, PartialEq, Eq)]
enum Menu {
    Key,
    Scale,
}
impl Menu {
    fn items(self) -> &'static [&'static str] {
        match self {
            Self::Key => &harmony::NOTE_NAMES,
            Self::Scale => &harmony::SCALE_NAMES,
        }
    }
    fn option_rect(self, index: usize) -> Rect {
        let (x, y, w, h) = match self {
            Self::Key => (32.0, 472.0, 64.0, 30.0),
            Self::Scale => (198.0, 472.0, 114.0, 30.0),
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
            Self::Scale => 2,
            Self::Key => 3,
        }
    }
    fn trigger_rect(self) -> Rect {
        match self {
            Self::Key => (32.0, 441.0, 158.0, 28.0),
            Self::Scale => (198.0, 441.0, 228.0, 28.0),
        }
    }
}
const CODES: [Code; 21] = [
    Code::KeyQ,
    Code::KeyW,
    Code::KeyE,
    Code::KeyR,
    Code::KeyT,
    Code::KeyY,
    Code::KeyU,
    Code::KeyA,
    Code::KeyS,
    Code::KeyD,
    Code::KeyF,
    Code::KeyG,
    Code::KeyH,
    Code::KeyJ,
    Code::KeyZ,
    Code::KeyX,
    Code::KeyC,
    Code::KeyV,
    Code::KeyB,
    Code::KeyN,
    Code::KeyM,
];
static PREFS: OnceLock<AppearanceStore> = OnceLock::new();
fn prefs() -> &'static AppearanceStore {
    PREFS.get_or_init(|| AppearanceStore::new("Chordboard"))
}
fn hit(r: Rect, x: f32, y: f32) -> bool {
    (r.0..=r.0 + r.2).contains(&x) && (r.1..=r.1 + r.3).contains(&y)
}
fn key_rect(index: usize) -> Rect {
    let row = index / 7;
    let col = index % 7;
    (
        32.0 + ([0.0, 0.25, 0.75][row] + col as f32) * 68.0,
        230.0 + row as f32 * 67.0,
        61.0,
        59.0,
    )
}
fn control_rect(index: usize) -> Rect {
    (
        32.0 + (index % 4) as f32 * 268.0,
        624.0 + (index / 4) as f32 * 64.0,
        252.0,
        54.0,
    )
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
    Control(ParamPtr, Rect),
    Key(u8),
}
struct ChordboardView {
    params: Arc<ChordboardParams>,
    bridge: Arc<Bridge>,
    context: Arc<dyn GuiContext>,
    font: Cell<Option<FontId>>,
    snapshot: Snapshot,
    pressed: [bool; 21],
    special: [bool; 4],
    focused: bool,
    drag: Option<Drag>,
    group: usize,
    page: usize,
    edit: Option<ValueEdit<&'static str>>,
    menu: Option<Menu>,
    menu_cursor: usize,
    status: String,
    key_anim: [f32; 21],
    string_anim: [f32; 12],
    trail: Vec<(f32, f32, Instant)>,
    last_frame: Instant,
    meters: [f32; 3],
    controls_signature: Vec<f32>,
    slot_signature: [u64; 8],
    mapping_axis: usize,
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
            pressed: [false; 21],
            special: [false; 4],
            focused: false,
            drag: None,
            group: 0,
            page: 0,
            edit: None,
            menu: None,
            menu_cursor: 0,
            status: String::new(),
            key_anim: [0.0; 21],
            string_anim: [0.0; 12],
            trail: Vec::with_capacity(24),
            last_frame: Instant::now(),
            meters: [0.0, 0.5, 0.5],
            controls_signature: Vec::new(),
            slot_signature: [0; 8],
            mapping_axis: 0,
        }
    }
    fn control(&self, id: &str) -> Option<Control> {
        self.params.controls().into_iter().find(|c| c.id == id)
    }
    fn group_controls(&self) -> Vec<Control> {
        self.params
            .controls()
            .into_iter()
            .filter(|c| c.group == self.group)
            .filter(|c| {
                !matches!(
                    c.id,
                    "latch" | "mpe" | "keyboard" | "fifths" | "key" | "scale" | "highlight"
                )
            })
            .collect()
    }
    fn shown_controls(&self) -> Vec<Control> {
        self.group_controls()
            .into_iter()
            .skip(self.page * 8)
            .take(8)
            .collect()
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
            Menu::Key => self.params.key.value() as usize,
            Menu::Scale => self.params.scale.value() as usize,
        };
        self.menu = Some(menu);
    }
    fn select_menu(&mut self, cx: &mut EventContext, menu: Menu, index: usize) {
        self.menu = None;
        match menu {
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
    fn play_key(&mut self, key: usize) {
        if self.pressed[key] {
            return;
        }
        if let Some(root) = harmony::keyboard_root(
            self.params.fifths.value(),
            self.params.bank.value() as u8,
            key % 7,
        ) {
            self.pressed[key] = true;
            self.bridge.send(Command::KeyDown(
                key as u8,
                (self.params.keyboard_octave.value() + root as i32).min(127) as u8,
                harmony::row_quality(key / 7),
            ));
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
                Drag::Control(ptr, _) => cx.emit(RawParamEvent::EndSetParameter(ptr)),
                Drag::Key(key) => {
                    if key < 21 {
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
        let old = self.snapshot;
        while let Some(s) = self.bridge.snapshots.pop() {
            self.snapshot = s;
        }
        let reduced = self.params.reduced_motion.value();
        let mut changed = old != self.snapshot;
        for i in 0..21 {
            let target = if self.pressed[i] || self.snapshot.accepted & (1 << i) != 0 {
                1.0
            } else {
                0.0
            };
            let previous = self.key_anim[i];
            self.key_anim[i] = if reduced {
                target
            } else {
                previous + (target - previous) * (1.0 - (-dt / 0.07).exp())
            };
            changed |= (self.key_anim[i] - previous).abs() > 0.001;
        }
        for i in 0..12 {
            if old.strikes[i] != self.snapshot.strikes[i] {
                self.string_anim[i] = 1.0;
            }
            if self.string_anim[i] > 0.001 {
                changed = true;
                self.string_anim[i] = if reduced {
                    0.0
                } else {
                    (self.string_anim[i] - dt * 3.5).max(0.0)
                };
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
            self.meters[i] = if reduced {
                target
            } else {
                prev + (target - prev) * (1.0 - (-dt / 0.055).exp())
            };
            changed |= (prev - self.meters[i]).abs() > 0.001;
        }
        let signature: Vec<f32> = self.params.controls().iter().map(|c| c.norm).collect();
        if signature != self.controls_signature {
            self.controls_signature = signature;
            changed = true;
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
                meta.consume();
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

/// Read-only rendering harness for visual QA without opening a DAW.
#[cfg(feature = "ui-preview")]
pub fn preview() {
    struct PreviewContext;
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
    nih_plug_vizia::vizia::Application::new(|cx| {
        nih_plug_vizia::assets::register_noto_sans_light(cx);
        let params = Arc::new(ChordboardParams::default());
        let bridge = Arc::new(Bridge::default());
        bridge.visible.store(true, Ordering::Relaxed);
        let mut engine = crate::engine::Engine::default();
        engine.note_on(2048, 60, 16, 0.8, Some(0), &mut |_| {});
        engine.note_on(2054, 66, 16, 0.8, Some(0), &mut |_| {});
        let mut view = ChordboardView::new(params, bridge, Arc::new(PreviewContext));
        view.snapshot = engine.snapshot();
        view.key_anim[0] = 1.0;
        view.key_anim[6] = 1.0;
        view.snapshot.x = 0.6;
        view.snapshot.y = 0.65;
        view.string_anim = [0.1, 0.2, 0.3, 0.6, 0.8, 0.4, 0.2, 0.1, 0.0, 0.0, 0.0, 0.0];
        view.status =
            "Native visual preview — MIDI and parameter editing require a plugin host".into();
        view.build(cx, |_| {})
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .focusable(true);
    })
    .title("Chordboard Preview")
    .inner_size((1120, 780))
    .ignore_default_theme()
    .run();
}
