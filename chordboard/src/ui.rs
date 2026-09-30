mod chords;
mod events;
mod layout;
mod mapping;
mod menu;
mod routing;
mod style;
use menu::Menu;
mod memories;
use memories::{MemoryDrag, MemoryUi};
use routing::RouteDrag;
mod performance;
mod piano;
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
    Route(RouteDrag),
    RouteNode(u8, ParamPtr),
    Split,
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
    ui_font: Cell<Option<FontId>>,
    bold_font: Cell<Option<FontId>>,
    motion: Vec<style::ControlMotion>,
    action_flash: Option<(Rect, f32)>,
    memory_ui: MemoryUi,
    snapshot: Snapshot,
    leading_progress: f32,
    pending_learn: Option<(u8, i32, u32)>,
    pressed: [bool; KEY_COUNT],
    special: [bool; 2],
    memory_held: [bool; 8],
    focused: bool,
    pointer: Option<(f32, f32)>,
    drag: Option<Drag>,
    pad_hover: bool,
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
    expand_progress: f32,
    expand_start: f32,
    expand_target: f32,
    expand_elapsed: f32,
    controls_signature: Vec<f32>,
    slot_signature: [u64; 8],
    route_slot: usize,
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
            ui_font: Cell::new(None),
            bold_font: Cell::new(None),
            motion: Vec::new(),
            action_flash: None,
            memory_ui: MemoryUi::default(),
            snapshot: Snapshot::default(),
            leading_progress: 1.0,
            pending_learn: None,
            pressed: [false; KEY_COUNT],
            special: [false; 2],
            memory_held: [false; 8],
            focused: false,
            pointer: None,
            drag: None,
            pad_hover: false,
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
            expand_progress: 0.0,
            expand_start: 0.0,
            expand_target: 0.0,
            expand_elapsed: pleasant_ui::page_slide::DURATION,
            controls_signature: Vec::new(),
            slot_signature: [0; 8],
            route_slot: 0,
            mapping_axis: 0,
            mapping_signature: [0; 2],
        }
    }
    fn control(&self, id: &str) -> Option<Control> {
        self.params.routes[self.route_slot]
            .control(id)
            .or_else(|| self.params.control(id))
    }
    fn arp_main(&self) -> bool {
        self.mode() == 3
    }
    fn can_expand_strum(&self) -> bool {
        self.mode() == 2
    }
    fn expand_t(&self) -> f32 {
        self.expand_progress
    }
    fn pad(&self) -> Rect {
        pad_rect(self.expand_t())
    }
    fn play_pad(&self) -> Rect {
        play_pad_rect(self.expand_t())
    }
    fn field(&self, manual: bool) -> Rect {
        field_rect(self.expand_t(), manual)
    }
    fn expand_button(&self) -> Rect {
        expand_rect(self.expand_t())
    }
    fn toggle_expand(&mut self) {
        if !self.can_expand_strum() && self.expand_target == 0.0 {
            return;
        }
        self.expand_start = self.expand_progress;
        self.expand_target = if self.expand_target > 0.5 { 0.0 } else { 1.0 };
        self.expand_elapsed = 0.0;
        self.menu = None;
        self.panel = None;
        self.edit = None;
    }
    fn set_panel(&mut self, panel: Option<Panel>) {
        self.release_keys();
        self.memory_held.fill(false);
        self.request_learn(0);
        self.memory_ui.armed = false;
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
        self.controls_for_mode(self.mode())
    }
    fn controls_for_mode(&self, mode: i32) -> Vec<(Control, Rect)> {
        let mut positions = Vec::new();
        let mode = mode.max(1);
        positions.extend(voicing_controls());
        positions.push(("root_on_select", ROOT_ON_SELECT));
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
    fn panel_rect(&self, panel: Panel) -> Rect {
        if panel == Panel::Mapping {
            mapping_panel_rect(self.mapping_axis)
        } else {
            panel.rect()
        }
    }
    fn panel_close_rect(&self, panel: Panel) -> Rect {
        let r = self.panel_rect(panel);
        (r.0 + r.2 - 40.0, r.1 + 8.0, 28.0, 28.0)
    }
    fn menu_trigger_rect(&self, menu: Menu) -> Rect {
        let r = mapping_panel_rect(self.mapping_axis);
        match menu {
            Menu::MappingKind => (r.0 + 12.0, r.1 + 48.0, 132.0, 28.0),
            Menu::MappingCc(_) | Menu::MappingChannel(true) => {
                (r.0 + 152.0, r.1 + 48.0, 96.0, 28.0)
            }
            Menu::MappingChannel(false) => (r.0 + 256.0, r.1 + 48.0, 116.0, 28.0),
            Menu::YTarget => (r.0 + 12.0, r.1 + 84.0, 172.0, 28.0),
            Menu::StrumRate => strum_rate_rect(self.expand_t()),
            _ => menu.trigger_rect(),
        }
    }
    fn menu_bounds(&self, menu: Menu) -> Rect {
        menu.bounds_at(self.menu_trigger_rect(menu))
    }
    fn menu_close_rect(&self, menu: Menu) -> Rect {
        let r = self.menu_bounds(menu);
        (r.0 + r.2 - 34.0, r.1 + 6.0, 26.0, 26.0)
    }
    fn menu_option_rect(&self, menu: Menu, index: usize) -> Rect {
        menu.option_rect_at(self.menu_bounds(menu), index)
    }
    fn panel_controls(&self) -> Vec<(Control, Rect)> {
        let Some(panel) = self.panel else {
            return Vec::new();
        };
        if panel == Panel::Routes {
            return ["route_enabled"]
                .into_iter()
                .filter_map(|id| self.control(id).map(|c| (c, route_control_rect(id))))
                .collect();
        }
        let ids: &[&str] = match panel {
            Panel::Routes => unreachable!(),
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
                    mapping_control_rect(self.mapping_axis, id)
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
    fn menu_selection(&self, menu: Menu) -> usize {
        match menu {
            Menu::RouteSource => self.params.routes[self.route_slot].source.value() as usize,
            Menu::RouteTarget => crate::engine::routing::available_targets()
                .position(|(i, _)| i == self.params.routes[self.route_slot].target.value() as usize)
                .unwrap_or(0),
            Menu::Quality => self.params.quality.value() as usize,
            Menu::Protocol => self.params.output_mode.value() as usize,
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
        }
    }
    fn open_menu(&mut self, menu: Menu) {
        self.release_keys();
        self.memory_held.fill(false);
        self.memory_ui.armed = false;
        self.menu_cursor = self.menu_selection(menu);
        self.menu = Some(menu);
    }
    fn select_menu(&mut self, cx: &mut EventContext, menu: Menu, index: usize) {
        self.menu = None;
        match menu {
            Menu::RouteSource => {
                let route = &self.params.routes[self.route_slot];
                Self::emit(
                    cx,
                    route.source.as_ptr(),
                    route.source.preview_normalized(index as i32),
                );
            }
            Menu::RouteTarget => {
                let Some((index, _)) = crate::engine::routing::available_targets().nth(index) else { return; };
                let route = &self.params.routes[self.route_slot];
                Self::emit(
                    cx,
                    route.target.as_ptr(),
                    route.target.preview_normalized(index as i32),
                );
            }
            Menu::Quality => Self::emit(
                cx,
                self.params.quality.as_ptr(),
                self.params.quality.preview_normalized(index as i32),
            ),
            Menu::Protocol => Self::emit(
                cx,
                self.params.output_mode.as_ptr(),
                self.params.output_mode.preview_normalized(index as i32),
            ),
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
    fn pad_norm(&self, x: f32, y: f32) -> (f32, f32) {
        let play = self.play_pad();
        (
            ((x - play.0) / play.2).clamp(0.0, 1.0),
            (1.0 - (y - play.1) / play.3).clamp(0.0, 1.0),
        )
    }
    fn strum_bound_at(&self, x: f32, y: f32) -> Option<(bool, bool)> {
        if self.mode() != 2 {
            return None;
        }
        let play = self.play_pad();
        for y_axis in [false, true] {
            let (min, max) = if y_axis {
                expression_bounds(self.params.y_min.value(), self.params.y_max.value())
            } else {
                strum_bounds(self.params.x_min.value(), self.params.x_max.value())
            };
            for (right, value) in [(false, min), (true, max)] {
                let (ax, ay, pointer) = strum_bound_anchor_in(play, y_axis, value);
                if pleasant_ui::tag_contains(ax, ay, pointer, x, y) {
                    return Some((y_axis, right));
                }
            }
        }
        None
    }
    fn can_pad_hover(&self) -> bool {
        self.mode() == 2
            && self.params.strum_latch.value()
            && self.panel.is_none()
            && self.menu.is_none()
            && self.edit.is_none()
            && self.drag.is_none()
            && self.page_elapsed >= pleasant_ui::page_slide::DURATION
    }
    fn set_pad_xy(&self, cx: &mut EventContext, x: f32, y: f32) {
        let (px, py) = self.pad_norm(x, y);
        cx.emit(RawParamEvent::SetParameterNormalized(
            self.params.x.as_ptr(),
            px,
        ));
        cx.emit(RawParamEvent::SetParameterNormalized(
            self.params.y.as_ptr(),
            py,
        ));
    }
    fn begin_pad(&mut self, cx: &mut EventContext, x: f32, y: f32, capture: bool) {
        let starting = !self.pad_hover && !matches!(self.drag, Some(Drag::Pad));
        let (px, py) = self.pad_norm(x, y);
        if starting {
            for ptr in [self.params.x.as_ptr(), self.params.y.as_ptr()] {
                cx.emit(RawParamEvent::BeginSetParameter(ptr));
            }
            self.bridge.send(Command::BeginGesture(px, py));
        }
        self.set_pad_xy(cx, x, y);
        if capture {
            self.pad_hover = false;
            self.drag = Some(Drag::Pad);
            cx.capture();
        } else {
            self.pad_hover = true;
        }
    }
    fn end_pad_hover(&mut self, cx: &mut EventContext) {
        if !self.pad_hover {
            return;
        }
        self.pad_hover = false;
        for ptr in [self.params.x.as_ptr(), self.params.y.as_ptr()] {
            cx.emit(RawParamEvent::EndSetParameter(ptr));
        }
        self.bridge.send(Command::EndGesture);
    }
    fn sync_pad_hover(&mut self, cx: &mut EventContext) {
        let over = self.can_pad_hover()
            && self.pointer.is_some_and(|(x, y)| {
                hit(self.play_pad(), x, y) && self.strum_bound_at(x, y).is_none()
            });
        if over {
            if !self.pad_hover {
                if let Some((x, y)) = self.pointer {
                    self.begin_pad(cx, x, y, false);
                }
            }
        } else {
            self.end_pad_hover(cx);
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
                Drag::RouteNode(_, ptr) => cx.emit(RawParamEvent::EndSetParameter(ptr)),
                Drag::Split => cx.emit(RawParamEvent::EndSetParameter(self.params.split_note.as_ptr())),
                Drag::Memory(_) | Drag::Route(_) => {}
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
            let value = self.parse_display_value(&control, &edit.text);
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
        let mode = self.mode().max(1);
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
        let was_expanding = self.expand_elapsed < pleasant_ui::page_slide::DURATION;
        if !self.can_expand_strum() && self.expand_target != 0.0 {
            self.expand_start = self.expand_progress;
            self.expand_target = 0.0;
            self.expand_elapsed = 0.0;
        }
        self.expand_elapsed = (self.expand_elapsed + dt).min(pleasant_ui::page_slide::DURATION);
        self.expand_progress = pleasant_ui::page_slide::position(
            self.expand_start,
            self.expand_target,
            self.expand_elapsed,
        );
        if was_expanding || self.expand_elapsed < pleasant_ui::page_slide::DURATION {
            cx.needs_redraw();
        }
        self.sync_pad_hover(cx);
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
        if old.leading_serial != self.snapshot.leading_serial {
            self.leading_progress = 0.0;
        }
        if self.leading_progress < 1.0 {
            self.leading_progress = (self.leading_progress + dt / 0.65).min(1.0);
            changed = true;
        }
        changed |= self.tick_motion(dt);
        changed |= self.tick_memories(dt);
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
