use super::AutomixerView;
use nih_plug::prelude::*;

pub(super) type Rect = (f32, f32, f32, f32);
pub(super) const SIZE: (f32, f32) = (1120.0, 760.0);
pub(super) const THEME: Rect = (1016.0, 21.0, 72.0, 30.0);
pub(super) const LOCAL: Rect = (802.0, 8.0, 194.0, 58.0);
pub(super) const PREVIOUS: Rect = (944.0, 179.0, 34.0, 28.0);
pub(super) const NEXT: Rect = (1054.0, 179.0, 34.0, 28.0);
pub(super) const PER_PAGE: usize = 8;
pub(super) const ROW: f32 = 22.0;
pub(super) const CHANNELS: i32 = 17;
pub(super) const CC_COUNT: i32 = 120;
pub(super) const CC_VISIBLE: i32 = 12;

pub(super) fn channel_label(value: i32) -> String {
    if value == 0 {
        "Off".into()
    } else {
        value.to_string()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Control {
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

pub(super) const GLOBAL: [Control; 8] = [
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
pub(super) enum MenuKind {
    Channel(usize),
    Cc(usize),
}

pub(super) struct OpenMenu {
    pub kind: MenuKind,
    pub scroll: i32,
}

#[derive(Clone, Copy)]
pub(super) enum Target {
    Parameter(Control),
    Name(usize),
}

#[derive(Clone, Copy)]
pub(super) struct BypassButton {
    pub slot: usize,
    pub owner: u64,
    pub rect: Rect,
}

#[derive(Clone, Copy)]
pub(super) struct DrawnStrip {
    pub slot: usize,
    pub mic: usize,
    pub rect: Rect,
}

pub(super) enum Parameter<'a> {
    Float(&'a FloatParam),
    Int(&'a IntParam),
}

macro_rules! parameter_method {
    ($self:expr, $name:ident $(, $arg:expr)*) => { match $self {
        Parameter::Float(p) => p.$name($($arg),*), Parameter::Int(p) => p.$name($($arg),*),
    } };
}

impl Parameter<'_> {
    pub(super) fn ptr(&self) -> ParamPtr {
        parameter_method!(self, as_ptr)
    }
    pub(super) fn normalized(&self) -> f32 {
        parameter_method!(self, unmodulated_normalized_value)
    }
    pub(super) fn default(&self) -> f32 {
        parameter_method!(self, default_normalized_value)
    }
    pub(super) fn text(&self, value: f32, unit: bool) -> String {
        parameter_method!(self, normalized_value_to_string, value, unit)
    }
    pub(super) fn parse(&self, value: &str) -> Option<f32> {
        parameter_method!(self, string_to_normalized_value, value)
    }
    pub(super) fn step(&self, value: f32, positive: bool, fine: bool) -> f32 {
        if positive {
            parameter_method!(self, next_normalized_step, value, fine)
        } else {
            parameter_method!(self, previous_normalized_step, value, fine)
        }
    }
}

impl AutomixerView {
    pub(super) fn page_count(registered: usize) -> usize {
        registered.div_ceil(PER_PAGE).max(1)
    }
    pub(super) fn strip_rect(column: usize) -> Rect {
        (32.0 + column as f32 * 133.0, 220.0, 125.0, 328.0)
    }
    pub(super) fn name_rect(r: Rect) -> Rect {
        (r.0 + 5.0, r.1 + 20.0, r.2 - 10.0, 24.0)
    }
    pub(super) fn weight_rect(r: Rect) -> Rect {
        (r.0 + 8.0, 490.0, r.2 - 16.0, 53.0)
    }
    pub(super) fn group_rect(r: Rect) -> Rect {
        (r.0 + 8.0, 424.0, r.2 - 16.0, 24.0)
    }
    pub(super) fn global_rect(index: usize) -> Rect {
        (32.0 + index as f32 * 133.0, 98.0, 125.0, 64.0)
    }
    pub(super) fn channel_rect() -> Rect {
        (32.0, 592.0, 280.0, 64.0)
    }
    pub(super) fn cc_rect() -> Rect {
        (328.0, 592.0, 280.0, 64.0)
    }
    pub(super) fn value_rect(r: Rect) -> Rect {
        (r.0 + r.2 * 0.43, r.1 + 3.0, r.2 * 0.57 - 6.0, 24.0)
    }
}
