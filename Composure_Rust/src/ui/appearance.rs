//! Rendering-only skins: every appearance uses the same parameter widgets and graph.
use super::{theme, EditorData};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::Color;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use pleasant_ui::preferences::Appearance;
use pleasant_ui::{Draw, BG, GOLD, LINE, MUTED, PANEL, TEAL, TEXT};

/// Title band above the plugin modules.
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
pub const ENV_W: f32 = 381.0;
pub const PLEASANT_EDITOR_WIDTH: u32 = 1349;
pub const SIDE_H: f32 = 577.0; // Includes the transfer graph header.
pub const PLEASANT_EDITOR_HEIGHT: u32 = 661;
pub const HARM_W: f32 = 168.0;
pub const HARMONIC_BUTTON_W: f32 = 120.0;
pub const HARMONIC_KNOB_W: f32 = 74.0;
pub const HARMONIC_KNOB_H: f32 = 112.0;
pub const HARM_X: f32 = PLEASANT_EDITOR_WIDTH as f32 - MODULE_MARGIN - HARM_W;
pub const TRANS_X: f32 = ENV_X + ENV_W + MODULE_GAP;
pub const TRANS_W: f32 = HARM_X - MODULE_GAP - TRANS_X;
pub const MID_H: f32 = SIDE_H;
pub const OUTPUT_H: f32 = MODULE_HEADER_H + 8.0 + theme::KNOB_SIZE + 12.0 + 28.0 + 12.0;
pub const GAIN_KNOB_SIZE: f32 = OUTPUT_H - MODULE_HEADER_H - 16.0;
pub const MS_BUTTON_W: f32 = 52.0;
pub const HARM_H: f32 = SIDE_H - MODULE_GAP - OUTPUT_H;
pub const OUTPUT_Y: f32 = ENV_Y + HARM_H + MODULE_GAP;
pub const DETECTION_BUTTON_W: f32 = 100.0;
pub const SC_EQ_BUTTON_W: f32 = 120.0;
pub const ADAPTIVE_GAP: f32 = 16.0;
pub const ADAPTIVE_BUTTON_H: f32 = 28.0;
pub const STRENGTH_KNOB_SIZE: f32 = theme::KNOB_SIZE + ADAPTIVE_GAP + ADAPTIVE_BUTTON_H;
pub const ENVELOPE_GRAPH_X: f32 = ENV_X + 10.0;
pub const ENVELOPE_GRAPH_Y: f32 = ENV_Y + MODULE_HEADER_H + 8.0;
pub const ENVELOPE_GRAPH_W: f32 = ENV_W - 20.0;
pub const ENVELOPE_GRAPH_H: f32 = 240.0;
pub const ENVELOPE_KNOB_Y: f32 = ENVELOPE_GRAPH_Y + ENVELOPE_GRAPH_H + 8.0;

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

pub fn meter_x(_mode: u8) -> (f32, f32) {
    let x = PLEASANT_GRAPH_X + PLEASANT_GRAPH_SIZE + 12.0;
    (x, x + PLEASANT_METER_W + PLEASANT_METER_GAP)
}
pub fn graph_x(_mode: u8) -> f32 {
    PLEASANT_GRAPH_X
}
pub fn graph_y(_mode: u8) -> f32 {
    PLEASANT_GRAPH_Y
}
pub fn graph_size(_mode: u8) -> f32 {
    PLEASANT_GRAPH_SIZE
}
pub fn editor_width(_mode: u8) -> u32 {
    PLEASANT_EDITOR_WIDTH
}
pub fn editor_height(_mode: u8) -> u32 {
    PLEASANT_EDITOR_HEIGHT
}
pub fn content_offset(_mode: u8) -> f32 {
    HEADER_H
}

pub fn read_choice() -> Appearance {
    match Appearance::read("Composure") {
        Appearance::Analog => Appearance::Dark,
        choice => choice,
    }
}

pub fn next_choice(choice: Appearance) -> Appearance {
    match choice {
        Appearance::Dark => Appearance::Light,
        Appearance::Light => Appearance::Auto,
        _ => Appearance::Dark,
    }
}

pub struct SetAppearance(pub u8);
pub struct RefreshAppearance;

pub fn encode(mode: Appearance) -> u8 {
    match mode {
        Appearance::Dark => 0,
        Appearance::Light => 1,
        Appearance::Analog => 0,
        Appearance::Auto => 3,
    }
}
pub fn decode(mode: u8) -> Appearance {
    match mode {
        1 => Appearance::Light,
        3 => Appearance::Auto,
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
    Switch,
    Button,
}

pub(super) fn harmonic_control_inactive(
    params: &crate::params::ComposureParams,
    name: &str,
) -> bool {
    matches!(
        name,
        "Harmonic Type" | "Drive" | "Harmonic Mix" | "Even" | "Odd"
    ) && (!params.harmonics_on.value()
        || (name != "Harmonic Mix" && params.harmonic_mix.value() == 0.0))
}

pub(super) fn program_control_inactive(
    params: &crate::params::ComposureParams,
    name: &str,
) -> bool {
    name == "Program Release Inverse"
        && !params.program_input_enabled()
        && !params.program_gr_enabled()
}

/// Draw a parameter control using the supported interface.
pub fn draw_control(
    cx: &mut DrawContext,
    canvas: &mut Canvas,
    param: &ParamWidgetBase,
    control: Control,
    norm: f32,
) {
    let b = cx.bounds();
    let scale = cx.scale_factor();
    let w = b.w / scale;
    let h = b.h / scale;
    let mut d = painter(cx, canvas, w);
    let value = param.normalized_value_to_string(param.modulated_normalized_value(), true);
    let harmonic_bypassed = harmonic_control_inactive(&EditorData::params.get(cx), param.name());
    let program_bypassed = program_control_inactive(&EditorData::params.get(cx), param.name());
    let inactive = harmonic_bypassed || program_bypassed;
    match control {
        Control::Knob => {
            let layout =
                pleasant_ui::draw::KnobLayout::new((0.0, 0.0, w, h)).with_text_sizes(13.0, 15.0);
            let accent = if param.name() == "Drive" {
                let t = norm.clamp(0.0, 1.0).powf(1.6);
                let burnt = Color::rgb(190, 66, 37);
                Color {
                    r: GOLD.r + (burnt.r - GOLD.r) * t,
                    g: GOLD.g + (burnt.g - GOLD.g) * t,
                    b: GOLD.b + (burnt.b - GOLD.b) * t,
                    a: GOLD.a,
                }
            } else {
                GOLD
            };
            if param.name() == "Input Rate" {
                d.knob_with_layout_bipolar(
                    &layout,
                    short_name(param.name()),
                    &value,
                    norm,
                    if inactive {
                        MUTED
                    } else if norm >= 0.5 {
                        TEAL
                    } else {
                        GOLD
                    },
                    inactive,
                );
            } else {
                d.knob_with_layout(
                    &layout,
                    short_name(param.name()),
                    &value,
                    norm,
                    accent,
                    inactive,
                );
            }
        }
        Control::Switch => {
            let value = match value.as_str() {
                "Transformer" => "Xformer",
                other => other,
            };
            if param.name() == "Harmonic Type" {
                let accent = if harmonic_bypassed {
                    MUTED
                } else if norm < 0.5 {
                    GOLD
                } else {
                    TEAL
                };
                harmonic_button(&mut d, w, h, &value.to_uppercase(), norm >= 0.5, accent);
            } else if param.name() == "Mid/Side" {
                button(
                    &mut d,
                    (0.0, 0.0, w, h),
                    if norm > 0.0 { "M/S" } else { "L/R" },
                    true,
                    if norm > 0.0 { GOLD } else { TEAL },
                );
            } else if param.name() == "Adaptive" {
                button(
                    &mut d,
                    (0.0, 0.0, w, h),
                    &param.name().to_uppercase(),
                    norm > 0.0,
                    TEAL,
                );
            } else if param.name() == "Detection Mode" {
                button(
                    &mut d,
                    (0.0, 0.0, w, h),
                    &value.to_uppercase(),
                    true,
                    if norm > 0.0 { GOLD } else { TEAL },
                );
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
                if inactive {
                    MUTED
                } else if label == "Inverse" {
                    GOLD
                } else {
                    TEAL
                },
            );
        }
    }
}

fn knob_arc(
    d: &mut Draw,
    layout: &pleasant_ui::draw::KnobLayout,
    radius: f32,
    start: f32,
    sweep: f32,
    color: Color,
    width: f32,
) {
    if sweep.abs() < 0.001 {
        return;
    }
    let points: Vec<_> = (0..=48)
        .map(|i| {
            let angle = start + sweep * i as f32 / 48.0;
            (
                layout.cx + radius * angle.cos(),
                layout.cy + radius * angle.sin(),
            )
        })
        .collect();
    d.poly(&points, color, width);
}

pub(super) fn draw_knob_activity(
    cx: &DrawContext,
    canvas: &mut Canvas,
    name: &str,
    display: &super::display::UiDisplay,
) {
    use std::sync::atomic::Ordering;
    let params = EditorData::params.get(cx);
    let activity = match name {
        "Input Rate" if params.input_rate_amount.value() != 0.0 => {
            display.input_rate_activity.load(Ordering::Relaxed)
        }
        "Input Dep" if params.program_input_enabled() => {
            display.input_dependence_activity.load(Ordering::Relaxed)
        }
        "GR Dep" if params.program_gr_enabled() => {
            display.gr_dependence_activity.load(Ordering::Relaxed)
        }
        "Input Rate" | "Input Dep" | "GR Dep" => 0.0,
        _ => return,
    };
    let w = cx.bounds().w / cx.scale_factor();
    let h = cx.bounds().h / cx.scale_factor();
    let layout = pleasant_ui::draw::KnobLayout::new((0.0, 0.0, w, h)).with_text_sizes(13.0, 15.0);
    let mut d = painter(cx, canvas, w);
    let start = 135.0_f32.to_radians();
    let sweep = 270.0_f32.to_radians();
    let color = if activity >= 0.0 { TEAL } else { GOLD };
    knob_arc(
        &mut d,
        &layout,
        layout.radius + 5.0,
        start,
        sweep,
        Color { a: 0.3, ..LINE },
        2.5,
    );
    // One octave (2x speed) fills two thirds of the corresponding half-ring.
    let amount = activity.abs() / (activity.abs() + 0.5);
    let origin = start + sweep * 0.5;
    let extent = sweep * 0.5 * amount * activity.signum();
    knob_arc(
        &mut d,
        &layout,
        layout.radius + 5.0,
        origin,
        extent,
        color,
        3.0,
    );
}

fn short_name(name: &str) -> &str {
    match name {
        "Harmonic Mix" => "Mix",
        "Input Rate" => "INPUT RATE",
        "Input Offset" => "Offset",
        "Makeup Gain" => "Gain",
        "Adaptive" => "ADAPTIVE",
        "Mid/Side Mode" => "MID / SIDE",
        "Harmonic Type" => "HARMONICS",
        "Detection Mode" => "DETECTION",
        _ => name,
    }
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

fn harmonic_button(d: &mut Draw, w: f32, h: f32, label: &str, transformer: bool, color: Color) {
    use nih_plug_vizia::vizia::vg::Paint;

    d.button((0.0, 0.0, w, h), "", true, color);
    let mut paint = Paint::color(color);
    if let Some(font) = d.font {
        paint.set_font(&[font]);
    }
    paint.set_font_size(13.0 * d.s);
    let text_w =
        d.c.measure_text(0.0, 0.0, label, &paint)
            .map(|metrics| metrics.width() / d.s)
            .unwrap_or(label.len() as f32 * 13.0 * 0.6);
    let icon_w = 22.0;
    let gap = 8.0;
    let x = (w - icon_w - gap - text_w) * 0.5;
    let y = h * 0.5 - 12.0;
    if transformer {
        // Opposing windings with the two lines of a magnetic core between them.
        for (coil_x, direction) in [(x + 6.0, 1.0), (x + 16.0, -1.0)] {
            for winding in 0..3 {
                let cy = y + 6.0 + winding as f32 * 4.0;
                let points: Vec<_> = (0..=12)
                    .map(|i| {
                        let angle = std::f32::consts::PI * i as f32 / 12.0;
                        (
                            coil_x + direction * 2.5 * angle.sin(),
                            cy - 2.0 * angle.cos(),
                        )
                    })
                    .collect();
                d.poly(&points, color, 1.4);
            }
            let outer_x = coil_x - direction * 5.0;
            d.poly(
                &[(outer_x, y + 2.0), (coil_x, y + 2.0), (coil_x, y + 4.0)],
                color,
                1.4,
            );
            d.poly(
                &[(coil_x, y + 16.0), (coil_x, y + 18.0), (outer_x, y + 18.0)],
                color,
                1.4,
            );
        }
        d.line(x + 10.0, y + 3.0, x + 10.0, y + 19.0, color, 1.2);
        d.line(x + 12.0, y + 3.0, x + 12.0, y + 19.0, color, 1.2);
    } else {
        // Glass dome, base, pins, and the heater inside the vacuum tube.
        let mut glass = vec![(x + 4.0, y + 19.0), (x + 4.0, y + 8.0)];
        glass.extend((0..=16).map(|i| {
            let angle = std::f32::consts::PI * i as f32 / 16.0;
            (x + 11.0 - 7.0 * angle.cos(), y + 8.0 - 7.0 * angle.sin())
        }));
        glass.extend([(x + 18.0, y + 19.0), (x + 4.0, y + 19.0)]);
        d.poly(&glass, color, 1.4);
        d.line(x + 4.0, y + 21.0, x + 18.0, y + 21.0, color, 1.4);
        for pin in [7.0, 11.0, 15.0] {
            d.line(x + pin, y + 21.0, x + pin, y + 24.0, color, 1.4);
        }
        d.line(x + 7.0, y + 9.0, x + 15.0, y + 9.0, color, 1.4);
        d.poly(
            &[
                (x + 8.0, y + 17.0),
                (x + 8.0, y + 13.0),
                (x + 11.0, y + 16.0),
                (x + 14.0, y + 13.0),
                (x + 14.0, y + 17.0),
            ],
            color,
            1.2,
        );
    }
    d.text(x + icon_w + gap, h * 0.5 + 4.5, label, 13.0, color);
}

pub struct Background;
impl Background {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self.build(cx, |_| {})
    }
}
impl View for Background {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let width = cx.bounds().w / cx.scale_factor();
        let height = cx.bounds().h / cx.scale_factor();
        let mut d = painter(cx, canvas, width);
        d.rect(0.0, 0.0, width, height, BG);
        d.rect(0.0, 0.0, width, HEADER_H, PANEL);
        d.line(24.0, HEADER_H, width - 24.0, HEADER_H, LINE, 1.0);
        d.text(30.0, 40.0, "COMPOSURE", 24.0, GOLD);
        d.text(220.0, 39.0, "CURVE DYNAMICS", 13.0, MUTED);

        let y = ENV_Y + HEADER_H;
        let expansion =
            super::graph_pages::GraphPages::progress.get(cx) * super::graph_pages::EXPANSION_W;
        let modules = [
            (ENV_X, y, ENV_W + expansion, SIDE_H, "ENVELOPE", TEAL),
            (TRANS_X + expansion, y, TRANS_W - expansion, MID_H, "", TEAL),
            (HARM_X, y, HARM_W, HARM_H, "HARMONICS", GOLD),
            (
                HARM_X,
                OUTPUT_Y + HEADER_H,
                HARM_W,
                OUTPUT_H,
                "OUTPUT",
                GOLD,
            ),
        ];
        for (x, y, w, h, title, accent) in modules {
            let params = EditorData::params.get(cx);
            let bypassed = title == "HARMONICS"
                && (!params.harmonics_on.value() || params.harmonic_mix.value() == 0.0);
            draw_module(&mut d, x, y, w, h, title, accent, bypassed);
        }
    }
}

fn draw_module(
    d: &mut Draw,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    title: &str,
    _accent: Color,
    bypassed: bool,
) {
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
    d.text(
        title_x,
        mid + size * 0.35,
        title,
        size,
        if bypassed { MUTED } else { TEXT },
    );
}

pub struct HarmonicsBypass {
    param: ParamWidgetBase,
}

pub fn build_harmonics_bypass(cx: &mut Context) {
    HarmonicsBypass {
        param: ParamWidgetBase::new(cx, EditorData::params, |p| &p.harmonics_on),
    }
    .build(cx, |_| {})
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
        let label = EditorData::appearance_choice.get(cx).label();
        let mut d = painter(cx, canvas, 108.0);
        d.appearance_button((0.0, 0.0, 108.0, 28.0), label);
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|window_event, meta| {
            if matches!(window_event, WindowEvent::MouseDown(MouseButton::Left)) {
                let next = next_choice(EditorData::appearance_choice.get(cx));
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
        .top(Pixels((HEADER_H - 28.0) * 0.5))
        .width(Pixels(108.0))
        .height(Pixels(28.0));
}
