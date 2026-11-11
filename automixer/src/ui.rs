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
    param,
    pointer::ValuePress,
    readout::{self, EditAction, PressAction},
    value_edit::ValueEdit,
    AppearanceStore, FONT_JETBRAINS_MONO, GOLD, LINE, MUTED, PANEL, TEAL, TEXT,
};
use std::{
    cell::Cell,
    sync::{atomic::Ordering, Arc, OnceLock},
    time::Duration,
};

type Rect = (f32, f32, f32, f32);
const SIZE: (f32, f32) = (1120.0, 760.0);
const THEME: Rect = (1016.0, 21.0, 72.0, 30.0);
const BYPASS: Rect = (922.0, 21.0, 82.0, 30.0);
const LOCAL: Rect = (710.0, 8.0, 194.0, 58.0);
const PREVIOUS: Rect = (952.0, 179.0, 46.0, 28.0);
const NEXT: Rect = (1042.0, 179.0, 46.0, 28.0);
const PER_PAGE: usize = 8;
fn inside(p: (f32, f32), r: Rect) -> bool {
    p.0 >= r.0 && p.0 <= r.0 + r.2 && p.1 >= r.1 && p.1 <= r.1 + r.3
}
fn prefs() -> &'static AppearanceStore {
    static PREFS: OnceLock<AppearanceStore> = OnceLock::new();
    PREFS.get_or_init(|| AppearanceStore::new("Automixer"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Control {
    Amount,
    Cap,
    Voices,
    Threshold,
    Hold,
    Fade,
    Assist,
    Mic,
    Weight(usize),
    MidiChannel(usize),
    MidiCC(usize),
}
const GLOBAL: [Control; 7] = [
    Control::Amount,
    Control::Cap,
    Control::Voices,
    Control::Threshold,
    Control::Hold,
    Control::Fade,
    Control::Assist,
];
#[derive(Clone, Copy)]
enum Target {
    Parameter(Control),
    Name(usize),
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
    drag: Option<(Control, (f32, f32), f32)>,
    press: Option<ValuePress<Control>>,
    edit: Option<ValueEdit<Target>>,
}

impl AutomixerView {
    fn parameter(&self, id: Control) -> Parameter<'_> {
        let p = &self.params.shared;
        match id {
            Control::Amount => Parameter::Float(&p.amount),
            Control::Cap => Parameter::Float(&p.cap),
            Control::Voices => Parameter::Int(&p.voices),
            Control::Threshold => Parameter::Float(&p.threshold),
            Control::Hold => Parameter::Float(&p.hold),
            Control::Fade => Parameter::Float(&p.fade),
            Control::Assist => Parameter::Float(&p.assist),
            Control::Mic => Parameter::Int(&self.params.mic),
            Control::Weight(i) => Parameter::Float(&p.mics[i].weight),
            Control::MidiChannel(i) => Parameter::Int(&p.mics[i].midi_channel),
            Control::MidiCC(i) => Parameter::Int(&p.mics[i].midi_cc),
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
    fn slots(&self) -> Vec<usize> {
        let mut slots: Vec<_> = self
            .network
            .slots
            .iter()
            .enumerate()
            .filter(|(_, slot)| slot.owner.load(Ordering::Acquire) != 0)
            .map(|(i, _)| i)
            .collect();
        slots.sort_by_key(|&i| self.network.slots[i].mic.load(Ordering::Acquire));
        slots
    }
    fn visible(&self) -> Vec<(usize, Rect)> {
        let slots = self.slots();
        let page = self.page.min(slots.len().saturating_sub(1) / PER_PAGE);
        slots
            .into_iter()
            .skip(page * PER_PAGE)
            .take(PER_PAGE)
            .enumerate()
            .map(|(column, slot)| (slot, (32.0 + column as f32 * 133.0, 220.0, 125.0, 326.0)))
            .collect()
    }
    fn name_rect(r: Rect) -> Rect {
        (r.0 + 5.0, r.1 + 8.0, r.2 - 10.0, 24.0)
    }
    fn weight_rect(r: Rect) -> Rect {
        (r.0 + 8.0, 476.0, r.2 - 16.0, 60.0)
    }
    fn controls(&self) -> Vec<(Control, Rect)> {
        let mut result: Vec<_> = GLOBAL
            .into_iter()
            .enumerate()
            .map(|(i, id)| (id, (32.0 + 152.0 * i as f32, 98.0, 144.0, 64.0)))
            .collect();
        result.push((Control::Mic, LOCAL));
        result.push((
            Control::MidiChannel(self.selected),
            (32.0, 592.0, 280.0, 64.0),
        ));
        result.push((Control::MidiCC(self.selected), (328.0, 592.0, 280.0, 64.0)));
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
                    if inside(point, THEME) {
                        prefs().toggle();
                    } else if inside(point, BYPASS) {
                        param::set_normalized_once(
                            cx,
                            self.params.bypass.as_ptr(),
                            (!self.params.bypass.value()) as u8 as f32,
                        );
                    } else if inside(point, PREVIOUS) {
                        self.page = self.page.saturating_sub(1);
                    } else if inside(point, NEXT) {
                        self.page =
                            (self.page + 1).min(self.slots().len().saturating_sub(1) / PER_PAGE);
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
                    self.end_drag(cx);
                    if let Some((slot, r)) = self
                        .visible()
                        .into_iter()
                        .find(|(_, r)| inside(point, Self::name_rect(*r)))
                    {
                        let mic = self.network.slots[slot].mic.load(Ordering::Acquire);
                        let name = self.params.shared.mics[mic]
                            .name
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .clone();
                        self.edit =
                            Some(ValueEdit::new(Target::Name(mic), Self::name_rect(r), name));
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
                WindowEvent::MouseUp(MouseButton::Left)
                | WindowEvent::FocusOut
                | WindowEvent::KeyDown(Code::Escape, _)
                | WindowEvent::MouseDown(MouseButton::Right) => self.end_drag(cx),
                WindowEvent::MouseLeave => self.hover = None,
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

    fn paint(&self, d: &mut Draw) {
        d.hover = if self.drag.is_none() && self.edit.is_none() {
            self.hover
        } else {
            None
        };
        let p = &self.params.shared;
        d.rect(0.0, 0.0, SIZE.0, 72.0, PANEL);
        d.text(32.0, 46.0, "AUTOMIXER", 25.0, GOLD);
        d.text(244.0, 44.0, "LINKED MICROPHONE MIXER", 11.0, MUTED);
        d.button_tinted(BYPASS, "BYPASS", self.params.bypass.value(), GOLD);
        d.appearance_button(THEME, prefs().label());
        d.text(32.0, 88.0, "ALL MICROPHONES", 10.0, MUTED);
        let labels = [
            "ALL AMOUNT",
            "MAX GR",
            "CHORUS > N",
            "THRESH",
            "HOLD",
            "FADE",
            "VOICE ASSIST",
        ];
        for (index, id) in GLOBAL.into_iter().enumerate() {
            let r = (32.0 + 152.0 * index as f32, 98.0, 144.0, 64.0);
            self.paint_control(d, id, r, labels[index]);
        }
        self.paint_control(d, Control::Mic, LOCAL, "THIS MIC");
        let monitor = self.network.monitor(self.network.now_ms());
        let active = monitor.active;
        let chorus = monitor.chorus;
        let registered = self.slots().len();
        d.text(
            32.0,
            199.0,
            &format!("{} INSTANCES   {} ACTIVE", registered, active),
            14.0,
            TEXT,
        );
        let state = if p.amount.value() == 0.0 {
            "AUTOMIX OFF".into()
        } else if monitor.members == 0 {
            "WAITING FOR AUDIO".into()
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
        d.button(PREVIOUS, "<", false, MUTED);
        d.button(NEXT, ">", false, MUTED);
        let page = self.page.min(registered.saturating_sub(1) / PER_PAGE);
        d.text_centered(1020.0, 198.0, &format!("{}", page + 1), 12.0, MUTED);
        let now = self.network.now_ms();
        let conflicts = self.network.conflict_mask();
        for (slot_index, r) in self.visible() {
            let slot = &self.network.slots[slot_index];
            let mic = slot.mic.load(Ordering::Acquire).min(CAPACITY - 1);
            let labels = slot
                .labels
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let local = Some(slot_index) == self.local_slot;
            let fresh = slot.updated_ms.load(Ordering::Relaxed) != 0
                && now.saturating_sub(slot.updated_ms.load(Ordering::Relaxed)) < 750;
            d.rect(r.0, r.1, r.2, r.3, PANEL);
            d.outline(r, if self.selected == mic { GOLD } else { LINE });
            let name = self.display_name(mic, labels.name.as_deref());
            let number = if let Some(input) = &labels.input {
                format!("{:02} · IN {:02}", mic + 1, input.first_channel + 1)
            } else {
                format!("{:02}{}", mic + 1, if local { " / HERE" } else { "" })
            };
            d.text_centered(
                r.0 + r.2 * 0.5,
                244.0,
                &name.chars().take(12).collect::<String>(),
                11.0,
                if local { GOLD } else { TEXT },
            );
            d.text_centered(r.0 + r.2 * 0.5, 263.0, &number, 10.0, MUTED);
            let state = slot.state.load(Ordering::Relaxed);
            let status = if conflicts & (1 << mic) != 0 {
                "ID CONFLICT"
            } else if !fresh || state == network::WAITING {
                "NO AUDIO"
            } else {
                match state {
                    network::BYPASSED => "BYPASSED",
                    network::CHORUS => "CHORUS",
                    network::QUIET => "QUIET",
                    _ => "ACTIVE",
                }
            };
            let color = if fresh && slot.active.load(Ordering::Relaxed) {
                TEAL
            } else {
                MUTED
            };
            d.text_centered(
                r.0 + r.2 * 0.5,
                284.0,
                status,
                9.0,
                if conflicts & (1 << mic) != 0 {
                    GOLD
                } else {
                    color
                },
            );
            let input = if fresh {
                slot.input_db.load(Ordering::Relaxed)
            } else {
                -120.0
            };
            let output = if fresh {
                slot.output_db.load(Ordering::Relaxed)
            } else {
                -120.0
            };
            let gr = if fresh {
                slot.reduction_db.load(Ordering::Relaxed)
            } else {
                0.0
            };
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
                (
                    2,
                    "GR",
                    gr,
                    (gr / p.cap.value().max(1.0)).clamp(0.0, 1.0),
                    GOLD,
                ),
            ] {
                let x = r.0 + 22.0 + column as f32 * 40.0;
                d.text_centered(x, 307.0, label, 9.0, MUTED);
                d.rect(x - 6.0, 318.0, 12.0, 106.0, LINE);
                if column == 2 {
                    d.rect(x - 6.0, 318.0, 12.0, normalized * 106.0, color);
                } else {
                    d.rect(
                        x - 6.0,
                        424.0 - normalized * 106.0,
                        12.0,
                        normalized * 106.0,
                        color,
                    );
                }
                d.text_centered(x, 440.0, &format!("{:.0}", value.max(-99.0)), 10.0, color);
            }
            let voice = if p.assist.value() == 0.0 {
                "VOICE OFF".into()
            } else if !fresh || state == network::WAITING || !slot.vad_ready.load(Ordering::Relaxed)
            {
                "VOICE WAIT".into()
            } else {
                format!("VOICE {:3.0}%", slot.speech.load(Ordering::Relaxed) * 100.0)
            };
            d.text_centered(r.0 + r.2 * 0.5, 463.0, &voice, 9.0, MUTED);
            self.paint_control(d, Control::Weight(mic), Self::weight_rect(r), "WT");
        }
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
                "SELECTED  {:02} / {}",
                self.selected + 1,
                name.chars().take(90).collect::<String>()
            ),
            12.0,
            GOLD,
        );
        self.paint_control(
            d,
            Control::MidiChannel(self.selected),
            (32.0, 592.0, 280.0, 64.0),
            "MIDI CHANNEL",
        );
        self.paint_control(
            d,
            Control::MidiCC(self.selected),
            (328.0, 592.0, 280.0, 64.0),
            "WEIGHT CC",
        );
        d.text(640.0, 608.0, "SELECTED WEIGHT", 10.0, MUTED);
        d.text(
            640.0,
            640.0,
            &format!("{:+.1} dB", selected.effective_weight()),
            22.0,
            GOLD,
        );
        d.text(850.0, 608.0, "CONTROL SOURCE", 10.0, MUTED);
        d.text(
            850.0,
            640.0,
            if selected.midi_value.load(Ordering::Relaxed) >= 0
                && selected.midi_channel.value() != 0
            {
                "MIDI CUE"
            } else {
                "FADER / HOST"
            },
            16.0,
            TEAL,
        );
        d.line(32.0, 687.0, 1088.0, 687.0, LINE, 1.0);
        let status = if self.local_slot.is_none() {
            "CAPACITY REACHED · 64 INSTANCES MAXIMUM".into()
        } else if conflicts != 0 {
            "MIC ID CONFLICT · ASSIGN A UNIQUE THIS MIC NUMBER".into()
        } else {
            format!(
                "{} PROCESSING   /   {} ACTIVE   /   CAP {:.1} dB   /   ZERO AUDIO LATENCY",
                monitor.members,
                active,
                p.cap.value()
            )
        };
        d.text(
            32.0,
            712.0,
            &status,
            11.0,
            if conflicts != 0 { GOLD } else { TEAL },
        );
        d.text(32.0, 742.0, self.tooltip(), 10.0, MUTED);
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        }
    }

    fn paint_control(&self, d: &mut Draw, id: Control, rect: Rect, label: &str) {
        let normalized = self.normalized(id);
        let value = if id == Control::Mic && self.params.mic.value() == 0 {
            format!("Auto {:02}", self.params.mic_index() + 1)
        } else {
            self.parameter(id).text(normalized, true)
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
            .is_some_and(|p| inside(p, Self::value_rect(rect)))
        {
            d.value_underline(Self::value_rect(rect), rect.1 + 18.0, GOLD);
        }
    }
    fn tooltip(&self) -> &'static str {
        if self.edit.is_some() {
            return "Type a value or a microphone name. Enter applies; Escape cancels. Names are saved with the project.";
        }
        let Some(point) = self.hover else {
            return "All instances share this mixer. Select a strip; drag its weight or click its value to type. Double-click a name to set an alias. Clear it to follow the host.";
        };
        if inside(point, BYPASS) {
            return "Bypass this microphone and remove it from sharing and the active count. Other microphones continue processing.";
        }
        if inside(point, THEME) {
            return "Cycle Dark, Light, and Auto appearance.";
        }
        if let Some((id, _)) = self.controls().into_iter().find(|(_, r)| inside(point, *r)) {
            return match id {
                Control::Amount => "All instances: fade automixing in or out. 0% is unity; 100% applies full reduction, including quiet mics during chorus.",
                Control::Cap => "Hard maximum reduction on every microphone, in both sharing and chorus modes. Zero dB keeps every mic at unity.",
                Control::Voices => "More than N active mics fades them to unity. Quiet mics still attenuate to the cap. Zero disables chorus protection.",
                Control::Threshold => "Input RMS activity threshold, before weighting. 3 dB hysteresis and a 20 ms onset debounce prevent flicker.",
                Control::Hold => "Hold activity below threshold, and hold chorus mode after the active count drops. This prevents chatter between voices.",
                Control::Fade => "Fade time for the global Amount and chorus transitions. Quiet microphones remain capped when chorus opens the active mics.",
                Control::Assist => "Optional Silero assistance, as in Damian PSE: voice confidence lowers the activity threshold by up to 6 dB. 0% disables it.",
                Control::Mic => "Auto prefers the hardware input number when audio activates. Enter a number to pin the mic and its MIDI cue mapping.",
                Control::Weight(_) => "Priority, not output gain: -12 to +12 dB. Editing replaces the last MIDI cue. Shift drags finely; double-click resets.",
                Control::MidiChannel(_) => "Selected microphone's MIDI channel, 1-16; 0 disables MIDI weight control. Send cues into any linked instance.",
                Control::MidiCC(_) => "Selected microphone's CC number. Values: 0 = -12 dB, 64 = neutral, 127 = +12 dB. The latest cue replaces its weight.",
            };
        }
        "IN / OUT: RMS dBFS. GR: applied reduction in dB. Voice meter: speech confidence. Linked instances must share a format and host process."
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
            AutomixerView {
                selected: params.mic_index(),
                params: params.clone(),
                network: network.clone(),
                local_slot,
                font: Cell::new(None),
                hover: None,
                page: 0,
                drag: None,
                press: None,
                edit: None,
            }
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

#[cfg(all(test, target_os = "macos"))]
#[path = "ui_preview.rs"]
mod preview;
