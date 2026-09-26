//! Rendering-only skins: every appearance uses the same parameter widgets and graph.
use super::{theme, EditorData};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::Color;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use pleasant_ui::preferences::Appearance;
use pleasant_ui::{Draw, BG, GOLD, LINE, MUTED, PANEL, TEAL, TEXT};

/// Pleasant title band. Analog keeps the metal plate, so this offset is zero there.
pub const HEADER_H: f32 = 56.0;

/// Shared module chrome with 24px bypass and a readable 17px title.
pub const MODULE_HEADER_H: f32 = 36.0;
pub const MODULE_HEADER_CTRL: f32 = 24.0;
pub const MODULE_TITLE_SIZE: f32 = 17.0;
pub const MODULE_GAP: f32 = 12.0;
pub const MODULE_MARGIN: f32 = 16.0;

/// Widget-space frames. Screen y is `widget_y + HEADER_H`.
pub const ENV_X: f32 = MODULE_MARGIN;
pub const ENV_Y: f32 = 12.0;
pub const ENV_W: f32 = 321.0;
pub const PLEASANT_EDITOR_WIDTH: u32 = theme::EDITOR_WIDTH + 167;
pub const SIDE_H: f32 = 637.0; // Includes the transfer graph header.
pub const PLEASANT_EDITOR_HEIGHT: u32 = theme::EDITOR_HEIGHT + 217;
pub const HARM_W: f32 = 168.0;
pub const HARM_X: f32 = PLEASANT_EDITOR_WIDTH as f32 - MODULE_MARGIN - HARM_W;
pub const TRANS_X: f32 = ENV_X + ENV_W + MODULE_GAP;
pub const TRANS_W: f32 = HARM_X - MODULE_GAP - TRANS_X;
pub const MID_H: f32 = SIDE_H;
pub const ENVELOPE_FOOTER_H: f32 = 96.0;
pub const DETECTION_BUTTON_W: f32 = 120.0;

pub fn pleasant_knob_size(_x: f32) -> f32 {
    theme::KNOB_SIZE
}

/// Square graph shifted left so axis labels sit inside the wider transfer module.
pub const PLEASANT_GRAPH_X: f32 = TRANS_X + 46.0;
// Reserve the axis gutter, meter handles, and readouts; use the rest for the graph.
pub const PLEASANT_GRAPH_SIZE: f32 = TRANS_W - 46.0 - 12.0 - PLEASANT_METERS_W - 12.0;
pub const PLEASANT_GRAPH_Y: f32 = ENV_Y + 44.0;
/// Shared bottom row for EQ band controls and program detection.
pub const PLEASANT_FOOTER_Y: f32 = PLEASANT_GRAPH_Y + PLEASANT_GRAPH_SIZE - 48.0;
pub const PLEASANT_METER_HEIGHT: f32 = PLEASANT_FOOTER_Y - PLEASANT_GRAPH_Y - 8.0;
pub const PLEASANT_METER_W: f32 = 44.0;
pub const PLEASANT_METER_GAP: f32 = 16.0;
pub const PLEASANT_METER_INSET: f32 = 7.0;
pub const PLEASANT_METERS_W: f32 =
    3.0 * PLEASANT_METER_W + 2.0 * (PLEASANT_METER_INSET + PLEASANT_METER_GAP);

pub fn graph_x(mode: u8) -> f32 {
    if mode == 2 {
        theme::GRAPH_X
    } else {
        PLEASANT_GRAPH_X
    }
}

pub fn graph_y(mode: u8) -> f32 {
    if mode == 2 {
        theme::GRAPH_Y
    } else {
        PLEASANT_GRAPH_Y
    }
}

pub fn graph_size(mode: u8) -> f32 {
    if mode == 2 {
        theme::GRAPH_SIZE
    } else {
        PLEASANT_GRAPH_SIZE
    }
}

pub fn meter_x(mode: u8) -> (f32, f32) {
    if mode == 2 {
        (theme::METER_X, theme::METER_X_RIGHT)
    } else {
        let x = PLEASANT_GRAPH_X + PLEASANT_GRAPH_SIZE + 12.0;
        (x, x + PLEASANT_METER_W + PLEASANT_METER_GAP)
    }
}

pub fn editor_width(mode: u8) -> u32 {
    if mode == 2 {
        theme::EDITOR_WIDTH
    } else {
        PLEASANT_EDITOR_WIDTH
    }
}

pub fn editor_height(mode: u8) -> u32 {
    if mode == 2 {
        theme::EDITOR_HEIGHT
    } else {
        PLEASANT_EDITOR_HEIGHT
    }
}

pub fn content_offset(mode: u8) -> f32 {
    if mode == 2 {
        0.0
    } else {
        HEADER_H
    }
}

pub struct SetAppearance(pub u8);

pub fn encode(mode: Appearance) -> u8 {
    match mode {
        Appearance::Dark => 0,
        Appearance::Light => 1,
        Appearance::Analog => 2,
    }
}
pub fn decode(mode: u8) -> Appearance {
    match mode {
        1 => Appearance::Light,
        2 => Appearance::Analog,
        _ => Appearance::Dark,
    }
}

pub(super) fn painter<'a>(cx: &DrawContext, canvas: &'a mut Canvas, width: f32) -> Draw<'a> {
    let b = cx.bounds();
    let cache = EditorData::font.get(cx);
    let font = cache.get().copied().or_else(|| {
        let font = canvas.add_font_mem(pleasant_ui::FONT_JETBRAINS_MONO).ok()?;
        let _ = cache.set(font);
        Some(font)
    });
    let mut d = Draw::new(
        canvas,
        EditorData::appearance.get(cx) == 1,
        b.w / width,
        b.x,
        b.y,
        font,
    );
    d.alpha_mul = cx.opacity();
    d
}

#[derive(Clone, Copy)]
pub enum Control {
    Knob,
    Slider(super::readout_controls::SliderFill),
    Switch,
    Button,
}

/// Return true when the Pleasant skin handled drawing; Analog falls through to the original art.
pub fn draw_control(
    cx: &mut DrawContext,
    canvas: &mut Canvas,
    param: &ParamWidgetBase,
    control: Control,
    norm: f32,
) -> bool {
    if EditorData::appearance.get(cx) == 2 {
        return false;
    }
    let b = cx.bounds();
    let scale = cx.scale_factor();
    let w = b.w / scale;
    let h = b.h / scale;
    let mut d = painter(cx, canvas, w);
    let value = param.normalized_value_to_string(param.modulated_normalized_value(), true);
    match control {
        Control::Knob => {
            let layout =
                pleasant_ui::draw::KnobLayout::new((0.0, 0.0, w, h)).with_text_sizes(13.0, 15.0);
            d.knob_with_layout(&layout, short_name(param.name()), &value, norm, GOLD, false);
        }
        Control::Slider(fill) => {
            use super::readout_controls::SliderFill;
            d.text(0.0, 14.0, short_name(param.name()), 13.0, MUTED);
            d.text_right(w, 14.0, &value, 14.0, GOLD);
            let (x, y, width, height) = slider_track(w);
            d.rect(x, y, width, height, LINE);
            let pos = x + width * norm.clamp(0.0, 1.0);
            let (start, end) = match fill {
                SliderFill::LeftToRight => (x, pos),
                SliderFill::RightToLeft => (pos, x + width),
                SliderFill::CenterOut => (x + width * 0.5, pos),
            };
            d.rect(start.min(end), y, (end - start).abs(), height, GOLD);
            if matches!(fill, SliderFill::CenterOut) {
                d.rect(x + width * 0.5 - 0.5, y, 1.0, height, MUTED);
            }
            d.rect(pos - 2.0, y - 2.0, 4.0, height + 4.0, GOLD);
        }
        Control::Switch => {
            let value = match value.as_str() {
                "Input-Dependent" => "Input",
                "GR Dependent" => "GR",
                "Rate-of-Change" => "Rate",
                "Transformer" => "Xformer",
                other => other,
            };
            if param.name() == "Harmonic Type" {
                let accent = if norm < 0.5 { GOLD } else { TEAL };
                button(
                    &mut d,
                    (0.0, 0.0, w, h),
                    &value.to_uppercase(),
                    true,
                    accent,
                );
            } else if matches!(param.name(), "Mid/Side" | "Normalize") {
                button(
                    &mut d,
                    (0.0, 0.0, w, h),
                    &param.name().to_uppercase(),
                    norm > 0.0,
                    TEAL,
                );
            } else if param.name() == "Program Release Mode" {
                let mode = match value {
                    "Input" => "INPUT",
                    "GR" => "GR",
                    _ => "GR RATE",
                };
                button(
                    &mut d,
                    (0.0, 0.0, w, h),
                    &format!("PGM DET: {mode}"),
                    true,
                    TEAL,
                );
            } else if param.name() == "Detection Mode" {
                button(&mut d, (0.0, 0.0, w, h), &value.to_uppercase(), true, TEAL);
            } else {
                d.text_centered(w * 0.5, 14.0, short_name(param.name()), 13.0, MUTED);
                button(
                    &mut d,
                    (0.0, 24.0, w, (h - 30.0).max(24.0)),
                    &value.to_uppercase(),
                    norm > 0.0,
                    TEAL,
                );
            }
        }
        Control::Button => {
            let label = match param.name() {
                "L" => "L",
                "SC" => "SC",
                "Program Release Inverse" => "Inverse",
                _ => param.name(),
            };
            button(
                &mut d,
                (0.0, 0.0, w, h),
                &label.to_uppercase(),
                norm > 0.0,
                if label == "Inverse" { GOLD } else { TEAL },
            );
        }
    }
    true
}

fn short_name(name: &str) -> &str {
    match name {
        "Harmonic Mix" => "Mix",
        "Input Offset" => "Offset",
        "Makeup Gain" => "Gain",
        "Program Release Blend" => "Program Blend",
        "Program Release Mode" => "PROGRAM",
        "RMS Normalization" => "NORMALIZE",
        "Mid/Side Mode" => "MID / SIDE",
        "Harmonic Type" => "HARMONICS",
        "Detection Mode" => "DETECTION",
        _ => name,
    }
}

pub const SLIDER_H: f32 = 36.0;

pub fn slider_track(w: f32) -> (f32, f32, f32, f32) {
    (2.0, 24.0, w - 4.0, 6.0)
}

pub fn value_rect(control: Control, w: f32, h: f32) -> (f32, f32, f32, f32) {
    match control {
        Control::Knob => pleasant_ui::draw::KnobLayout::new((0.0, 0.0, w, h))
            .with_text_sizes(13.0, 15.0)
            .value_rect(0.0, w),
        _ => (w - 72.0, 0.0, 72.0, 20.0),
    }
}

pub fn button(d: &mut Draw, rect: (f32, f32, f32, f32), label: &str, on: bool, color: Color) {
    d.button(rect, "", on, color);
    d.text_centered(
        rect.0 + rect.2 * 0.5,
        rect.1 + rect.3 * 0.5 + 4.5,
        label,
        13.0,
        if on { color } else { MUTED },
    );
}

pub struct Background;
impl Background {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self.build(cx, |_| {})
    }
}
impl View for Background {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        if EditorData::appearance.get(cx) == 2 {
            return;
        }
        let width = cx.bounds().w / cx.scale_factor();
        let height = cx.bounds().h / cx.scale_factor();
        let mut d = painter(cx, canvas, width);
        d.rect(0.0, 0.0, width, height, BG);
        d.rect(0.0, 0.0, width, HEADER_H, PANEL);
        d.line(24.0, HEADER_H, width - 24.0, HEADER_H, LINE, 1.0);
        d.text(30.0, 40.0, "COMPOSURE", 24.0, GOLD);
        d.text(220.0, 39.0, "CURVE DYNAMICS", 13.0, MUTED);

        let y = ENV_Y + HEADER_H;
        let modules = [
            (ENV_X, y, ENV_W, SIDE_H, "ENVELOPE", TEAL),
            (TRANS_X, y, TRANS_W, MID_H, "", TEAL),
            (HARM_X, y, HARM_W, SIDE_H, "HARMONICS", GOLD),
        ];
        for (x, y, w, h, title, accent) in modules {
            draw_module(&mut d, x, y, w, h, title, accent);
        }
    }
}

fn draw_module(d: &mut Draw, x: f32, y: f32, w: f32, h: f32, title: &str, _accent: Color) {
    const INSET: f32 = 10.0;
    d.rect(x, y, w, h, PANEL);
    d.outline((x, y, w, h), LINE);
    if title.is_empty() {
        return;
    }
    let size = MODULE_TITLE_SIZE;
    let mid = y + MODULE_HEADER_H * 0.5;
    let title_x = if title == "HARMONICS" {
        x + INSET + MODULE_HEADER_CTRL + 8.0
    } else {
        x + INSET
    };
    d.text(title_x, mid + size * 0.35, title, size, TEXT);
}

pub struct HarmonicsBypass {
    param: ParamWidgetBase,
}

pub fn build_harmonics_bypass(cx: &mut Context) {
    HarmonicsBypass {
        param: ParamWidgetBase::new(cx, EditorData::params, |p| &p.harmonics_on),
    }
    .build(cx, |_| {})
    .display(EditorData::appearance.map(|mode| {
        if *mode == 2 {
            Display::None
        } else {
            Display::Flex
        }
    }))
    .position_type(PositionType::SelfDirected)
    .left(Pixels(HARM_X + 10.0))
    .top(Pixels(ENV_Y + (MODULE_HEADER_H - MODULE_HEADER_CTRL) * 0.5))
    .width(Pixels(MODULE_HEADER_CTRL))
    .height(Pixels(MODULE_HEADER_CTRL));
}

impl View for HarmonicsBypass {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let mut d = painter(cx, canvas, MODULE_HEADER_CTRL);
        d.bypass_button_static(
            (0.0, 0.0, MODULE_HEADER_CTRL, MODULE_HEADER_CTRL),
            self.param.modulated_normalized_value() < 0.5,
            GOLD,
        );
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| {
            if matches!(window_event, WindowEvent::MouseDown(MouseButton::Left)) {
                super::param_widget_ext::toggle_bool_param(cx, &self.param);
                cx.needs_redraw();
                meta.consume();
            }
        });
    }
}

struct AppearanceSelector;

impl View for AppearanceSelector {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let label = match EditorData::appearance.get(cx) {
            1 => "LIGHT",
            2 => "ANALOG",
            _ => "DARK",
        };
        let mut d = painter(cx, canvas, 84.0);
        button(&mut d, (0.0, 0.0, 84.0, 28.0), label, false, MUTED);
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| {
            if matches!(window_event, WindowEvent::MouseDown(MouseButton::Left)) {
                let next = decode(EditorData::appearance.get(cx)).next();
                cx.emit(SetAppearance(encode(next)));
                meta.consume();
            }
        });
    }
}

pub fn build_selector(cx: &mut Context) {
    AppearanceSelector
        .build(cx, |_| {})
        .z_index(10)
        .position_type(PositionType::SelfDirected)
        .left(Stretch(1.0))
        .right(Pixels(24.0))
        .top(EditorData::appearance.map(|mode| {
            Pixels(if *mode == 2 {
                462.0
            } else {
                (HEADER_H - 28.0) * 0.5
            })
        }))
        .width(Pixels(84.0))
        .height(Pixels(28.0));
}
