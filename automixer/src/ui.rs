use crate::{
    network::{self, Network, CAPACITY},
    params::AutomixerParams,
};
use nih_plug::prelude::*;
use nih_plug_vizia::{
    create_vizia_editor,
    vizia::{prelude::*, vg::FontId},
    widgets::util::ModifiersExt,
    ViziaTheming,
};
use pleasant_ui::{
    draw::{Draw, EditorViewport},
    load_font_once, param,
    pointer::ValuePress,
    point_in_rect,
    readout::{self, EditAction, PressAction},
    start_frame_timer,
    value_edit::ValueEdit,
    FONT_JETBRAINS_MONO, GOLD, LINE, MUTED, PANEL, TEAL, TEXT,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

type Rect = (f32, f32, f32, f32);
const SIZE: (f32, f32) = (1120.0, 760.0);
const THEME: Rect = (1016.0, 21.0, 72.0, 30.0);
const LOCAL: Rect = (802.0, 8.0, 194.0, 58.0);
const PREVIOUS: Rect = (944.0, 179.0, 34.0, 28.0);
const NEXT: Rect = (1054.0, 179.0, 34.0, 28.0);
const PER_PAGE: usize = 8;
const ROW: f32 = 22.0;
const CHANNELS: i32 = 17;
const CC_COUNT: i32 = 120;
const CC_VISIBLE: i32 = 12;
fn channel_label(value: i32) -> String {
    if value == 0 {
        "Off".into()
    } else {
        value.to_string()
    }
}
pleasant_ui::appearance_store!("Automixer");

#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Amount,
    VoiceCap,
    OtherCap,
    Voices,
    Threshold,
    Hold,
    Fade,
    Assist,
    Mic,
    Weight(usize),
}
const GLOBAL: [Control; 8] = [
    Control::Amount,
    Control::VoiceCap,
    Control::OtherCap,
    Control::Voices,
    Control::Threshold,
    Control::Hold,
    Control::Fade,
    Control::Assist,
];
#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuKind {
    Channel(usize),
    Cc(usize),
}
struct OpenMenu {
    kind: MenuKind,
    scroll: i32,
}
#[derive(Clone, Copy)]
enum Target {
    Parameter(Control),
    Name(usize),
}

#[derive(Clone, Copy)]
struct BypassButton {
    slot: usize,
    owner: u64,
    rect: Rect,
}

enum Parameter<'a> {
    Float(&'a FloatParam),
    Int(&'a IntParam),
}
macro_rules! parameter_method {
    ($self:expr, $name:ident $(, $arg:expr)*) => { match $self {
        Parameter::Float(p) => p.$name($($arg),*), Parameter::Int(p) => p.$name($($arg),*),
    } };
}
impl Parameter<'_> {
    fn ptr(&self) -> ParamPtr {
        parameter_method!(self, as_ptr)
    }
    fn normalized(&self) -> f32 {
        parameter_method!(self, unmodulated_normalized_value)
    }
    fn default(&self) -> f32 {
        parameter_method!(self, default_normalized_value)
    }
    fn text(&self, value: f32, unit: bool) -> String {
        parameter_method!(self, normalized_value_to_string, value, unit)
    }
    fn parse(&self, value: &str) -> Option<f32> {
        parameter_method!(self, string_to_normalized_value, value)
    }
    fn step(&self, value: f32, positive: bool, fine: bool) -> f32 {
        if positive {
            parameter_method!(self, next_normalized_step, value, fine)
        } else {
            parameter_method!(self, previous_normalized_step, value, fine)
        }
    }
}

struct AutomixerView {
    params: Arc<AutomixerParams>,
    network: Arc<Network>,
    local_slot: Option<usize>,
    font: Cell<Option<FontId>>,
    hover: Option<(f32, f32)>,
    page: usize,
    selected: usize,
    // A click targets the strip that was drawn, even if the registry has since changed.
    bypass_buttons: Cell<[Option<BypassButton>; PER_PAGE]>,
    drag: Option<(Control, (f32, f32), f32)>,
    press: Option<ValuePress<Control>>,
    edit: Option<ValueEdit<Target>>,
    menu: Option<OpenMenu>,
    group_press: Option<usize>,
}

impl AutomixerView {
    fn new(params: Arc<AutomixerParams>, network: Arc<Network>, local_slot: Option<usize>) -> Self {
        let page = Self::slots(&network)
            .iter()
            .position(|&index| Some(index) == local_slot)
            .unwrap_or(0)
            / PER_PAGE;
        Self {
            selected: params.mic_index(),
            params,
            network,
            local_slot,
            font: Cell::new(None),
            hover: None,
            page,
            bypass_buttons: Cell::new([None; PER_PAGE]),
            drag: None,
            press: None,
            edit: None,
            menu: None,
            group_press: None,
        }
    }

    fn parameter(&self, id: Control) -> Parameter<'_> {
        let p = &self.params.shared;
        match id {
            Control::Amount => Parameter::Float(&p.amount),
            Control::VoiceCap => Parameter::Float(&p.cap),
            Control::OtherCap => Parameter::Float(&p.other_cap),
            Control::Voices => Parameter::Int(&p.voices),
            Control::Threshold => Parameter::Float(&p.threshold),
            Control::Hold => Parameter::Float(&p.hold),
            Control::Fade => Parameter::Float(&p.fade),
            Control::Assist => Parameter::Float(&p.assist),
            Control::Mic => Parameter::Int(&self.params.mic),
            Control::Weight(i) => Parameter::Float(&p.mics[i].weight),
        }
    }
    fn normalized(&self, id: Control) -> f32 {
        if let Control::Weight(i) = id {
            (self.params.shared.mics[i].effective_weight() + 12.0) / 24.0
        } else {
            self.parameter(id).normalized()
        }
    }
    fn prepare_edit(&self, id: Control, normalized: f32) {
        if let Control::Weight(i) = id {
            let mic = &self.params.shared.mics[i];
            // Changed values clear MIDI from the parameter callback, when the
            // host actually applies them. A no-op base value has no callback.
            if mic.weight.preview_plain(normalized.clamp(0.0, 1.0)) == mic.weight.value() {
                mic.manual_edit();
            }
        }
    }
    fn slots(network: &Network) -> Vec<usize> {
        let mut slots: Vec<_> = network
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.owner.load(Ordering::Acquire) != 0)
            .map(|(i, _)| i)
            .collect();
        slots.sort_by_key(|&i| network.slots[i].mic.load(Ordering::Acquire));
        slots
    }
    fn visible(&self) -> Vec<(usize, Rect)> {
        let slots = Self::slots(&self.network);
        let page = self.page.min(Self::page_count(slots.len()) - 1);
        Self::strips(slots, page).collect()
    }
    fn page_count(registered: usize) -> usize {
        registered.div_ceil(PER_PAGE).max(1)
    }
    fn strips(slots: Vec<usize>, page: usize) -> impl Iterator<Item = (usize, Rect)> {
        slots
            .into_iter()
            .skip(page * PER_PAGE)
            .take(PER_PAGE)
            .enumerate()
            .map(|(column, slot)| (slot, (32.0 + column as f32 * 133.0, 220.0, 125.0, 328.0)))
    }
    fn name_rect(r: Rect) -> Rect {
        (r.0 + 5.0, r.1 + 20.0, r.2 - 10.0, 24.0)
    }
    fn weight_rect(r: Rect) -> Rect {
        (r.0 + 8.0, 490.0, r.2 - 16.0, 53.0)
    }
    fn group_rect(r: Rect) -> Rect {
        (r.0 + 8.0, 424.0, r.2 - 16.0, 24.0)
    }
    fn global_rect(index: usize) -> Rect {
        (32.0 + index as f32 * 133.0, 98.0, 125.0, 64.0)
    }
    fn channel_rect() -> Rect {
        (32.0, 592.0, 280.0, 64.0)
    }
    fn cc_rect() -> Rect {
        (328.0, 592.0, 280.0, 64.0)
    }
    fn bypass_at(&self, point: (f32, f32)) -> Option<BypassButton> {
        self.bypass_buttons
            .get()
            .into_iter()
            .flatten()
            .find(|button| point_in_rect(point.0, point.1, button.rect))
    }
    fn toggle_bypass_at(&self, point: (f32, f32)) -> bool {
        let Some(button) = self.bypass_at(point) else {
            return false;
        };
        if let Some(params) = self.network.slots[button.slot].editor_params(button.owner) {
            params.toggle_strip_bypass();
        }
        true
    }
    fn change_page(&mut self, forward: bool) {
        let last = Self::page_count(Self::slots(&self.network).len()) - 1;
        let current = self.page.min(last);
        let next = if forward {
            (current + 1).min(last)
        } else {
            current.saturating_sub(1)
        };
        if next != current {
            self.bypass_buttons.set([None; PER_PAGE]);
        }
        self.page = next;
    }
    fn controls(&self) -> Vec<(Control, Rect)> {
        let mut result: Vec<_> = GLOBAL
            .into_iter()
            .enumerate()
            .map(|(i, id)| (id, Self::global_rect(i)))
            .collect();
        result.push((Control::Mic, LOCAL));
        for (slot, rect) in self.visible() {
            result.push((
                Control::Weight(self.network.slots[slot].mic.load(Ordering::Acquire)),
                Self::weight_rect(rect),
            ));
        }
        result
    }
    fn value_rect(r: Rect) -> Rect {
        (r.0 + r.2 * 0.43, r.1 + 3.0, r.2 * 0.57 - 6.0, 24.0)
    }
    fn end_drag(&mut self, cx: &mut EventContext) {
        if let Some((id, _, _)) = self.drag.take() {
            param::end(cx, self.parameter(id).ptr());
        }
        self.press = None;
        cx.release();
    }
    fn commit(&mut self, cx: &mut EventContext) {
        let Some(mut edit) = self.edit.take() else {
            return;
        };
        match edit.target {
            Target::Parameter(id) => {
                if let Some(value) = self
                    .parameter(id)
                    .parse(&edit.text)
                    .filter(|v| v.is_finite())
                {
                    self.prepare_edit(id, value);
                    param::set_normalized_once(cx, self.parameter(id).ptr(), value);
                } else {
                    edit.invalid = true;
                    self.edit = Some(edit);
                }
            }
            Target::Name(i) => {
                let name = edit.text.trim().chars().take(12).collect::<String>();
                *self.params.shared.mics[i]
                    .name
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = name;
            }
        }
    }
    fn menu_spec(kind: MenuKind) -> (Rect, i32, i32) {
        match kind {
            MenuKind::Channel(_) => (Self::channel_rect(), CHANNELS, CHANNELS),
            MenuKind::Cc(_) => (Self::cc_rect(), CC_COUNT, CC_VISIBLE),
        }
    }
    fn menu_value(&self, kind: MenuKind) -> i32 {
        let mic = match kind {
            MenuKind::Channel(mic) | MenuKind::Cc(mic) => mic,
        };
        let mic = &self.params.shared.mics[mic];
        match kind {
            MenuKind::Channel(_) => mic.midi_channel.value(),
            MenuKind::Cc(_) => mic.midi_cc.value(),
        }
    }
    fn menu_body(kind: MenuKind, rows: i32) -> Rect {
        let (anchor, _, _) = Self::menu_spec(kind);
        let height = rows as f32 * ROW + 8.0;
        let y = (anchor.1 - 6.0 - height).max(8.0);
        (anchor.0, y, anchor.2, height)
    }
    fn menu_row(body: Rect, row: i32) -> Rect {
        (
            body.0 + 4.0,
            body.1 + 4.0 + row as f32 * ROW,
            body.2 - 8.0,
            ROW - 1.0,
        )
    }
    fn menu_index_at(&self, point: (f32, f32)) -> Option<i32> {
        let menu = self.menu.as_ref()?;
        let (_, count, visible) = Self::menu_spec(menu.kind);
        let rows = visible.min(count);
        let body = Self::menu_body(menu.kind, rows);
        if !point_in_rect(point.0, point.1, body) {
            return None;
        }
        let row = ((point.1 - (body.1 + 4.0)) / ROW) as i32;
        if row < 0 || row >= rows {
            return None;
        }
        let index = menu.scroll + row;
        (index >= 0 && index < count).then_some(index)
    }
    fn open_menu(&mut self, kind: MenuKind) {
        let current = self.menu_value(kind);
        let (_, count, visible) = Self::menu_spec(kind);
        let max_scroll = (count - visible).max(0);
        self.menu = Some(OpenMenu {
            kind,
            scroll: (current - visible / 2).clamp(0, max_scroll),
        });
    }
    fn scroll_menu(&mut self, dy: f32) {
        let Some(kind) = self.menu.as_ref().map(|menu| menu.kind) else {
            return;
        };
        let (_, count, visible) = Self::menu_spec(kind);
        let max_scroll = (count - visible).max(0);
        let step = if dy > 0.0 { -1 } else if dy < 0.0 { 1 } else { 0 };
        if let Some(menu) = &mut self.menu {
            menu.scroll = (menu.scroll + step).clamp(0, max_scroll);
        }
    }
    fn choose_menu(&mut self, cx: &mut EventContext, index: i32) {
        let Some(kind) = self.menu.as_ref().map(|menu| menu.kind) else {
            return;
        };
        let mic = match kind {
            MenuKind::Channel(mic) | MenuKind::Cc(mic) => mic,
        };
        let param = match kind {
            MenuKind::Channel(_) => &self.params.shared.mics[mic].midi_channel,
            MenuKind::Cc(_) => &self.params.shared.mics[mic].midi_cc,
        };
        if let Some(value) = param.string_to_normalized_value(&index.to_string()) {
            param::set_normalized_once(cx, param.as_ptr(), value);
        }
    }
    fn group_at(&self, point: (f32, f32)) -> Option<usize> {
        self.visible().into_iter().find_map(|(slot, rect)| {
            point_in_rect(point.0, point.1, Self::group_rect(rect))
                .then(|| self.network.slots[slot].mic.load(Ordering::Acquire))
        })
    }
    fn finish_group(&mut self, cx: &mut EventContext, point: (f32, f32)) {
        let Some(mic) = self.group_press.take() else {
            return;
        };
        if self.group_at(point) != Some(mic) {
            return;
        }
        let next = if self.params.shared.mics[mic].is_voice() {
            1.0
        } else {
            0.0
        };
        param::set_normalized_once(cx, self.params.shared.mics[mic].group.as_ptr(), next);
    }
    fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window: &WindowEvent, meta| {
            let b = cx.bounds();
            let Some(viewport) = EditorViewport::fit((b.x, b.y, b.w, b.h), SIZE) else {
                return;
            };
            let point = viewport.to_local(cx.mouse().cursorx, cx.mouse().cursory);
            match readout::handle_press(&mut self.press, window, point) {
                PressAction::Inactive => {}
                PressAction::Blocked => return,
                PressAction::BeginDrag(press) => {
                    let initial = self.normalized(press.target);
                    param::begin(cx, self.parameter(press.target).ptr());
                    self.drag = Some((press.target, press.origin, initial));
                }
                PressAction::BeginEdit(press) => {
                    cx.release();
                    self.edit = Some(ValueEdit::new(
                        Target::Parameter(press.target),
                        press.rect,
                        self.parameter(press.target)
                            .text(self.normalized(press.target), false),
                    ));
                    nih_plug_vizia::consume_window_event(cx, window, meta);
                    cx.needs_redraw();
                    return;
                }
                PressAction::Released | PressAction::Cancelled => {
                    cx.release();
                }
            }
            if let Some(edit) = &mut self.edit {
                match readout::handle_edit(edit, cx, window, point) {
                    EditAction::Commit => {
                        self.commit(cx);
                    }
                    EditAction::CommitAndContinue => {
                        self.commit(cx);
                        self.end_drag(cx);
                    }
                    EditAction::Cancel => {
                        self.edit = None;
                    }
                    EditAction::Handled => {}
                    EditAction::Blocked => return,
                }
                nih_plug_vizia::consume_window_event(cx, window, meta);
                cx.needs_redraw();
                return;
            }
            match window {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if self.menu.is_some() {
                        if let Some(index) = self.menu_index_at(point) {
                            self.choose_menu(cx, index);
                        }
                        self.menu = None;
                    } else if point_in_rect(point.0, point.1, THEME) {
                        prefs().toggle();
                    } else if self.toggle_bypass_at(point) {
                    } else if point_in_rect(point.0, point.1, PREVIOUS) {
                        self.change_page(false);
                    } else if point_in_rect(point.0, point.1, NEXT) {
                        self.change_page(true);
                    } else if point_in_rect(point.0, point.1, Self::channel_rect()) {
                        let mic = self.selected.min(CAPACITY - 1);
                        self.open_menu(MenuKind::Channel(mic));
                    } else if point_in_rect(point.0, point.1, Self::cc_rect()) {
                        let mic = self.selected.min(CAPACITY - 1);
                        self.open_menu(MenuKind::Cc(mic));
                    } else if let Some(mic) = self.group_at(point) {
                        self.selected = mic;
                        self.group_press = Some(mic);
                    } else {
                        if let Some((slot, _)) = self
                            .visible()
                            .into_iter()
                            .find(|(_, r)| point_in_rect(point.0, point.1, *r))
                        {
                            self.selected = self.network.slots[slot].mic.load(Ordering::Acquire);
                        }
                        if let Some((id, r)) = self
                            .controls()
                            .into_iter()
                            .find(|(_, r)| point_in_rect(point.0, point.1, *r))
                        {
                            let value_rect = Self::value_rect(r);
                            if point_in_rect(point.0, point.1, value_rect) {
                                self.press = Some(ValuePress::new(id, value_rect, point));
                            } else {
                                let initial = self.normalized(id);
                                param::begin(cx, self.parameter(id).ptr());
                                self.drag = Some((id, point, initial));
                            }
                            cx.capture();
                        }
                    }
                }
                WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                    self.group_press = None;
                    self.menu = None;
                    self.end_drag(cx);
                    if let Some((slot, r)) = self
                        .visible()
                        .into_iter()
                        .find(|(_, r)| point_in_rect(point.0, point.1, Self::name_rect(*r)))
                    {
                        let mic = self.network.slots[slot].mic.load(Ordering::Acquire);
                        let name = self.params.shared.mics[mic]
                            .name
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .clone();
                        self.edit =
                            Some(ValueEdit::new(Target::Name(mic), Self::name_rect(r), name));
                    } else if let Some((id, _)) = self
                        .controls()
                        .into_iter()
                        .find(|(_, r)| point_in_rect(point.0, point.1, *r))
                    {
                        self.prepare_edit(id, self.parameter(id).default());
                        param::set_normalized_once(
                            cx,
                            self.parameter(id).ptr(),
                            self.parameter(id).default(),
                        );
                    }
                }
                WindowEvent::MouseMove(_, _) => {
                    self.hover = Some(point);
                    if let Some((id, origin, initial)) = self.drag {
                        let scale = if cx.modifiers().shift() {
                            0.0005
                        } else {
                            0.005
                        };
                        let value = initial + (point.0 - origin.0 + origin.1 - point.1) * scale;
                        self.prepare_edit(id, value);
                        param::set_normalized(cx, self.parameter(id).ptr(), value);
                    }
                }
                WindowEvent::MouseScroll(_, dy) if self.menu.is_some() => self.scroll_menu(*dy),
                WindowEvent::MouseScroll(_, dy) if self.drag.is_none() => {
                    let mic = self.selected.min(CAPACITY - 1);
                    let fine = cx.modifiers().shift();
                    let list = if point_in_rect(point.0, point.1, Self::channel_rect()) {
                        Some(&self.params.shared.mics[mic].midi_channel)
                    } else if point_in_rect(point.0, point.1, Self::cc_rect()) {
                        Some(&self.params.shared.mics[mic].midi_cc)
                    } else {
                        None
                    };
                    if let Some(param) = list {
                        let current = param.unmodulated_normalized_value();
                        let next = if *dy > 0.0 {
                            param.next_normalized_step(current, fine)
                        } else {
                            param.previous_normalized_step(current, fine)
                        };
                        let ptr = param.as_ptr();
                        param::set_normalized_once(cx, ptr, next);
                    } else if let Some((id, _)) = self
                        .controls()
                        .into_iter()
                        .find(|(_, r)| point_in_rect(point.0, point.1, *r))
                    {
                        let value = self.parameter(id).step(
                            self.normalized(id),
                            *dy > 0.0,
                            cx.modifiers().shift(),
                        );
                        self.prepare_edit(id, value);
                        param::set_normalized_once(cx, self.parameter(id).ptr(), value);
                    }
                }
                WindowEvent::MouseUp(MouseButton::Left) => {
                    self.finish_group(cx, point);
                    self.end_drag(cx);
                }
                WindowEvent::FocusOut
                | WindowEvent::KeyDown(Code::Escape, _)
                | WindowEvent::MouseDown(MouseButton::Right) => {
                    self.menu = None;
                    self.group_press = None;
                    self.end_drag(cx);
                }
                WindowEvent::MouseLeave => {
                    self.hover = None;
                    self.menu = None;
                    self.group_press = None;
                }
                _ => return,
            }
            nih_plug_vizia::consume_window_event(cx, window, meta);
            cx.needs_redraw();
        });
    }

    fn display_name(&self, mic: usize, host_name: Option<&str>) -> String {
        let fallback = format!("MIC {:02}", mic + 1);
        let alias = self.params.shared.mics[mic]
            .name
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if alias.is_empty() || *alias == fallback {
            host_name.map(str::to_owned).unwrap_or(fallback)
        } else {
            alias.clone()
        }
    }

    fn paint_bypass(d: &mut Draw, rect: Rect, pressed: bool) {
        let color = if pressed { GOLD } else { TEXT };
        d.button_surface(
            rect,
            GOLD,
            pressed as u8 as f32,
            d.is_hovered(rect) as u8 as f32,
            0.0,
        );
        let x = rect.0 + rect.2 * 0.5;
        let y = rect.1 + rect.3 * 0.5 - 1.0;
        d.power_icon(x - 23.0, y, color);
        d.text_middle(x - 12.0, y, "BYPASS", 11.0, color);
    }

    fn paint_page_button(d: &mut Draw, rect: Rect, forward: bool, enabled: bool) {
        let x = rect.0 + rect.2 * 0.5;
        let y = rect.1 + rect.3 * 0.5 - 1.0;
        let direction = if forward { 1.0 } else { -1.0 };
        if enabled {
            d.button_surface(rect, TEXT, 0.0, d.is_hovered(rect) as u8 as f32, 0.0);
        }
        d.poly(
            &[
                (x - 2.0 * direction, y - 5.0),
                (x + 3.0 * direction, y),
                (x - 2.0 * direction, y + 5.0),
            ],
            if enabled { TEXT } else { MUTED },
            1.5,
        );
    }

    fn paint(&self, d: &mut Draw) {
        d.hover = if self.drag.is_none() && self.edit.is_none() {
            self.hover
        } else {
            None
        };
        let p = &self.params.shared;
        d.rect(0.0, 0.0, SIZE.0, 72.0, PANEL);
        d.text(32.0, 46.0, "AUTOMIXER", 25.0, GOLD);
        d.appearance_button(THEME, prefs().label());
        d.text(32.0, 88.0, "ALL MICROPHONES", 10.0, MUTED);
        let labels = [
            "AUTOMIX",
            "VOICE GR",
            "OTHER GR",
            "CHORUS > N",
            "THRESH",
            "HOLD",
            "FADE",
            "VOICE ASSIST",
        ];
        for (index, id) in GLOBAL.into_iter().enumerate() {
            self.paint_control(d, id, Self::global_rect(index), labels[index]);
        }
        self.paint_control(d, Control::Mic, LOCAL, "THIS MIC");
        let monitor = self.network.monitor(self.network.now_ms());
        let active = monitor.active;
        let chorus = monitor.chorus;
        let slots = Self::slots(&self.network);
        let registered = slots.len();
        d.text(
            32.0,
            199.0,
            &format!(
                "{registered} LINKED   {} IN MIX   {active} ACTIVE",
                monitor.members
            ),
            14.0,
            TEXT,
        );
        let state = if p.amount.value() == 0.0 {
            "AUTOMIX OFF".into()
        } else if monitor.members == 0 {
            "NO MICS IN MIX".into()
        } else if chorus > 0.001 {
            format!("CHORUS FADE  {:3.0}%", chorus * 100.0)
        } else {
            "GAIN SHARING".into()
        };
        d.text(
            398.0,
            199.0,
            &state,
            13.0,
            if chorus > 0.0 { TEAL } else { GOLD },
        );
        let pages = Self::page_count(registered);
        let page = self.page.min(pages - 1);
        Self::paint_page_button(d, PREVIOUS, false, page > 0);
        Self::paint_page_button(d, NEXT, true, page + 1 < pages);
        d.text_centered(
            1016.0,
            197.0,
            &format!("{} / {pages}", page + 1),
            12.0,
            TEXT,
        );
        let now = self.network.now_ms();
        let conflicts = self.network.conflict_mask();
        let mut bypass_buttons = [None; PER_PAGE];
        for (column, (slot_index, r)) in Self::strips(slots, page).enumerate() {
            let slot = &self.network.slots[slot_index];
            let owner = slot.owner.load(Ordering::Acquire);
            let params = slot.editor_params(owner);
            let strip_bypassed = params
                .as_ref()
                .is_some_and(|params| params.strip_bypassed());
            let host_bypassed = params.as_ref().is_some_and(|params| params.bypass.value());
            let mic = slot.mic.load(Ordering::Acquire).min(CAPACITY - 1);
            let labels = slot
                .labels
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let local = Some(slot_index) == self.local_slot;
            let updated = slot.updated_ms.load(Ordering::Relaxed);
            let fresh = updated != 0 && now.saturating_sub(updated) < 750;
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            d.outline(r, if self.selected == mic { GOLD } else { LINE });
            let name = self.display_name(mic, labels.name.as_deref());
            let center = r.0 + r.2 * 0.5;
            d.text_centered(
                center,
                236.0,
                &format!(
                    "MIC {:02}{}",
                    mic + 1,
                    if local { " · THIS MIC" } else { "" }
                ),
                9.0,
                if local { GOLD } else { MUTED },
            );
            d.text_centered(
                center,
                258.0,
                &name.chars().take(12).collect::<String>(),
                13.0,
                TEXT,
            );
            if let Some(input) = &labels.input {
                d.text_centered(
                    center,
                    276.0,
                    &format!("IN {:02}", input.first_channel + 1),
                    10.0,
                    MUTED,
                );
            }
            let state = slot.state.load(Ordering::Relaxed);
            let has_meter = fresh && state != network::WAITING;
            let (status, color) = if host_bypassed {
                ("HOST BYPASS", GOLD)
            } else if strip_bypassed {
                ("BYPASSED", GOLD)
            } else if conflicts & (1 << mic) != 0 {
                ("ID CONFLICT", GOLD)
            } else if !has_meter {
                ("NO AUDIO", MUTED)
            } else {
                match state {
                    network::BYPASSED => ("BYPASSED", GOLD),
                    network::CHORUS => ("CHORUS", TEAL),
                    network::QUIET => ("QUIET", MUTED),
                    _ => ("ACTIVE", TEAL),
                }
            };
            d.text_centered(center, 297.0, status, 10.0, color);
            let input = if has_meter {
                slot.input_db.load(Ordering::Relaxed)
            } else {
                -120.0
            };
            let output = if has_meter {
                slot.output_db.load(Ordering::Relaxed)
            } else {
                -120.0
            };
            let gr = if has_meter {
                slot.reduction_db.load(Ordering::Relaxed)
            } else {
                0.0
            };
            let cap = if p.mics[mic].is_voice() {
                p.cap.value()
            } else {
                p.other_cap.value()
            };
            let gr_norm = (gr / cap.max(1.0)).clamp(0.0, 1.0);
            for (column, label, value, normalized, color) in [
                (
                    0,
                    "IN",
                    input,
                    ((input + 60.0) / 60.0).clamp(0.0, 1.0),
                    MUTED,
                ),
                (
                    1,
                    "OUT",
                    output,
                    ((output + 60.0) / 60.0).clamp(0.0, 1.0),
                    TEAL,
                ),
                (2, "GR", gr, gr_norm, GOLD),
            ] {
                let x = r.0 + 22.0 + column as f32 * 40.0;
                let height = normalized * 70.0;
                d.text_centered(x, 318.0, label, 9.0, MUTED);
                d.rect(x - 6.0, 328.0, 12.0, 70.0, LINE);
                if height > 0.0 {
                    let top = if column == 2 { 328.0 } else { 398.0 - height };
                    d.rect(x - 6.0, top, 12.0, height, color);
                }
                if has_meter {
                    d.text_centered(x, 412.0, &format!("{:.0}", value.max(-99.0)), 10.0, color);
                } else {
                    d.text_centered(x, 412.0, "–", 10.0, MUTED);
                }
            }
            let voice = p.mics[mic].is_voice();
            let group_label = if !voice {
                "OTHER".to_string()
            } else if p.assist.value() > 0.0 && !strip_bypassed && !host_bypassed {
                if !has_meter || !slot.vad_ready.load(Ordering::Relaxed) {
                    "VOICE WAIT".into()
                } else {
                    format!("VOICE {:3.0}%", slot.speech.load(Ordering::Relaxed) * 100.0)
                }
            } else {
                "VOICE".into()
            };
            d.button(
                Self::group_rect(r),
                &group_label,
                true,
                if voice { TEAL } else { GOLD },
            );
            let rect = (r.0 + 8.0, 455.0, r.2 - 16.0, 29.0);
            if params.is_some() {
                bypass_buttons[column] = Some(BypassButton {
                    slot: slot_index,
                    owner,
                    rect,
                });
            }
            Self::paint_bypass(d, rect, strip_bypassed);
            self.paint_control(d, Control::Weight(mic), Self::weight_rect(r), "WEIGHT");
        }
        self.bypass_buttons.set(bypass_buttons);
        if registered == 0 {
            d.text(50.0, 260.0, "NO INSTANCES", 14.0, MUTED);
        }
        let selected = &p.mics[self.selected];
        let labels = self
            .network
            .slots
            .iter()
            .find(|slot| {
                slot.owner.load(Ordering::Acquire) != 0
                    && slot.mic.load(Ordering::Acquire) == self.selected
            })
            .map(|slot| {
                slot.labels
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone()
            })
            .unwrap_or_default();
        let name = self.display_name(self.selected, labels.name.as_deref());
        if let Some(input) = &labels.input {
            d.text(
                32.0,
                675.0,
                &input.label.chars().take(140).collect::<String>(),
                11.0,
                MUTED,
            );
        }
        d.text(
            32.0,
            576.0,
            &format!(
                "SELECTED  {:02}  {}  ·  {}",
                self.selected + 1,
                name.chars().take(48).collect::<String>(),
                if selected.is_voice() { "VOICE" } else { "OTHER" }
            ),
            12.0,
            GOLD,
        );
        let mic = self.selected.min(CAPACITY - 1);
        self.paint_dropdown(
            d,
            Self::channel_rect(),
            "MIDI CHANNEL",
            &channel_label(selected.midi_channel.value()),
            matches!(self.menu, Some(OpenMenu { kind: MenuKind::Channel(open), .. }) if open == mic),
        );
        self.paint_dropdown(
            d,
            Self::cc_rect(),
            "WEIGHT CC",
            &selected.midi_cc.value().to_string(),
            matches!(self.menu, Some(OpenMenu { kind: MenuKind::Cc(open), .. }) if open == mic),
        );
        let warning = if self.local_slot.is_none() {
            Some("CAPACITY REACHED · 64 INSTANCES MAXIMUM")
        } else if conflicts != 0 {
            Some("MIC ID CONFLICT · ASSIGN A UNIQUE THIS MIC NUMBER")
        } else {
            None
        };
        if let Some(warning) = warning {
            d.text(32.0, 706.0, warning, 11.0, GOLD);
        }
        d.line(32.0, 722.0, 1088.0, 722.0, LINE, 1.0);
        d.text(32.0, 742.0, self.tooltip(), 10.0, MUTED);
        self.paint_menu(d);
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        }
    }

    fn paint_dropdown(&self, d: &mut Draw, rect: Rect, label: &str, value: &str, open: bool) {
        d.button_surface(
            rect,
            TEAL,
            open as u8 as f32,
            d.is_hovered(rect) as u8 as f32,
            0.0,
        );
        d.text(rect.0 + 12.0, rect.1 + 18.0, label, 9.5, MUTED);
        d.text(rect.0 + 12.0, rect.1 + 44.0, value, 16.0, TEXT);
        let x = rect.0 + rect.2 - 22.0;
        let y = rect.1 + 40.0;
        d.poly(
            &[(x, y - 4.0), (x + 10.0, y - 4.0), (x + 5.0, y + 3.0)],
            TEXT,
            1.5,
        );
    }

    fn paint_menu(&self, d: &mut Draw) {
        let Some(menu) = &self.menu else {
            return;
        };
        let (_, count, visible) = Self::menu_spec(menu.kind);
        let rows = visible.min(count);
        let body = Self::menu_body(menu.kind, rows);
        d.rect(body.0, body.1, body.2, body.3, PANEL);
        d.outline(body, LINE);
        let current = self.menu_value(menu.kind);
        for row in 0..rows {
            let index = menu.scroll + row;
            let item = Self::menu_row(body, row);
            if d.hover
                .is_some_and(|point| point_in_rect(point.0, point.1, item))
                || index == current
            {
                d.rect(item.0, item.1, item.2, item.3, LINE);
            }
            let label = if matches!(menu.kind, MenuKind::Channel(_)) && index == 0 {
                "Off".to_string()
            } else {
                index.to_string()
            };
            d.text_middle(
                item.0 + 10.0,
                item.1 + item.3 * 0.5,
                &label,
                12.0,
                if index == current { GOLD } else { TEXT },
            );
        }
    }

    fn paint_control(&self, d: &mut Draw, id: Control, rect: Rect, label: &str) {
        let normalized = self.normalized(id);
        let value = if id == Control::Mic && self.params.mic.value() == 0 {
            format!("Auto {:02}", self.params.mic_index() + 1)
        } else {
            self.parameter(id)
                .text(normalized, !matches!(id, Control::Weight(_)))
        };
        if matches!(id, Control::Weight(_)) {
            d.control_bipolar(rect, label, &value, normalized, GOLD);
        } else {
            d.control(
                rect,
                label,
                &value,
                normalized,
                if id == Control::Amount { GOLD } else { TEAL },
            );
        }
        if self
            .hover
            .is_some_and(|p| point_in_rect(p.0, p.1, Self::value_rect(rect)))
        {
            d.value_underline(Self::value_rect(rect), rect.1 + 18.0, GOLD);
        }
    }
    fn tooltip(&self) -> &'static str {
        if self.edit.is_some() {
            return "Type a value or a microphone name. Enter applies; Escape cancels. Names are saved with the project.";
        }
        if let Some(menu) = &self.menu {
            return match menu.kind {
                MenuKind::Channel(_) => "Choose the MIDI channel. Off ignores cues for this microphone. Scroll to move through the list.",
                MenuKind::Cc(_) => "Choose the weight CC, 0 through 119. Value 0 is -12 dB, 64 is 0 dB, and 127 is +12 dB. Scroll the list.",
            };
        }
        let Some(point) = self.hover else {
            return "All instances share this mixer. Click VOICE or OTHER on a strip to set its group. Drag a weight, or click its value to type.";
        };
        if self.bypass_at(point).is_some() {
            return "Bypass sends this mic at unity and removes it from its group, the mix, and the active count. Host bypass is controlled by the DAW.";
        }
        if self.group_at(point).is_some() {
            return "Voice mics share with each other. Chorus and voice assist apply only to them. Other mics share separately, with their own max GR.";
        }
        if point_in_rect(point.0, point.1, THEME) {
            return "Cycle Dark, Light, and Auto appearance.";
        }
        if point_in_rect(point.0, point.1, PREVIOUS) || point_in_rect(point.0, point.1, NEXT) {
            return "Browse the linked microphone strips.";
        }
        if point_in_rect(point.0, point.1, Self::channel_rect()) {
            return "MIDI channel for the selected microphone. Click to open the list. Off ignores cues. Scroll to step while the list is closed.";
        }
        if point_in_rect(point.0, point.1, Self::cc_rect()) {
            return "Weight CC for the selected microphone, 0-119. Click to open the list. The latest cue replaces the weight. Scroll to step while closed.";
        }
        if let Some((id, _)) = self
            .controls()
            .into_iter()
            .find(|(_, r)| point_in_rect(point.0, point.1, *r))
        {
            return match id {
                Control::Amount => "Automix depth for every microphone. 0% is unity gain. 100% applies the full computed reduction, including quiet voice mics during chorus.",
                Control::VoiceCap => "Maximum reduction for Voice microphones, in sharing and in chorus. 0 dB keeps every voice mic at unity.",
                Control::OtherCap => "Maximum reduction for Other microphones. They gain-share only with each other and ignore chorus.",
                Control::Voices => "When more than N Voice mics are active, those active mics fade to unity. Quiet voice mics stay at Voice GR. Zero disables chorus. Other mics are not counted.",
                Control::Threshold => "Input RMS activity threshold, before weighting. 3 dB hysteresis and a 20 ms onset debounce prevent flicker.",
                Control::Hold => "Hold activity below threshold, and hold chorus after the voice count drops. This prevents chatter between voices.",
                Control::Fade => "Fade time for Automix and for voice chorus transitions. Quiet voice mics remain at Voice GR while chorus opens the active voice mics.",
                Control::Assist => "Voice group only. Speech confidence lowers that mic's activity threshold by up to 6 dB. 0% disables it. Other mics use the plain threshold.",
                Control::Mic => "Auto prefers the hardware input number when audio activates. Enter a number to pin the mic and its MIDI cue mapping.",
                Control::Weight(_) => "Priority, not output gain: -12 to +12 dB. Editing replaces the last MIDI cue. Shift drags finely; double-click resets.",
            };
        }
        "IN / OUT: RMS dBFS. GR: applied reduction in dB. VOICE shows speech confidence when voice assist is on. Linked instances must share a format and host process."
    }
}

impl View for AutomixerView {
    fn element(&self) -> Option<&'static str> {
        Some("automixer-view")
    }
    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let editing = self.edit.is_some();
        self.handle_event(cx, event);
        pleasant_ui::value_edit::sync_text_input(cx, editing, self.edit.is_some());
    }
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let font = load_font_once(canvas, &self.font, FONT_JETBRAINS_MONO);
        let b = cx.bounds();
        if let Some(mut d) = Draw::new_fitted(canvas, prefs().light(), (b.x, b.y, b.w, b.h), SIZE, font)
        {
            self.paint(&mut d);
        }
    }
}

pub fn create(
    params: Arc<AutomixerParams>,
    network: Arc<Network>,
    local_slot: Option<usize>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(
        params.editor_state.clone(),
        ViziaTheming::Custom,
        move |cx, _| {
            AutomixerView::new(params.clone(), network.clone(), local_slot)
                .build(cx, |cx| {
                    start_frame_timer(cx, Duration::from_millis(33), |cx| cx.needs_redraw());
                })
                .width(Stretch(1.0))
                .height(Stretch(1.0));
        },
    )
}

#[cfg(all(test, target_os = "macos"))]
#[path = "ui_preview.rs"]
mod preview;
