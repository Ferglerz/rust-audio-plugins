use crate::{
    network::{self, Network, CAPACITY},
    params::{AutomixerParams, MicGroup, MixMode},
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
    param,
    pointer::ValuePress,
    readout::{self, EditAction, PressAction},
    value_edit::ValueEdit,
    AppearanceStore, FONT_JETBRAINS_MONO, GOLD, LINE, MUTED, PANEL, TEAL, TEXT,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc, OnceLock},
    time::{Duration, Instant},
};

type Rect = (f32, f32, f32, f32);
const SIZE: (f32, f32) = (1120.0, 960.0);
const THEME: Rect = (1016.0, 17.0, 72.0, 30.0);
const PAGE_PREV: Rect = (848.0, 17.0, 28.0, 28.0);
const PAGE_NEXT: Rect = (968.0, 17.0, 28.0, 28.0);
const PRIMARY_Y: f32 = 76.0;
const CONTROL_H: f32 = 58.0;
const SETUP_Y: f32 = 810.0;
const STRIPS_TOP: f32 = 216.0;
const SPECTRAL_SETUP_BUTTON: Rect = (1050.0, 156.0, 38.0, 38.0);

const STRIP_BOTTOM: f32 = 748.0;
const SETUP_BUTTON: Rect = (1050.0, 86.0, 38.0, 38.0);
const MODE_BUTTON: Rect = (240.0, 17.0, 220.0, 30.0);
const PER_PAGE: usize = 8;
const NAME_LINE: usize = 14;
const NAME_LIMIT: usize = NAME_LINE * 2;
const ROW: f32 = 22.0;
const CHANNELS: i32 = 17;
const CC_COUNT: i32 = 120;
const CC_VISIBLE: i32 = CHANNELS;
fn inside(p: (f32, f32), r: Rect) -> bool {
    p.0 >= r.0 && p.0 <= r.0 + r.2 && p.1 >= r.1 && p.1 <= r.1 + r.3
}

struct StripLayout {
    mic_label: Rect,
    name: Rect,
    meta: Rect,
    meters: Rect,
    speech: Rect,
    group: Rect,
    midi: Rect,
    bypass: Rect,
    weight: Rect,
    spectral_weight: Rect,
    eq_blend: Rect,
}

impl StripLayout {
    fn new(strip: Rect, _local: bool) -> Self {
        let bottom = strip.1 + strip.3;
        let eq_blend = (strip.0 + 8.0, bottom - 60.0, strip.2 - 16.0, 52.0);
        let spectral_weight = (strip.0 + 8.0, bottom - 116.0, strip.2 - 16.0, 52.0);
        let weight = (strip.0 + 8.0, bottom - 172.0, strip.2 - 16.0, 52.0);
        let group = (strip.0 + 8.0, weight.1 - 32.0, strip.2 - 16.0, 26.0);
        let midi = (strip.0 + 8.0, group.1 - 28.0, strip.2 - 16.0, 24.0);
        let bypass = (strip.0 + strip.2 - 30.0, midi.1 - 28.0, 22.0, 22.0);
        let speech = (strip.0 + 8.0, bypass.1, bypass.0 - strip.0 - 12.0, 22.0);
        let mut y = strip.1 + 8.0;
        let this_mic = (strip.0 + 6.0, y, strip.2 - 12.0, 16.0);
        let mic_label = this_mic;
        y += 16.0;
        let name = (strip.0 + 6.0, y, strip.2 - 12.0, 34.0);
        y += 36.0;
        let meta = (strip.0 + 6.0, y, strip.2 - 12.0, 16.0);
        y += 18.0;
        let meters = (
            strip.0 + 8.0,
            y,
            strip.2 - 16.0,
            (speech.1 - 8.0 - y).max(48.0),
        );
        Self {
            mic_label,
            name,
            meta,
            meters,
            speech,
            group,
            midi,
            bypass,
            weight,
            spectral_weight,
            eq_blend,
        }
    }

    fn spectrum(&self) -> Rect {
        (self.meters.0, self.meters.1, self.meters.2, 70.0)
    }

    fn spectral_meters(&self) -> Rect {
        (
            self.meters.0,
            self.meters.1 + 78.0,
            self.meters.2,
            self.meters.3 - 78.0,
        )
    }
}

fn name_lines(name: &str) -> [String; 2] {
    let all: Vec<char> = name.chars().collect();
    let truncated = all.len() > NAME_LIMIT;
    let chars = &all[..all.len().min(NAME_LIMIT)];
    if chars.len() <= NAME_LINE {
        return [chars.iter().collect(), String::new()];
    }
    let split = chars[..NAME_LINE]
        .iter()
        .rposition(|c| c.is_whitespace())
        .filter(|index| *index > 0)
        .unwrap_or(NAME_LINE);
    let mut rest = chars[split..]
        .iter()
        .copied()
        .skip_while(|c| c.is_whitespace());
    let mut second: String = rest.by_ref().take(NAME_LINE).collect();
    if (truncated || rest.next().is_some()) && !second.is_empty() {
        second.pop();
        second.push('…');
    }
    [
        chars[..split].iter().collect::<String>().trim().to_string(),
        second,
    ]
}
fn channel_label(value: i32) -> String {
    if value == 0 {
        "Off".into()
    } else {
        value.to_string()
    }
}
fn prefs() -> &'static AppearanceStore {
    static PREFS: OnceLock<AppearanceStore> = OnceLock::new();
    PREFS.get_or_init(|| AppearanceStore::new("Automixer"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Amount,
    SpectralBlend(usize),
    SpectralWeight(usize),
    SpectralAmount,
    SpectralCut,
    Low,
    High,
    Attack,
    Release,
    Smooth,
    VoiceCap,
    OtherCap,
    Voices,
    Threshold,
    Hold,
    Fade,
    Assist,
    Weight(usize),
}
const PRIMARY: [Control; 3] = [Control::Amount, Control::VoiceCap, Control::OtherCap];
const SETUP: [Control; 5] = [
    Control::Voices,
    Control::Threshold,
    Control::Hold,
    Control::Fade,
    Control::Assist,
];
const PRIMARY_WIDTHS: [f32; 3] = [220.0, 160.0, 160.0];
#[derive(Clone, Copy, PartialEq, Eq)]
enum MenuKind {
    Channel(usize),
    Cc(usize),
}
struct OpenMenu {
    mic: usize,
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
    setup_open: bool,
    spectral_setup: bool,
    tray_started: Instant,
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
            setup_open: false,
            spectral_setup: false,
            tray_started: Instant::now(),
        }
    }

    fn parameter(&self, id: Control) -> Parameter<'_> {
        let p = &self.params.shared;
        match id {
            Control::Amount => Parameter::Float(&p.amount),
            Control::SpectralBlend(i) => Parameter::Float(&p.mics[i].eq_blend),
            Control::SpectralWeight(i) => Parameter::Float(&p.mics[i].spectral_weight),
            Control::SpectralAmount => Parameter::Float(&p.spectral_amount),
            Control::SpectralCut => Parameter::Float(&p.spectral_cut),
            Control::Low => Parameter::Float(&p.spectral_low),
            Control::High => Parameter::Float(&p.spectral_high),
            Control::Attack => Parameter::Float(&p.spectral_attack),
            Control::Release => Parameter::Float(&p.spectral_release),
            Control::Smooth => Parameter::Float(&p.spectral_smooth),
            Control::VoiceCap => Parameter::Float(&p.cap),
            Control::OtherCap => Parameter::Float(&p.other_cap),
            Control::Voices => Parameter::Int(&p.voices),
            Control::Threshold => Parameter::Float(&p.threshold),
            Control::Hold => Parameter::Float(&p.hold),
            Control::Fade => Parameter::Float(&p.fade),
            Control::Assist => Parameter::Float(&p.assist),
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
        self.strips(slots, page).collect()
    }
    fn page_count(registered: usize) -> usize {
        registered.div_ceil(PER_PAGE).max(1)
    }
    fn spectral(&self) -> bool {
        self.params.shared.mode.value() == MixMode::SpectralSpace
    }
    fn primary(&self) -> &[Control] {
        &PRIMARY
    }
    fn spectral_control_rect(index: usize) -> Rect {
        let (x, y, w, h) = Self::primary_rect(index);
        (x, y + 70.0, w, h)
    }
    fn setup(&self) -> &[Control] {
        if self.spectral_setup {
            &[
                Control::Low,
                Control::High,
                Control::Attack,
                Control::Release,
                Control::Smooth,
            ]
        } else {
            &SETUP
        }
    }
    fn tray_ready(&self) -> bool {
        self.tray_started.elapsed().as_secs_f32() >= pleasant_ui::page_slide::DURATION
    }
    fn strips_top(&self) -> f32 {
        STRIPS_TOP
    }
    fn strips(&self, slots: Vec<usize>, page: usize) -> impl Iterator<Item = (usize, Rect)> {
        let top = self.strips_top();
        let height = STRIP_BOTTOM - top;
        slots
            .into_iter()
            .skip(page * PER_PAGE)
            .take(PER_PAGE)
            .enumerate()
            .map(move |(column, slot)| (slot, (32.0 + column as f32 * 133.0, top, 125.0, height)))
    }
    fn primary_rect(index: usize) -> Rect {
        let mut x = 32.0;
        for width in PRIMARY_WIDTHS.iter().take(index) {
            x += width + 10.0;
        }
        (x, PRIMARY_Y, PRIMARY_WIDTHS[index], CONTROL_H)
    }
    fn setup_control_rect(index: usize) -> Rect {
        let width = (1088.0 - 32.0 - 4.0 * 10.0) / 5.0;
        (
            32.0 + index as f32 * (width + 10.0),
            SETUP_Y,
            width,
            CONTROL_H,
        )
    }
    fn status_rect(&self) -> Rect {
        (602.0, PRIMARY_Y, 438.0, CONTROL_H)
    }
    fn bypass_at(&self, point: (f32, f32)) -> Option<BypassButton> {
        self.bypass_buttons
            .get()
            .into_iter()
            .flatten()
            .find(|button| inside(point, button.rect))
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
        self.menu = None;
        if let Some((slot, _)) = self.visible().first() {
            self.selected = self.network.slots[*slot].mic.load(Ordering::Acquire);
        }
    }
    fn controls(&self) -> Vec<(Control, Rect)> {
        let mut result: Vec<_> = self
            .primary()
            .iter()
            .copied()
            .enumerate()
            .map(|(i, id)| (id, Self::primary_rect(i)))
            .collect();
        if self.spectral() {
            result.push((Control::SpectralAmount, Self::spectral_control_rect(0)));
            result.push((Control::SpectralCut, Self::spectral_control_rect(1)));
        }
        if self.setup_open {
            result.extend(
                self.setup()
                    .iter()
                    .copied()
                    .enumerate()
                    .map(|(i, id)| (id, Self::setup_control_rect(i))),
            );
        }
        for (slot, rect) in self.visible() {
            let layout = StripLayout::new(rect, Some(slot) == self.local_slot);
            let mic = self.network.slots[slot].mic.load(Ordering::Acquire);
            result.push((Control::Weight(mic), layout.weight));
            result.push((Control::SpectralWeight(mic), layout.spectral_weight));
            result.push((Control::SpectralBlend(mic), layout.eq_blend));
        }
        result
    }
    fn value_rect(r: Rect) -> Rect {
        if r.2 < 125.0 {
            (r.0 + 6.0, r.1 + 14.0, r.2 - 12.0, 14.0)
        } else {
            (r.0 + r.2 * 0.43, r.1 + 3.0, r.2 * 0.57 - 6.0, 24.0)
        }
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
                let name = edit
                    .text
                    .trim()
                    .chars()
                    .take(NAME_LIMIT)
                    .collect::<String>();
                *self.params.shared.mics[i]
                    .name
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = name;
            }
        }
    }
    fn midi_at(&self, point: (f32, f32)) -> Option<usize> {
        self.visible().into_iter().find_map(|(slot, rect)| {
            inside(point, StripLayout::new(rect, false).midi)
                .then(|| self.network.slots[slot].mic.load(Ordering::Acquire))
        })
    }
    fn menu_body(&self) -> Option<Rect> {
        let mic = self.menu.as_ref()?.mic;
        let (_, strip) = self
            .visible()
            .into_iter()
            .find(|(slot, _)| self.network.slots[*slot].mic.load(Ordering::Acquire) == mic)?;
        let anchor = StripLayout::new(strip, false).midi;
        let height = CHANNELS as f32 * ROW + 32.0;
        Some((
            anchor.0.min(SIZE.0 - 264.0),
            (anchor.1 - height - 4.0).max(8.0),
            256.0,
            height,
        ))
    }
    fn menu_row(body: Rect, column: usize, row: i32) -> Rect {
        (
            body.0 + 4.0 + column as f32 * 124.0,
            body.1 + 28.0 + row as f32 * ROW,
            120.0,
            ROW - 1.0,
        )
    }
    fn menu_index_at(&self, point: (f32, f32)) -> Option<(MenuKind, i32)> {
        let menu = self.menu.as_ref()?;
        let body = self.menu_body()?;
        for row in 0..CHANNELS {
            if inside(point, Self::menu_row(body, 0, row)) {
                return Some((MenuKind::Channel(menu.mic), row));
            }
            if inside(point, Self::menu_row(body, 1, row)) {
                return Some((MenuKind::Cc(menu.mic), menu.scroll + row));
            }
        }
        None
    }
    fn open_menu(&mut self, mic: usize) {
        let current = self.params.shared.mics[mic].midi_cc.value();
        self.menu = Some(OpenMenu {
            mic,
            scroll: (current - CC_VISIBLE / 2).clamp(0, CC_COUNT - CC_VISIBLE),
        });
    }
    fn scroll_menu(&mut self, dy: f32) {
        if let Some(menu) = &mut self.menu {
            let step = if dy > 0.0 {
                -1
            } else if dy < 0.0 {
                1
            } else {
                0
            };
            menu.scroll = (menu.scroll + step).clamp(0, CC_COUNT - CC_VISIBLE);
        }
    }
    fn choose_menu(&mut self, cx: &mut EventContext, kind: MenuKind, index: i32) {
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
            let layout = StripLayout::new(rect, Some(slot) == self.local_slot);
            inside(point, layout.group)
                .then(|| self.network.slots[slot].mic.load(Ordering::Acquire))
        })
    }
    fn speech_at(&self, point: (f32, f32)) -> bool {
        self.visible().into_iter().any(|(slot, rect)| {
            let layout = StripLayout::new(rect, Some(slot) == self.local_slot);
            inside(point, layout.speech)
        })
    }
    fn finish_group(&mut self, cx: &mut EventContext, point: (f32, f32)) {
        let Some(mic) = self.group_press.take() else {
            return;
        };
        if self.group_at(point) != Some(mic) {
            return;
        }
        let next = match self.params.shared.mics[mic].group.value() {
            MicGroup::Voice => 0.5,
            MicGroup::Other => 1.0,
            MicGroup::Drums => 0.0,
        };
        param::set_normalized_once(cx, self.params.shared.mics[mic].group.as_ptr(), next);
    }
    fn handle_event(&mut self, cx: &mut EventContext, event: &mut Event) {
        let visible = self.visible();
        if !visible
            .iter()
            .any(|(slot, _)| self.network.slots[*slot].mic.load(Ordering::Acquire) == self.selected)
        {
            if let Some((slot, _)) = visible.first() {
                self.selected = self.network.slots[*slot].mic.load(Ordering::Acquire);
                self.menu = None;
            }
        }
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
            if point.1 >= 790.0
                && point.1 < 910.0
                && !self.tray_ready()
                && matches!(
                    window,
                    WindowEvent::MouseDown(_)
                        | WindowEvent::MouseScroll(_, _)
                        | WindowEvent::MouseDoubleClick(_)
                )
            {
                return;
            }
            match window {
                WindowEvent::MouseDown(MouseButton::Left) => {
                    if self.menu.is_some() {
                        if let Some((kind, index)) = self.menu_index_at(point) {
                            self.choose_menu(cx, kind, index);
                        } else if !self.menu_body().is_some_and(|body| inside(point, body)) {
                            self.menu = None;
                        }
                    } else if inside(point, THEME) {
                        prefs().toggle();
                    } else if inside(point, MODE_BUTTON) {
                        self.end_drag(cx);
                        self.menu = None;
                        self.setup_open = false;
                        param::set_normalized_once(
                            cx,
                            self.params.shared.mode.as_ptr(),
                            if self.spectral() { 0.0 } else { 1.0 },
                        );
                    } else if self.spectral() && inside(point, SPECTRAL_SETUP_BUTTON) {
                        self.setup_open = !self.setup_open || !self.spectral_setup;
                        self.spectral_setup = true;
                        self.tray_started = Instant::now();
                    } else if inside(point, SETUP_BUTTON) {
                        self.setup_open = !self.setup_open || self.spectral_setup;
                        self.spectral_setup = false;
                        self.tray_started = Instant::now();
                    } else if self.toggle_bypass_at(point) {
                    } else if self.page_count_now() > 1 && inside(point, PAGE_PREV) {
                        self.change_page(false);
                    } else if self.page_count_now() > 1 && inside(point, PAGE_NEXT) {
                        self.change_page(true);
                    } else if let Some(mic) = self.midi_at(point) {
                        self.selected = mic;
                        self.open_menu(mic);
                    } else if let Some(mic) = self.group_at(point) {
                        self.selected = mic;
                        self.group_press = Some(mic);
                    } else {
                        if let Some((slot, _)) =
                            self.visible().into_iter().find(|(_, r)| inside(point, *r))
                        {
                            self.selected = self.network.slots[slot].mic.load(Ordering::Acquire);
                        }
                        if let Some((id, r)) =
                            self.controls().into_iter().find(|(_, r)| inside(point, *r))
                        {
                            let value_rect = Self::value_rect(r);
                            if inside(point, value_rect) {
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
                    if let Some((slot, r)) = self.visible().into_iter().find(|(slot, r)| {
                        let layout = StripLayout::new(*r, Some(*slot) == self.local_slot);
                        inside(point, layout.name)
                    }) {
                        let mic = self.network.slots[slot].mic.load(Ordering::Acquire);
                        let name = self.params.shared.mics[mic]
                            .name
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .clone();
                        let layout = StripLayout::new(r, Some(slot) == self.local_slot);
                        self.edit = Some(ValueEdit::new(Target::Name(mic), layout.name, name));
                    } else if let Some((id, _)) =
                        self.controls().into_iter().find(|(_, r)| inside(point, *r))
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
                    if let Some((id, _)) =
                        self.controls().into_iter().find(|(_, r)| inside(point, *r))
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
                    self.setup_open = false;
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

    fn page_count_now(&self) -> usize {
        Self::page_count(Self::slots(&self.network).len())
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
        let hovered = d.is_hovered(rect);
        let color = if pressed {
            GOLD
        } else if hovered {
            TEXT
        } else {
            MUTED
        };
        if pressed || hovered {
            d.button_surface(rect, GOLD, pressed as u8 as f32, hovered as u8 as f32, 0.0);
        }
        d.power_icon(rect.0 + rect.2 * 0.5, rect.1 + rect.3 * 0.5, color);
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
        d.rect(0.0, 0.0, SIZE.0, 64.0, PANEL);
        d.text(32.0, 42.0, "AUTOMIXER", 25.0, GOLD);
        d.appearance_button(THEME, prefs().label());
        d.button(
            MODE_BUTTON,
            if self.spectral() {
                "SPECTRAL ON"
            } else {
                "SPECTRAL OFF"
            },
            self.spectral(),
            TEAL,
        );
        let monitor = self.network.monitor(self.network.now_ms());
        let slots = Self::slots(&self.network);
        let registered = slots.len();
        let pages = Self::page_count(registered);
        let page = self.page.min(pages - 1);
        if pages > 1 {
            Self::paint_page_button(d, PAGE_PREV, false, page > 0);
            Self::paint_page_button(d, PAGE_NEXT, true, page + 1 < pages);
            d.text_centered(
                (PAGE_PREV.0 + PAGE_PREV.2 + PAGE_NEXT.0) * 0.5,
                36.0,
                &format!("{} / {pages}", page + 1),
                12.0,
                TEXT,
            );
        }
        for (index, &id) in self.primary().iter().enumerate() {
            let label = match id {
                Control::Amount => "GAIN AMOUNT",
                Control::SpectralCut => "MAX CUT",
                Control::VoiceCap => "VOICE MAX GR",
                _ => "OTHER MAX GR",
            };
            self.paint_control(d, id, Self::primary_rect(index), label);
        }
        self.paint_status(
            d,
            monitor.chorus,
            registered,
            monitor.voice_active,
            monitor.members,
        );
        self.paint_cog(d, SETUP_BUTTON, self.setup_open && !self.spectral_setup);
        if self.spectral() {
            self.paint_control(
                d,
                Control::SpectralAmount,
                Self::spectral_control_rect(0),
                "DUCK AMOUNT",
            );
            self.paint_control(
                d,
                Control::SpectralCut,
                Self::spectral_control_rect(1),
                "MAX CUT",
            );
            self.paint_cog(
                d,
                SPECTRAL_SETUP_BUTTON,
                self.setup_open && self.spectral_setup,
            );
        }
        let now = self.network.now_ms();
        let conflicts = self.network.conflict_mask();
        let mut bypass_buttons = [None; PER_PAGE];
        for (column, (slot_index, r)) in self.strips(slots, page).enumerate() {
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
            let layout = StripLayout::new(r, local);
            let name = self.display_name(mic, labels.name.as_deref());
            let center = r.0 + r.2 * 0.5;
            d.text_centered(
                center,
                layout.mic_label.1 + 12.0,
                &format!("{} {:02}", if local { "THIS MIC" } else { "MIC" }, mic + 1),
                9.0,
                if local { TEAL } else { MUTED },
            );
            let lines = name_lines(&name);
            let name_y = if lines[1].is_empty() {
                layout.name.1 + 22.0
            } else {
                layout.name.1 + 14.0
            };
            d.text_centered(center, name_y, &lines[0], 13.0, TEXT);
            if !lines[1].is_empty() {
                d.text_centered(center, layout.name.1 + 30.0, &lines[1], 13.0, TEXT);
            }
            let state = slot.state.load(Ordering::Relaxed);
            let has_meter = fresh && state != network::WAITING;
            let meta = if host_bypassed {
                Some(("HOST BYPASS", GOLD))
            } else if conflicts & (1 << mic) != 0 {
                Some(("ID CONFLICT", GOLD))
            } else if strip_bypassed {
                Some(("BYPASSED", GOLD))
            } else if !has_meter {
                Some(("NO METER DATA", MUTED))
            } else {
                None
            };
            if let Some((text, color)) = meta {
                d.text_centered(center, layout.meta.1 + 12.0, text, 10.0, color);
            } else if let Some(input) = &labels.input {
                d.text_centered(
                    center,
                    layout.meta.1 + 12.0,
                    &format!("INPUT {:02}", input.first_channel + 1),
                    10.0,
                    MUTED,
                );
            }
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
            let gr_norm = (gr / 24.0).clamp(0.0, 1.0);
            let meters = if self.spectral() {
                self.paint_spectrum(d, slot, layout.spectrum());
                layout.spectral_meters()
            } else {
                layout.meters
            };
            let bar_top = meters.1 + 16.0;
            let bar_bottom = meters.1 + meters.3 - 16.0;
            let bar_h = (bar_bottom - bar_top).max(24.0);
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
                let height = normalized * bar_h;
                d.text_centered(x, meters.1 + 10.0, label, 9.0, MUTED);
                d.rect(x - 6.0, bar_top, 12.0, bar_h, LINE);
                if height > 0.0 {
                    let top = if column == 2 {
                        bar_top
                    } else {
                        bar_top + bar_h - height
                    };
                    d.rect(x - 6.0, top, 12.0, height, color);
                }
                let readout = if has_meter {
                    format!("{:.0}", value.max(-99.0))
                } else {
                    "–".into()
                };
                d.text_centered(
                    x,
                    bar_bottom + 12.0,
                    &readout,
                    10.0,
                    if has_meter { color } else { MUTED },
                );
            }
            let voice = p.mics[mic].is_voice();
            if voice && p.assist.value() > 0.0 && !strip_bypassed && !host_bypassed {
                let speech = if !has_meter || !slot.vad_ready.load(Ordering::Relaxed) {
                    "SPEECH WAIT".into()
                } else {
                    format!(
                        "SPEECH {:3.0}%",
                        slot.speech.load(Ordering::Relaxed) * 100.0
                    )
                };
                d.text_middle(
                    layout.speech.0,
                    layout.speech.1 + layout.speech.3 * 0.5,
                    &speech,
                    10.0,
                    TEAL,
                );
            }
            d.button(
                layout.group,
                match p.mics[mic].group.value() {
                    MicGroup::Voice => "VOICE",
                    MicGroup::Other => "OTHER",
                    MicGroup::Drums => "DRUMS",
                },
                true,
                if voice { TEAL } else { GOLD },
            );
            if params.is_some() {
                bypass_buttons[column] = Some(BypassButton {
                    slot: slot_index,
                    owner,
                    rect: layout.bypass,
                });
            }
            let mapping = &p.mics[mic];
            let midi_label = if mapping.midi_channel.value() == 0 {
                "MIDI OFF ▾".to_string()
            } else {
                format!(
                    "CH {} · CC {} ▾",
                    mapping.midi_channel.value(),
                    mapping.midi_cc.value()
                )
            };
            d.button(layout.midi, &midi_label, true, MUTED);
            Self::paint_bypass(d, layout.bypass, strip_bypassed);
            self.paint_control(d, Control::Weight(mic), layout.weight, "GAIN WEIGHT");
            self.paint_control(
                d,
                Control::SpectralWeight(mic),
                layout.spectral_weight,
                "SPECTRAL WEIGHT",
            );
            self.paint_control(d, Control::SpectralBlend(mic), layout.eq_blend, "EQ BLEND");
        }
        self.bypass_buttons.set(bypass_buttons);
        if registered == 0 {
            d.text(50.0, self.strips_top() + 40.0, "NO INSTANCES", 14.0, MUTED);
        }
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
        d.text(
            32.0,
            776.0,
            &format!(
                "SELECTED  {:02}  {}",
                self.selected + 1,
                name.chars().take(48).collect::<String>()
            ),
            12.0,
            GOLD,
        );
        if self.setup_open {
            d.scissor(32.0, 790.0, 1056.0, 118.0);
            d.offset_x = 1056.0
                * (1.0
                    - pleasant_ui::page_slide::position(
                        0.0,
                        1.0,
                        self.tray_started.elapsed().as_secs_f32(),
                    ));
            d.rect(32.0, 790.0, 1056.0, 118.0, PANEL);
            for (index, &id) in self.setup().iter().enumerate() {
                let label = match id {
                    Control::Voices => "CHORUS ABOVE",
                    Control::Threshold => "THRESHOLD",
                    Control::Hold => "HOLD",
                    Control::Fade => "FADE",
                    Control::Assist => "VOICE ASSIST",
                    Control::Low => "LOW FREQ",
                    Control::High => "HIGH FREQ",
                    Control::Attack => "ATTACK",
                    Control::Release => "RELEASE",
                    _ => "SMOOTHING",
                };
                self.paint_control(d, id, Self::setup_control_rect(index), label);
            }
            d.offset_x = 0.0;
            d.reset_scissor();
        }
        let warning = if self.local_slot.is_none() {
            Some("CAPACITY REACHED · 64 INSTANCES MAXIMUM")
        } else if conflicts != 0 {
            Some("MIC ID CONFLICT · DUPLICATE IDS EXCLUDED FROM MIX")
        } else {
            None
        };
        if let Some(warning) = warning {
            d.text(32.0, 906.0, warning, 11.0, GOLD);
        }
        d.line(32.0, 922.0, 1088.0, 922.0, LINE, 1.0);
        d.text(32.0, 942.0, self.tooltip(), 10.0, MUTED);
        self.paint_menu(d);
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        }
    }

    fn paint_cog(&self, d: &mut Draw, rect: Rect, open: bool) {
        d.button_surface(
            rect,
            TEAL,
            open as u8 as f32,
            d.is_hovered(rect) as u8 as f32,
            0.0,
        );
        if open {
            d.text_centered(
                rect.0 + rect.2 * 0.5,
                rect.1 + rect.3 * 0.5 + 4.0,
                "×",
                18.0,
                TEAL,
            );
        } else {
            d.cog_icon(rect.0 + rect.2 * 0.5, rect.1 + rect.3 * 0.5, MUTED);
        }
    }
    fn paint_spectrum(&self, d: &mut Draw, slot: &network::Slot, bounds: Rect) {
        let updated = slot.updated_ms.load(Ordering::Relaxed);
        let snapshot = (updated != 0 && self.network.now_ms().saturating_sub(updated) < 750)
            .then(|| slot.spectrum.read())
            .flatten();
        let (left, top, width, height) = bounds;
        d.outline(bounds, LINE);
        let Some((_, _, cut)) = snapshot else {
            d.text_centered(
                left + width * 0.5,
                top + height * 0.5 + 3.0,
                "NO CUT DATA",
                9.0,
                MUTED,
            );
            return;
        };
        for i in 1..crate::spectral::BANDS {
            let x = |i: usize| {
                left + 1.0 + i as f32 / (crate::spectral::BANDS - 1) as f32 * (width - 2.0)
            };
            let cy = |i: usize| top + 1.0 + (cut[i] / 24.0).clamp(0.0, 1.0) * (height - 2.0);
            d.line(x(i - 1), cy(i - 1), x(i), cy(i), GOLD, 1.5);
        }
    }

    fn paint_status(
        &self,
        d: &mut Draw,
        chorus: f32,
        registered: usize,
        active: usize,
        members: usize,
    ) {
        let rect = self.status_rect();
        d.rect(rect.0, rect.1, rect.2, rect.3, PANEL);
        d.outline(rect, LINE);
        let (title, color) = if members == 0 {
            ("NO MICS IN MIX".to_string(), GOLD)
        } else if self.spectral()
            && self.params.shared.spectral_low.value() > self.params.shared.spectral_high.value()
        {
            ("LOW > HIGH · CHECK RANGE".to_string(), GOLD)
        } else if chorus > 0.001 && self.params.shared.amount.value() > 0.0 {
            (
                format!(
                    "CHORUS {:3.0}%{}",
                    chorus * 100.0,
                    if self.spectral() { " + SPECTRAL" } else { "" }
                ),
                TEAL,
            )
        } else if self.params.shared.amount.value() == 0.0 {
            (
                if self.spectral() {
                    "SPECTRAL ONLY"
                } else {
                    "GAIN SHARING OFF"
                }
                .to_string(),
                GOLD,
            )
        } else {
            (
                if self.spectral() {
                    "GAIN + SPECTRAL"
                } else {
                    "GAIN SHARING"
                }
                .to_string(),
                TEAL,
            )
        };
        d.text(rect.0 + 12.0, rect.1 + 22.0, &title, 13.0, color);
        d.text(
            rect.0 + 12.0,
            rect.1 + 42.0,
            &format!("{registered} LINKED · {members} IN MIX · {active} VOICE ACTIVE"),
            11.0,
            MUTED,
        );
    }

    fn paint_menu(&self, d: &mut Draw) {
        let Some(menu) = &self.menu else {
            return;
        };
        let Some(body) = self.menu_body() else {
            return;
        };
        d.rect(body.0, body.1, body.2, body.3, PANEL);
        d.outline(body, LINE);
        d.text(body.0 + 12.0, body.1 + 18.0, "CHANNEL", 10.0, MUTED);
        d.text(body.0 + 136.0, body.1 + 18.0, "WEIGHT CC", 10.0, MUTED);
        let mic = &self.params.shared.mics[menu.mic];
        for row in 0..CHANNELS {
            for column in 0..2 {
                let index = if column == 0 { row } else { menu.scroll + row };
                let current = if column == 0 {
                    mic.midi_channel.value()
                } else {
                    mic.midi_cc.value()
                };
                let item = Self::menu_row(body, column, row);
                if d.hover.is_some_and(|point| inside(point, item)) || index == current {
                    d.rect(item.0, item.1, item.2, item.3, LINE);
                }
                let label = if column == 0 {
                    channel_label(index)
                } else {
                    format!("CC {index}")
                };
                d.text_middle(
                    item.0 + 10.0,
                    item.1 + item.3 * 0.5,
                    &label,
                    12.0,
                    if index == current { TEAL } else { TEXT },
                );
            }
        }
    }

    fn paint_control(&self, d: &mut Draw, id: Control, rect: Rect, label: &str) {
        let normalized = self.normalized(id);
        let value = if id == Control::Voices && self.params.shared.voices.value() == 0 {
            "OFF".to_string()
        } else if let Control::SpectralBlend(mic) = id {
            format!("{:.0}%", self.params.shared.mics[mic].eq_blend.value())
        } else {
            self.parameter(id).text(
                normalized,
                !matches!(id, Control::Weight(_) | Control::SpectralWeight(_)),
            )
        };
        if matches!(
            id,
            Control::Weight(_) | Control::SpectralWeight(_) | Control::SpectralBlend(_)
        ) {
            d.control_bipolar(rect, "", "", normalized, GOLD);
            d.text_centered(rect.0 + rect.2 * 0.5, rect.1 + 11.0, label, 8.0, MUTED);
            d.text_centered(rect.0 + rect.2 * 0.5, rect.1 + 25.0, &value, 11.0, TEXT);
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
            .is_some_and(|p| inside(p, Self::value_rect(rect)))
        {
            d.value_underline(
                Self::value_rect(rect),
                rect.1 + if rect.2 < 125.0 { 26.0 } else { 18.0 },
                GOLD,
            );
        }
    }
    fn tooltip(&self) -> &'static str {
        if let Some(edit) = &self.edit {
            if edit.invalid {
                return "Invalid value. Enter a number in the control's units; Enter applies, Escape cancels.";
            }
            return match edit.target {
                Target::Name(_) => "Rename this mic (28 characters maximum). Clear the name to follow the host track name. Enter applies; Escape cancels.",
                Target::Parameter(_) => "Type a value in the control's units. Enter applies; Escape cancels.",
            };
        }
        if self.menu.is_some() {
            return "Choose channel and weight CC here. Off disables weight cues. Scroll for more CCs; click outside to close. CC values: 0 = -12 dB, 64 = neutral, 127 = +12 dB.";
        }
        let Some(point) = self.hover else {
            return "Controls are shared across linked instances. Drag to adjust; click values to type; double-click to reset.";
        };
        if self.spectral()
            && self
                .visible()
                .iter()
                .any(|(_, rect)| inside(point, StripLayout::new(*rect, false).spectrum()))
        {
            return "Reduction: 40 Hz–20 kHz left to right. Gold: spectral cut only, 0 dB at the top to 24 dB at the bottom, before EQ Blend.";
        }
        if inside(point, MODE_BUTTON) {
            return "Enable spectral ducking and per-track EQ alongside gain sharing for all linked tracks. Adds 2048 samples of latency; switch while stopped.";
        }
        for (_, rect) in self.visible() {
            let layout = StripLayout::new(rect, false);
            if inside(point, layout.name) {
                return "Double-click to rename this mic. Clear the alias to follow the host track name. Names are saved with the project.";
            }
            if inside(point, layout.meta) {
                return "INPUT is the hardware channel. NO METER DATA means no recent update. For ID conflicts, assign a unique mic ID through each affected instance’s host parameter controls.";
            }
        }
        if self.bypass_at(point).is_some() && self.spectral() {
            return "Power icon toggles bypass; gold means bypassed. Removes influence and releases cuts and EQ. FFT delay remains. Clear HOST BYPASS in the DAW.";
        }
        if self.bypass_at(point).is_some() {
            return "Power icon toggles strip bypass; gold means bypassed. Fades to unity and leaves sharing and chorus. Clear HOST BYPASS in the DAW to resume.";
        }
        if self.group_at(point).is_some() && self.spectral() {
            return "Voice shares gain separately with chorus and voice assist. Other and Drums share gain together. All types interact spectrally; Drums have faster high bands.";
        }
        if self.group_at(point).is_some() {
            return "Cycle Voice / Other / Drums. Voice shares separately with chorus and voice assist. Other and Drums share Other Max GR; drums respond faster.";
        }
        if inside(point, THEME) {
            return "Cycle Dark, Light, and Auto appearance.";
        }
        if self.page_count_now() > 1 && (inside(point, PAGE_PREV) || inside(point, PAGE_NEXT)) {
            return "Browse the linked microphone strips.";
        }
        if self.spectral() && inside(point, SPECTRAL_SETUP_BUTTON) {
            return "Open spectral frequency range, attack, release and smoothing settings. Gain sharing stays active.";
        }
        if inside(point, SETUP_BUTTON) {
            return "Open gain-sharing chorus and activity detection settings. These also apply when spectral processing is on.";
        }
        if inside(point, self.status_rect()) && self.spectral() {
            if self.params.shared.spectral_low.value() > self.params.shared.spectral_high.value() {
                return "Low is above High: no spectral cuts or EQ are applied. Correct the range in spectral settings. Gain sharing continues; FFT delay remains.";
            }
            return "Gain sharing and spectral processing run together. SPECTRAL ONLY means gain Amount is zero. IN MIX includes silent tracks; VOICE ACTIVE drives chorus.";
        }
        if inside(point, self.status_rect()) {
            return "LINKED includes idle instances. VOICE ACTIVE counts microphones above threshold, not distinct speakers. Bleed can trigger chorus.";
        }
        if self.speech_at(point) {
            return "Speech confidence for this Voice microphone. Voice Assist lowers its activity threshold by up to 6 dB. WAIT means the detector has no result yet.";
        }
        if self.midi_at(point).is_some() {
            return "MIDI weight assignment for this track. Open to choose both channel and CC; Off disables MIDI cues.";
        }
        if let Some((id, _)) = self.controls().into_iter().find(|(_, r)| inside(point, *r)) {
            return match id {
                Control::SpectralBlend(_) => "This track’s EQ: positive matches tone, negative separates it; zero is neutral. Up to ±6 dB. Independent of weights and Amount; requires Spectral On.",
                Control::SpectralCut => "Maximum spectral cut in dB. Only higher Spectral Weights create cuts; equal Spectral Weights never duck each other.",
                Control::Low | Control::High => "Frequency range for ducking and EQ Blend. Low above High leaves no affected frequencies. FFT delay remains.",
                Control::Attack => "How quickly spectral cuts engage, in addition to FFT analysis time.",
                Control::Release => "Recovery time for ducking. Also sets EQ response, with a 100 ms minimum; Drums use faster high-band timing.",
                Control::Smooth => "Blend adjacent bands for smoother ducking and EQ Blend curves.",
                Control::SpectralAmount => "Depth of spectral cuts across all tracks. Zero disables ducking after release; EQ Blend remains active and FFT latency remains.",
                Control::SpectralWeight(_) => "Spectral weight: higher priorities duck lower ones in overlapping bands. Equal weights add no cuts. Independent of Gain Weight and MIDI; requires Spectral On.",
                Control::Amount => "Gain-sharing depth. Zero disables broadband reduction; spectral ducking and EQ remain independent. 100% applies full sharing and chorus reduction.",
                Control::VoiceCap => "Maximum broadband reduction for Voice mics, including chorus. Zero disables gain-sharing reduction; spectral cuts and EQ remain independent.",
                Control::OtherCap => "Maximum broadband reduction for Other and Drums. They share together and ignore voice chorus. Spectral cuts and EQ remain independent.",
                Control::Voices => "Chorus opens active Voice mics when their count exceeds this value: 3 means 4 or more. Quiet voices stay reduced. Zero disables chorus.",
                Control::Threshold => "Activity threshold before weighting, not a noise gate. Uses 3 dB hysteresis and 20 ms onset confirmation; Drums skip onset confirmation.",
                Control::Hold => "Activity hold plus chorus return hold; the delays add together. Drums use a fixed 20 ms activity hold.",
                Control::Fade => "Fade time for Automix and for voice chorus transitions. Quiet voice mics remain at Voice GR while chorus opens the active voice mics.",
                Control::Assist => "Voice group only. Speech confidence lowers that mic's activity threshold by up to 6 dB. 0% disables it. Other mics use the plain threshold.",
                Control::Weight(_) => "Gain weight sets gain-sharing priority, not output volume. MIDI cues control this weight only. Editing replaces the cue; Shift drags finely; double-click resets.",
            };
        }
        if self.spectral() {
            "IN / OUT: RMS dBFS. GR: gain-sharing reduction plus largest spectral cut; excludes EQ. Gold graph shows spectral cuts only. LINKED includes idle tracks."
        } else {
            "IN / OUT: RMS dBFS. GR: applied reduction, shown on a fixed 0–24 dB scale. Voice shares separately; Other and Drums share together."
        }
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
        if self.font.get().is_none() {
            self.font.set(canvas.add_font_mem(FONT_JETBRAINS_MONO).ok());
        }
        let b = cx.bounds();
        if let Some(mut d) = Draw::new_fitted(
            canvas,
            prefs().light(),
            (b.x, b.y, b.w, b.h),
            SIZE,
            self.font.get(),
        ) {
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
                    let timer = cx.add_timer(Duration::from_millis(33), None, |cx, action| {
                        if let TimerAction::Tick(_) = action {
                            cx.needs_redraw();
                        }
                    });
                    cx.start_timer(timer);
                })
                .width(Stretch(1.0))
                .height(Stretch(1.0));
        },
    )
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    fn view() -> AutomixerView {
        let network = Arc::new(Network::default());
        let params = Arc::new(AutomixerParams::new(network.params.clone(), None, 0));
        AutomixerView::new(params, network, None)
    }

    #[test]
    fn midi_dropdown_targets_strip_and_keeps_channel_and_cc_in_one_popup() {
        let network = Arc::new(Network::default());
        let registrations: Vec<_> = (0..8).map(|_| network.register()).collect();
        let params = Arc::new(AutomixerParams::new(network.params.clone(), None, 0));
        let mut editor = AutomixerView::new(params, network, None);
        let (slot, rect) = editor.visible()[7];
        let mic = editor.network.slots[slot].mic.load(Ordering::Acquire);
        let layout = StripLayout::new(rect, false);
        assert_eq!(
            editor.midi_at((layout.midi.0 + 2.0, layout.midi.1 + 2.0)),
            Some(mic)
        );
        assert!(layout.meters.1 + layout.meters.3 < layout.midi.1);
        editor.open_menu(mic);
        let body = editor.menu_body().unwrap();
        assert!(body.0 >= 0.0 && body.0 + body.2 <= SIZE.0);
        assert!(body.1 >= 0.0 && body.1 + body.3 < 922.0);
        let channel = AutomixerView::menu_row(body, 0, 16);
        assert!(
            matches!(editor.menu_index_at((channel.0 + 2.0, channel.1 + 2.0)), Some((MenuKind::Channel(target), 16)) if target == mic)
        );
        for _ in 0..200 {
            editor.scroll_menu(-1.0);
        }
        let cc = AutomixerView::menu_row(body, 1, 16);
        assert!(
            matches!(editor.menu_index_at((cc.0 + 2.0, cc.1 + 2.0)), Some((MenuKind::Cc(target), 119)) if target == mic)
        );
        drop(registrations);
    }

    #[test]
    fn setup_starts_closed() {
        let ids: Vec<_> = view().controls().into_iter().map(|(id, _)| id).collect();
        assert!(ids.contains(&Control::Amount));
        assert!(ids.contains(&Control::VoiceCap));
        assert!(ids.contains(&Control::OtherCap));
        assert!(!ids.contains(&Control::Threshold));
        assert!(!ids.contains(&Control::Voices));
        assert!(!ids.contains(&Control::Hold));
        assert!(!ids.contains(&Control::Fade));
        assert!(!ids.contains(&Control::Assist));
    }

    #[test]
    fn setup_reveals_detector_controls() {
        let mut editor = view();
        editor.setup_open = true;
        let ids: Vec<_> = editor.controls().into_iter().map(|(id, _)| id).collect();
        for id in [
            Control::Voices,
            Control::Threshold,
            Control::Hold,
            Control::Fade,
            Control::Assist,
        ] {
            assert!(ids.contains(&id));
        }
    }

    #[test]
    fn setup_preserves_meter_geometry_and_page_change_selects_visible_track() {
        let strip = (32.0, STRIPS_TOP, 125.0, STRIP_BOTTOM - STRIPS_TOP);
        assert_eq!(
            StripLayout::new(strip, true).meters,
            StripLayout::new(strip, false).meters
        );
        let network = Arc::new(Network::default());
        let instances: Vec<_> = (0..9).map(|_| crate::Automixer::new(&network)).collect();
        let mut editor = AutomixerView::new(
            instances[0].params.clone(),
            network,
            instances[0].registration.index,
        );
        let before = editor.visible()[0].1;
        editor.setup_open = true;
        assert_eq!(editor.visible()[0].1, before);
        editor.setup_open = false;
        editor.change_page(true);
        assert_eq!(editor.selected, 8);
    }

    #[test]
    fn combined_controls_keep_gain_sharing_and_independent_track_controls() {
        let mut network = Network::default();
        Arc::get_mut(&mut network.params).unwrap().mode =
            EnumParam::new("Mix mode", MixMode::SpectralSpace);
        let network = Arc::new(network);
        let registration = network.register();
        let params = Arc::new(AutomixerParams::new(network.params.clone(), None, 0));
        let mut editor = AutomixerView::new(params, network, None);
        editor.setup_open = true;
        let ids: Vec<_> = editor.controls().into_iter().map(|(id, _)| id).collect();
        for id in [
            Control::Amount,
            Control::VoiceCap,
            Control::OtherCap,
            Control::Voices,
            Control::Assist,
            Control::SpectralAmount,
            Control::SpectralCut,
            Control::Weight(0),
            Control::SpectralWeight(0),
            Control::SpectralBlend(0),
        ] {
            assert!(ids.contains(&id));
        }
        editor.spectral_setup = true;
        let ids: Vec<_> = editor.controls().into_iter().map(|(id, _)| id).collect();
        assert!(ids.contains(&Control::Low) && ids.contains(&Control::High));
        assert!(ids.contains(&Control::Amount) && ids.contains(&Control::VoiceCap));
        let (_, rect) = editor.visible()[0];
        let layout = StripLayout::new(rect, true);
        assert!(layout.spectral_meters().3 >= 80.0);
        assert!(layout.weight.1 + layout.weight.3 < layout.spectral_weight.1);
        assert!(layout.spectral_weight.1 + layout.spectral_weight.3 < layout.eq_blend.1);
        assert!(layout.eq_blend.1 + layout.eq_blend.3 < STRIP_BOTTOM);
        drop(registration);
    }

    #[test]
    fn long_names_use_two_lines() {
        let lines = name_lines("Choir Left Overhead");
        assert_eq!(lines[0], "Choir Left");
        assert_eq!(lines[1], "Overhead");
        let short = name_lines("ALICE");
        assert_eq!(short[0], "ALICE");
        assert!(short[1].is_empty());
    }
}

#[cfg(all(test, target_os = "macos"))]
#[path = "ui_preview.rs"]
mod preview;
