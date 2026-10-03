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

const MEMORY_COUNT: usize = crate::engine::MEMORY_COUNT;
const MEMORY_CODES: [Code; MEMORY_COUNT] = [
    Code::KeyZ,
    Code::KeyX,
    Code::KeyC,
    Code::KeyV,
    Code::KeyB,
    Code::KeyN,
    Code::KeyM,
    Code::Comma,
    Code::Period,
    Code::Slash,
];
const MEMORY_HINTS: [&str; MEMORY_COUNT] = ["Z", "X", "C", "V", "B", "N", "M", ",", ".", "/"];

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
    RouteContour {
        origin: (f32, f32),
        min: f32,
        max: f32,
        slot: usize,
        axis: pleasant_ui::pointer::AxisLock,
    },
    Control(ControlDrag),
    Key(u8),
    Memory(MemoryDrag),
    Route(RouteDrag),
    RouteNode(u8, ParamPtr),
    Split,
    BassSplit,
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
    memory_held: [bool; MEMORY_COUNT],
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
    note_anim: [f32; 128],
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
    slot_signature: [u64; MEMORY_COUNT],
    route_slot: usize,
    selected_modulator: Option<usize>,
    modulator_pulse: f32,
    pending_route_remove: Option<usize>,
    route_progress: f32,
    route_start: f32,
    route_elapsed: f32,
    route_hover: Option<usize>,
    mapping_axis: usize,
    mapping_signature: [u32; 2],
    pub(super) show_bass_split: bool,
    pub(super) voicing_pulse: f32,
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
            memory_held: [false; MEMORY_COUNT],
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
            note_anim: [0.0; 128],
            trail: Vec::with_capacity(24),
            last_frame: Instant::now(),
            meters: [0.0, 0.5, 0.5],
            page_position: performance_page(params.mode.value()) as f32,
            page_start: params.mode.value().max(1) as f32,
            page_target: performance_page(params.mode.value()),
            page_elapsed: pleasant_ui::page_slide::DURATION,
            expand_progress: 0.0,
            expand_start: 0.0,
            expand_target: 0.0,
            expand_elapsed: pleasant_ui::page_slide::DURATION,
            controls_signature: Vec::new(),
            slot_signature: [0; MEMORY_COUNT],
            route_slot: 0,
            selected_modulator: None,
            modulator_pulse: 0.0,
            pending_route_remove: None,
            route_progress: 0.0,
            route_start: 0.0,
            route_elapsed: pleasant_ui::page_slide::DURATION,
            route_hover: None,
            mapping_axis: 0,
            mapping_signature: [0; 2],
            show_bass_split: true,
            voicing_pulse: 0.0,
        }
    }
    fn control(&self, id: &str) -> Option<Control> {
        self.params.routes[self.route_slot]
            .control(id)
            .or_else(|| self.params.control(id))
    }
    fn sweep_synced(&self) -> bool {
        self.routed_plain("strum_sync")
            .map_or(self.params.strum_sync.value(), |v| v >= 0.5)
    }
    fn arp_main(&self) -> bool {
        matches!(self.mode(), 1 | 3)
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
    pub(super) fn voicing_bounds(&self) -> Option<Rect> {
        let voicing = self.spread_preview_notes();
        let notes = voicing.as_slice();
        if notes.is_empty() {
            return None;
        }
        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        for &note in notes {
            let r = piano_key_rect(note);
            min_x = min_x.min(r.0);
            max_x = max_x.max(r.0 + r.2);
        }
        Some((
            min_x - 2.0,
            PIANO_KEYS.1 - 2.0,
            max_x - min_x + 4.0,
            PIANO_KEYS.3 + 22.0,
        ))
    }
    fn toggle_expand(&mut self) {
        if !self.can_expand_strum() && self.expand_target == 0.0 {
            return;
        }
        self.expand_start = self.expand_progress;
        self.expand_target = if self.expand_target > 0.5 { 0.0 } else { 1.0 };
        self.expand_elapsed = 0.0;
        self.menu = None;
        self.set_panel(None);
        self.edit = None;
    }
    fn set_panel(&mut self, panel: Option<Panel>) {
        self.release_keys();
        self.memory_held.fill(false);
        self.request_learn(0);
        self.memory_ui.armed = false;
        if (self.panel == Some(Panel::Routes)) != (panel == Some(Panel::Routes)) {
            self.route_start = self.route_progress;
            self.route_elapsed = 0.0;
        }
        self.panel = panel;
        if panel != Some(Panel::Routes) {
            self.selected_modulator = None;
        }
        self.edit = None;
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
        if mode == 2 {
            positions.push(("root_on_select", ROOT_ON_SELECT));
        }
        if matches!(mode, 1 | 3) {
            positions.extend(
                arp_controls()
                    .into_iter()
                    .filter(|(id, _)| mode == 3 || *id != "gate"),
            );
            if !self.sweep_synced() {
                positions.extend(strum_controls());
            }
        }
        positions.extend(output_controls(mode));
        positions.extend(self.keyboard_controls().into_iter().map(|(c, r)| (c.id, r)));
        if !self.params.tempo_sync.value() {
            positions.push(("tempo", TEMPO_CONTROL));
        }
        if self.sweep_synced() {
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
        (r.0 + r.2 - 48.0, r.1 + 6.0, 36.0, 32.0)
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
            Menu::KeyboardParam(id) => keyboard_control_rect(id, self.params.mpe_enabled()),
            _ => menu.trigger_rect(),
        }
    }
    fn menu_bounds(&self, menu: Menu) -> Rect {
        menu.bounds_at(self.menu_trigger_rect(menu))
    }
    fn menu_option_rect(&self, menu: Menu, index: usize) -> Rect {
        menu.option_rect_at(self.menu_bounds(menu), index)
    }
    fn keyboard_controls(&self) -> Vec<(Control, Rect)> {
        let ids: &[&str] = if self.params.mpe_enabled() {
            &["bass_channel", "bass_split", "upper", "members"]
        } else {
            &[
                "bass_channel",
                "bass_split",
                "output_channel",
                "upper_channel",
            ]
        };
        ids.iter()
            .filter_map(|id| {
                self.control(id)
                    .map(|c| (c, keyboard_control_rect(id, self.params.mpe_enabled())))
            })
            .collect()
    }
    fn panel_controls(&self) -> Vec<(Control, Rect)> {
        let Some(panel) = self.panel else {
            return Vec::new();
        };
        if panel == Panel::Routes {
            if !self.has_selected_route() {
                return Vec::new();
            }
            return ["route_enabled"]
                .into_iter()
                .filter_map(|id| self.control(id).map(|c| (c, route_control_rect(id))))
                .collect();
        }
        let ids: &[&str] = match self.mapping_axis {
            0 => &["x_reverse"],
            1 if self.params.y_target.value() == 5 => &["y", "y_reverse", "y_cc"],
            1 => &["y", "y_reverse"],
            _ => &[],
        };
        ids.iter()
            .filter_map(|id| {
                self.control(id)
                    .map(|c| (c, mapping_control_rect(self.mapping_axis, id)))
            })
            .collect()
    }
    // Popovers own their pointer targets; covered controls cannot react underneath.
    fn placed_controls(&self) -> Vec<(Control, Rect)> {
        let mut controls = self.base_controls();
        if let Some(panel) = self.panel {
            let p = self.panel_rect(panel);
            controls.retain(|(_, r)| !hit(p, r.0 + r.2 / 2.0, r.1 + r.3 / 2.0));
            controls.extend(self.panel_controls());
        }
        controls
    }
    fn mapping(&self) -> crate::engine::Mapping {
        crate::engine::Mapping::decode(
            self.params
                .mapping(self.mapping_axis)
                .load(Ordering::Relaxed),
        )
    }
    fn emit(cx: &mut EventContext, ptr: ParamPtr, norm: f32) {
        pleasant_ui::param::set_normalized_once(cx, ptr, norm);
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
            Menu::KeyboardParam(id) => self.control(id).map_or(0, |c| {
                let (min, _) = keyboard_menu_range(id).expect("keyboard dropdown");
                // The view owns this parameter pointer.
                (unsafe { c.ptr.unmodulated_plain_value() } as i32 - min) as usize
            }),
            Menu::YTarget => self.params.y_target.value() as usize,
            Menu::StrumRate => ARP_RATES
                .iter()
                .position(|(_, beats)| (*beats - self.params.strum_beats.value()).abs() < 0.0001)
                .unwrap_or(3),
            Menu::Key => harmony::KEY_CHOICES
                .iter()
                .position(|(name, _, _)| *name == self.key_name())
                .unwrap_or(0),
            Menu::Scale => self.params.scale.value() as usize,
            Menu::MappingKind => match self.mapping().kind {
                3 => 2,
                1 | 2 => 1,
                _ => 0,
            },
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
            Menu::KeyboardParam(id) => {
                if let Some(c) = self.control(id) {
                    let (min, max) = keyboard_menu_range(id).expect("keyboard dropdown");
                    let value = (min + index as i32).min(max) as f32;
                    Self::emit(cx, c.ptr, unsafe { c.ptr.preview_normalized(value) });
                }
            }
            Menu::RouteSource => {
                let route = &self.params.routes[self.route_slot];
                if index > 0
                    && !crate::engine::routing::TARGETS[route.target.value() as usize]
                        .accepts_source(index as u8)
                {
                    return;
                }
                Self::emit(
                    cx,
                    route.source.as_ptr(),
                    route.source.preview_normalized(index as i32),
                );
            }
            Menu::RouteTarget => {
                let Some((index, _)) = crate::engine::routing::available_targets().nth(index)
                else {
                    return;
                };
                let route = &self.params.routes[self.route_slot];
                if !crate::engine::routing::TARGETS[index]
                    .accepts_source(route.source.value() as u8)
                {
                    return;
                }
                Self::emit(
                    cx,
                    route.target.as_ptr(),
                    route.target.preview_normalized(index as i32),
                );
            }
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
            Menu::Key => {
                let (_, key, spelling) = harmony::KEY_CHOICES[index];
                Self::emit(
                    cx,
                    self.params.key.as_ptr(),
                    self.params.key.preview_normalized(key as i32),
                );
                Self::emit(
                    cx,
                    self.params.key_spelling.as_ptr(),
                    self.params.key_spelling.preview_normalized(spelling),
                );
            }
            Menu::Scale => Self::emit(
                cx,
                self.params.scale.as_ptr(),
                self.params.scale.preview_normalized(index as i32),
            ),
            _ => {
                let mut m = self.mapping();
                match menu {
                    Menu::MappingKind => {
                        m.kind = [0, 1, 3][index];
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
                // New CC choices accept any channel; saved channel locks remain compatible.
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
    fn keyboard_layout(&self) -> u8 {
        match self.params.scale_layout.value() {
            0 => self.params.fifths.value() as u8,
            n => n as u8 + 1,
        }
    }
    fn set_keyboard_layout(&mut self, cx: &mut EventContext, layout: u8) {
        self.release_keys();
        if layout < 2 {
            self.set(cx, "fifths", layout as f32);
        }
        Self::emit(
            cx,
            self.params.scale_layout.as_ptr(),
            self.params
                .scale_layout
                .preview_normalized(layout.saturating_sub(1) as i32),
        );
    }
    fn key_name(&self) -> String {
        harmony::key_name(
            self.params.key.value() as u8,
            self.params.scale.value() as u8,
            self.params.key_spelling.value(),
        )
    }
    fn note_name(&self, note: u8) -> String {
        harmony::note_name_in_key(
            note,
            self.params.key.value() as u8,
            self.params.scale.value() as u8,
            self.params.key_spelling.value(),
        )
    }
    fn named_chord(&self, chord: SavedChord) -> String {
        harmony::chord_name_in_key(
            chord,
            self.params.key.value() as u8,
            self.params.scale.value() as u8,
            self.params.key_spelling.value(),
        )
    }
    fn keyboard_chord(&self, index: usize) -> Option<harmony::KeyboardChord> {
        harmony::keyboard_chord(
            self.keyboard_layout(),
            self.params.key.value() as u8,
            self.params.scale.value() as u8,
            index,
        )
        .filter(|chord| self.params.keyboard_octave.value() + chord.root as i32 <= 127)
    }
    fn keyboard_key_rect(&self, index: usize) -> Rect {
        let r = key_rect(index);
        if self.keyboard_chord(index).is_some() {
            r
        } else {
            (r.0, r.1, 0.0, 0.0)
        }
    }
    fn selection_mode(&self) -> usize {
        if self.params.always_chord.value() {
            2
        } else if self
            .routed_plain("root_on_select")
            .map_or(self.params.root_on_select.value(), |v| v >= 0.5)
        {
            1
        } else {
            0
        }
    }
    fn cycle_selection_mode(&self, cx: &mut EventContext) {
        self.step_selection_mode(cx, 1);
    }
    fn step_selection_mode(&self, cx: &mut EventContext, step: i32) {
        let next = (self.selection_mode() as i32 + step).rem_euclid(3);
        self.set(cx, "always_chord", (next == 2) as u8 as f32);
        self.set(cx, "root_on_select", (next == 1) as u8 as f32);
    }
    fn play_key(&mut self, key: usize) {
        if self.pressed[key] {
            return;
        }
        if let Some(chord) = self.keyboard_chord(key) {
            self.pressed[key] = true;
            self.bridge.send(Command::KeyDown(
                key as u8,
                (self.params.keyboard_octave.value() + chord.root as i32) as u8,
                chord.quality,
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
            && self.panel != Some(Panel::Mapping)
            && self.menu.is_none()
            && self.edit.is_none()
            && self.drag.is_none()
            && self.page_elapsed >= pleasant_ui::page_slide::DURATION
    }
    fn set_pad_xy(&self, cx: &mut EventContext, x: f32, y: f32) {
        let (px, py) = self.pad_norm(x, y);
        pleasant_ui::param::set_normalized(cx, self.params.x.as_ptr(),
            px);
        pleasant_ui::param::set_normalized(cx, self.params.y.as_ptr(),
            py);
    }
    fn begin_pad(&mut self, cx: &mut EventContext, x: f32, y: f32, capture: bool) {
        let starting = !self.pad_hover && !matches!(self.drag, Some(Drag::Pad));
        let (px, py) = self.pad_norm(x, y);
        if starting {
            for ptr in [self.params.x.as_ptr(), self.params.y.as_ptr()] {
                pleasant_ui::param::begin(cx, ptr);
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
            pleasant_ui::param::end(cx, ptr);
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
                        pleasant_ui::param::end(cx, ptr);
                    }
                    self.bridge.send(Command::EndGesture);
                }
                Drag::StrumBound(y_axis, right) => {
                    pleasant_ui::param::end(cx, self.bound_param(y_axis, right).as_ptr());
                }
                Drag::RouteContour { slot, .. } => {
                    pleasant_ui::param::end(cx, self.params.routes[slot].min.as_ptr());
                    pleasant_ui::param::end(cx, self.params.routes[slot].max.as_ptr());
                }
                Drag::RouteNode(_, ptr) => pleasant_ui::param::end(cx, ptr),
                Drag::BassSplit => pleasant_ui::param::end(cx, self.params.bass_split.as_ptr()),
                Drag::Split => pleasant_ui::param::end(cx, self.params.split_note.as_ptr()),
                Drag::Memory(_) | Drag::Route(_) => {}
                Drag::Control(drag) => {
                    if drag.started {
                        pleasant_ui::param::end(cx, drag.ptr);
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
        if self.selected_modulator.is_some() {
            self.modulator_pulse = (self.modulator_pulse + dt * 3.0) % std::f32::consts::TAU;
            cx.needs_redraw();
        }
        let was_routing = self.route_elapsed < pleasant_ui::page_slide::DURATION;
        self.route_elapsed = (self.route_elapsed + dt).min(pleasant_ui::page_slide::DURATION);
        self.route_progress = pleasant_ui::page_slide::position(
            self.route_start,
            if self.panel == Some(Panel::Routes) {
                1.0
            } else {
                0.0
            },
            self.route_elapsed,
        );
        if was_routing {
            cx.needs_redraw();
        }
        let was_sliding = self.page_elapsed < pleasant_ui::page_slide::DURATION;
        let mode = performance_page(self.mode());
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
        self.receive_learned_split(cx);
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
        for i in 0..128 {
            if old.note_strikes[i] != self.snapshot.note_strikes[i] {
                self.note_anim[i] = 1.0;
            }
            if self.note_anim[i] > 0.001 {
                changed = true;
                self.note_anim[i] = (self.note_anim[i] - dt * 3.5).max(0.0);
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
        for i in 0..MEMORY_COUNT {
            let v = self.params.slot(i).load(Ordering::Relaxed);
            if v != self.slot_signature[i] {
                self.slot_signature[i] = v;
                changed = true;
            }
        }
        self.voicing_pulse = (self.voicing_pulse + dt * 4.0) % std::f32::consts::TAU;
        let voicing_hovered = self
            .hover_pointer()
            .is_some_and(|(x, y)| self.voicing_bounds().is_some_and(|r| hit(r, x, y)));
        if self.current_chord_key().is_some()
            || voicing_hovered
            || self.hover_pointer().is_some_and(|(x, y)| {
                hit(PIANO_LEADING, x, y) || hit(self.control_octave_bounds(), x, y)
            })
            || self.learning() == 4
        {
            changed = true;
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
pub(crate) fn initial_editor_size() -> (u32, u32) {
    (W as u32, H as u32)
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
            nih_plug_vizia::widgets::ResizeHandle::new(cx)
                .position_type(PositionType::SelfDirected)
                .right(Pixels(0.0))
                .bottom(Pixels(0.0))
                .width(Pixels(16.0))
                .height(Pixels(16.0));
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
