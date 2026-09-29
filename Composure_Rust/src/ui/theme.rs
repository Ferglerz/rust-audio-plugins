//! Control sizes and graph colors.

pub use nih_plug_vizia::vizia::prelude::Color;

pub const KNOB_SIZE: f32 = 109.0;
pub const SWITCH_SLOT_W: f32 = 83.0;

pub const GRAPH_GRID: Color = Color::rgb(64, 64, 64);
pub const GRAPH_UNITY: Color = Color::rgb(90, 90, 90);
pub const GRAPH_CURVE: Color = Color::rgb(204, 153, 51);
pub const GRAPH_INPUT_DOT: Color = Color::rgb(235, 235, 235);
pub const GRAPH_GR_DOT: Color = Color::rgb(51, 204, 102);

/// GR cut (compression) meter/histogram color components.
pub const GR_CUT_R: f32 = 1.0;
pub const GR_CUT_G: f32 = 0.5;
pub const GR_CUT_B: f32 = 0.0;

/// GR boost (expansion) meter/histogram color components.
pub const GR_BOOST_R: f32 = 0.2;
pub const GR_BOOST_G: f32 = 0.5;
pub const GR_BOOST_B: f32 = 1.0;
