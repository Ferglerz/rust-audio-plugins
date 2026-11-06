pub mod draw;
pub mod editor;
pub mod envelope;
pub mod graph;
pub mod handles;
pub mod math;
pub mod page_slide;
pub mod param;
pub mod pointer;
pub mod preferences;
pub mod readout;
pub mod spectrum;
pub mod theme;
pub mod tempo;
pub mod xy_pad;
pub mod value_edit;

pub use draw::{ButtonAnim, Draw, TRACE_REST_PX};
pub use editor::{frame_timer, load_font_once, start_frame_timer, FRAME_INTERVAL};
pub use handles::{point_in_rect, tag_contains, tag_hit_rect, TagPointer};
pub use pointer::{idle_hover, local_xy};
pub use preferences::AppearanceStore;
pub use theme::{transform_color, with_alpha, BG, COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT};
pub use value_edit::{slider_value_rect, typed_char, ValueEdit};

pub const FONT_JETBRAINS_MONO: &[u8] = include_bytes!("../assets/JetBrainsMono-Medium.ttf");
