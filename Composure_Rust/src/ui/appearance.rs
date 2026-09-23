//! Rendering-only skins: every appearance uses the same parameter widgets and graph.
use super::{theme, EditorData};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg::Color;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;
use pleasant_ui::preferences::Appearance;
use pleasant_ui::{Draw, BG, GOLD, LINE, MUTED, PANEL, TEAL, TEXT};

/// Pleasant title band. Analog keeps the metal plate, so this offset is zero there.
pub const HEADER_H: f32 = 56.0;

/// Module header matches Damian: 24px bypass, 15px title, no hairline.
pub const MODULE_HEADER_H: f32 = 36.0;
pub const MODULE_HEADER_CTRL: f32 = 24.0;
pub const MODULE_TITLE_SIZE: f32 = 15.0;
pub const MODULE_GAP: f32 = 12.0;
pub const MODULE_MARGIN: f32 = 16.0;

/// Widget-space frames. Screen y is `widget_y + HEADER_H`.
pub const ENV_X: f32 = MODULE_MARGIN;
pub const ENV_Y: f32 = 12.0;
pub const ENV_W: f32 = 160.0;
pub const SIDE_H: f32 = 420.0;
pub const HARM_W: f32 = 168.0;
pub const HARM_X: f32 = theme::EDITOR_WIDTH as f32 - MODULE_MARGIN - HARM_W;
pub const DET_X: f32 = ENV_X + ENV_W + MODULE_GAP;
pub const DET_W: f32 = 330.0;
pub const TRANS_X: f32 = DET_X + DET_W + MODULE_GAP;
pub const TRANS_W: f32 = HARM_X - MODULE_GAP - TRANS_X;
pub const MID_H: f32 = 308.0;
pub const BAR_X: f32 = DET_X;
pub const BAR_Y: f32 = ENV_Y + MID_H + MODULE_GAP;
pub const BAR_W: f32 = TRANS_X + TRANS_W - DET_X;
pub const BAR_H: f32 = ENV_Y + SIDE_H - BAR_Y;

/// Square graph shifted left so axis labels sit inside the wider transfer module.
pub const PLEASANT_GRAPH_X: f32 = 614.0;

pub fn graph_x(mode: u8) -> f32 {
    if mode == 2 {
        theme::GRAPH_X
    } else {
        PLEASANT_GRAPH_X
    }
}

pub fn meter_x(mode: u8) -> (f32, f32) {
    if mode == 2 {
        (theme::METER_X, theme::METER_X_RIGHT)
    } else {
        let x = PLEASANT_GRAPH_X + theme::GRAPH_SIZE + 10.0;
        (x, x + theme::METER_W + theme::METER_GAP)
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

fn painter<'a>(cx: &DrawContext, canvas: &'a mut Canvas, width: f32) -> Draw<'a> {
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
            // Square slots pin the arc 8px above the readout. Damian's taller
            // slots leave ~15px. Keep the readout on the widget bottom.
            let arc_h = (h - 3.0).max(64.0);
            d.knob((0.0, 0.0, w, arc_h), param.name(), "", norm, GOLD, false);
            d.text_centered(w * 0.5, h - 8.0, &value, 13.0, GOLD);
        }
        Control::Slider(fill) => {
            use super::readout_controls::SliderFill;
            let n = norm.clamp(0.0, 1.0);
            let (start, end) = match fill {
                SliderFill::LeftToRight => (0.0, n),
                SliderFill::RightToLeft => (n, 1.0),
                SliderFill::CenterOut => (n.min(0.5), n.max(0.5)),
            };
            d.rect(0.0, 0.0, w, h, PANEL);
            d.outline((0.0, 0.0, w, h), LINE);
            d.rect(w * start, h - 3.0, w * (end - start), 3.0, TEAL);
            d.text_centered(w * 0.5, h * 0.5 + 4.0, &value, 13.0, TEXT);
        }
        Control::Switch => {
            let value = match value.as_str() {
                "Input-Dependent" => "Input",
                "GR Dependent" => "GR",
                "Rate-of-Change" => "Rate",
                "Transformer" => "Xformer",
                other => other,
            };
            d.text_centered(w * 0.5, 14.0, short_name(param.name()), 11.0, MUTED);
            d.button(
                (0.0, 24.0, w, (h - 30.0).max(24.0)),
                value,
                norm > 0.0,
                TEAL,
            );
        }
        // The existing child label supplies the compact button's text.
        Control::Button => d.button((0.0, 0.0, w, h), "", norm > 0.0, TEAL),
    }
    true
}

fn short_name(name: &str) -> &str {
    match name {
        "Program Release Mode" => "PROGRAM",
        "RMS Normalization" => "NORMALIZE",
        "Mid/Side Mode" => "MID / SIDE",
        "Harmonic Type" => "HARMONICS",
        "Detection Mode" => "DETECTION",
        _ => name,
    }
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
        let mut d = painter(cx, canvas, theme::EDITOR_WIDTH as f32);
        d.rect(0.0, 0.0, 1182.0, 504.0, BG);
        d.rect(0.0, 0.0, 1182.0, HEADER_H, PANEL);
        d.line(24.0, HEADER_H, 1158.0, HEADER_H, LINE, 1.0);
        d.text(30.0, 40.0, "COMPOSURE", 24.0, GOLD);
        d.text(220.0, 39.0, "CURVE DYNAMICS", 13.0, MUTED);

        let y = ENV_Y + HEADER_H;
        let modules = [
            (ENV_X, y, ENV_W, SIDE_H, "ENVELOPE", TEAL),
            (DET_X, y, DET_W, MID_H, "DETECTOR & TIMING", GOLD),
            (TRANS_X, y, TRANS_W, MID_H, "TRANSFER CURVE", TEAL),
            (HARM_X, y, HARM_W, SIDE_H, "HARMONICS", GOLD),
        ];
        for (x, y, w, h, title, accent) in modules {
            draw_module(&mut d, x, y, w, h, title, accent);
        }
        let toolbar = (BAR_X, BAR_Y + HEADER_H, BAR_W, BAR_H);
        d.rect(toolbar.0, toolbar.1, toolbar.2, toolbar.3, PANEL);
        d.outline(toolbar, LINE);

        // The shared curve remains a dark instrument screen in either Pleasant theme.
        d.light = false;
        let gy = theme::GRAPH_Y + HEADER_H;
        d.rect(
            PLEASANT_GRAPH_X - 4.0,
            gy - 4.0,
            theme::GRAPH_SIZE + 8.0,
            theme::GRAPH_SIZE + 8.0,
            BG,
        );
    }
}

fn draw_module(d: &mut Draw, x: f32, y: f32, w: f32, h: f32, title: &str, accent: Color) {
    const INSET: f32 = 10.0;
    d.rect(x, y, w, h, PANEL);
    d.outline((x, y, w, h), LINE);
    let btn = MODULE_HEADER_CTRL;
    d.bypass_button_static(
        (
            x + INSET,
            y + (MODULE_HEADER_H - btn) * 0.5,
            btn,
            btn,
        ),
        false,
        accent,
    );
    let size = MODULE_TITLE_SIZE;
    let mid = y + MODULE_HEADER_H * 0.5;
    d.text(x + INSET + btn + 8.0, mid + size * 0.35, title, size, TEXT);
}

pub fn build_selector(cx: &mut Context) {
    Button::new(
        cx,
        |cx| {
            let next = decode(EditorData::appearance.get(cx)).next();
            cx.emit(SetAppearance(encode(next)));
        },
        |cx| {
            Label::new(cx, EditorData::appearance.map(|mode| match *mode {
                1 => "LIGHT",
                2 => "ANALOG",
                _ => "DARK",
            }))
        },
    )
    .class("appearance-button")
    .z_index(10)
    .position_type(PositionType::SelfDirected)
    .left(Pixels(1074.0))
    .top(EditorData::appearance.map(|mode| {
        Pixels(if *mode == 2 { 462.0 } else { (HEADER_H - 28.0) * 0.5 })
    }))
    .width(Pixels(84.0))
    .height(Pixels(28.0));
}
