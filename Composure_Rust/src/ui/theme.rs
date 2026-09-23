//! Layout + colors from JSFX `Interface/Core/` and `04_UI_Controls/03_control_layout.jsfx-inc`.

pub use nih_plug_vizia::vizia::prelude::Color;

// @gfx 1182 504
pub const EDITOR_WIDTH: u32 = 1182;
pub const EDITOR_HEIGHT: u32 = 504;

/// JSFX `render_background_image()` blit scale (`00_ui_orchestration.jsfx-inc`).
pub const JSFX_BG_BLIT_SCALE_X: f32 = 0.29;
/// Pre-exported `assets/ui/BG.png` scale (`3693×1576 * 0.32 → 1182×504`).
pub const BG_ASSET_SCALE_X: f32 = 0.32;
/// Layout X is tuned to the 0.29 blit; Rust BG art is 0.32 → shift right to match PNG.
pub const LAYOUT_X_SCALE: f32 = BG_ASSET_SCALE_X / JSFX_BG_BLIT_SCALE_X;

/// Map JSFX gfx X (or horizontal extent) to Rust BG coordinates.
#[inline]
pub const fn sx(x: f32) -> f32 {
    x * LAYOUT_X_SCALE
}

/// Map BG-asset X back to JSFX gfx space.
#[inline]
pub const fn jsfx_x(mapped: f32) -> f32 {
    mapped / LAYOUT_X_SCALE
}

pub const GRAPH_X: f32 = sx(582.0);
pub const GRAPH_Y: f32 = 56.0;
/// JSFX logical graph span (square in gfx space).
pub const GRAPH_SIZE_JSFX: f32 = 232.0;
/// Square graph extent on the 0.32 BG asset (X was scaled; Y matched for square grid).
pub const GRAPH_SIZE_X: f32 = sx(GRAPH_SIZE_JSFX);
pub const GRAPH_SIZE: f32 = GRAPH_SIZE_X;

pub const METER_X: f32 = sx(582.0 + 232.0 + 11.0);
pub const METER_Y: f32 = GRAPH_Y;
pub const METER_W: f32 = sx(23.0);
/// JSFX `get_meter_h()` — same span as the graph.
pub const METER_H: f32 = GRAPH_SIZE;
pub const METER_REFLECTION_GAP: f32 = 21.0;
pub const METER_REFLECTION_H: f32 = 7.0;
pub const METER_GAP: f32 = sx(4.0);
pub const METER_X_RIGHT: f32 = METER_X + METER_W + METER_GAP;

pub const ENVELOPE_GROUP_X: f32 = -55.0;
pub const ENVELOPE_GROUP_Y: f32 = 40.0;
pub const ENVELOPE_GROUP_W: f32 = sx(260.0);
pub const ENVELOPE_GROUP_H: f32 = GRAPH_SIZE;

pub const INPUT_GROUP_X: f32 = sx(194.0);
pub const INPUT_GROUP_Y: f32 = 36.0;
pub const INPUT_GROUP_W: f32 = sx(368.0);
pub const INPUT_GROUP_H: f32 = GRAPH_SIZE;

pub const HARMONICS_GROUP_X: f32 = sx(891.0);
pub const HARMONICS_GROUP_Y: f32 = 40.0;
pub const HARMONICS_GROUP_W: f32 = sx(166.0);
pub const HARMONICS_GROUP_H: f32 = GRAPH_SIZE;

pub const TOOLBAR_X: f32 = sx(195.0);
pub const TOOLBAR_Y: f32 = 327.0;
pub const TOOLBAR_W: f32 = sx(716.0);
pub const TOOLBAR_H: f32 = 100.0;

pub const KNOB_SIZE: f32 = 109.0;

/// Image toggle switches render at 2× native art size.
pub const SWITCH_SCALE: f32 = 2.0;

/// JSFX layout slot for image toggles (spacing unchanged when scaling art).
pub const SWITCH_SLOT_W: f32 = 83.0;
pub const SWITCH_SLOT_H: f32 = 77.0;
/// Image toggle switches — 2× `switch_*.png` (83×77 → 166×154).
pub const SWITCH_W: f32 = SWITCH_SLOT_W * SWITCH_SCALE;
pub const SWITCH_H: f32 = SWITCH_SLOT_H * SWITCH_SCALE;
/// Visual center from left in source art at 2× (see FerglerUI `draw_image_toggle_button`).
pub const SWITCH_VIS_CENTER_X: f32 = 54.0 * SWITCH_SCALE;
/// Bitmap is shifted left so visual center aligns with control center (`w/2 - vis_center`).
pub const SWITCH_LEFT_OVERHANG: f32 = SWITCH_VIS_CENTER_X - SWITCH_W * 0.5;
/// Widget width: full image + right-side hit padding to match JSFX control right edge.
pub const SWITCH_WIDGET_W: f32 = SWITCH_W + SWITCH_LEFT_OVERHANG;

/// Parallax fader layout slot (`SLIDER_W` / `SLIDER_H` in JSFX) — row gaps stay on these.
pub const PARALLAX_SLOT_W: f32 = sx(123.0);
pub const PARALLAX_SLOT_H: f32 = 27.0;
/// Parallax fader widget size (1× art; readout sliders use `PARALLAX_SLOT_H`).
pub const PARALLAX_SLIDER_W: f32 = PARALLAX_SLOT_W;
pub const PARALLAX_SLIDER_H: f32 = PARALLAX_SLOT_H;

/// FerglerUI `slider_style_label_height` / `draw_centered_label` offset above track.
pub const READOUT_SLIDER_LABEL_H: f32 = 11.0;

pub const GROUP_TITLE_H: f32 = 22.0;
pub const GROUP_PAD: f32 = 10.0;

/// Generic JSFX buttons (L, SC, Brickwall, Inverse) — `BUTTON_H` in FerglerUI layout.
pub const BUTTON_H: f32 = 27.0;

/// Offset to center a larger visual control inside its JSFX layout slot.
#[inline]
pub const fn align_in_slot(slot: f32, visual: f32) -> f32 {
    (slot - visual) * 0.5
}

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
