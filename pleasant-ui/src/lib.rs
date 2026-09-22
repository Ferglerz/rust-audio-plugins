pub mod draw;
pub mod handles;
pub mod math;
pub mod pointer;
pub mod preferences;
pub mod spectrum;
pub mod theme;
pub mod value_edit;

pub use draw::{ButtonAnim, Draw};
pub use handles::{tag_contains, tag_hit_rect, TagPointer};
pub use pointer::idle_hover;
pub use preferences::AppearanceStore;
pub use theme::{transform_color, BG, COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT};
pub use value_edit::{slider_value_rect, typed_char, ValueEdit};

pub const FONT_JETBRAINS_MONO: &[u8] = include_bytes!("../assets/JetBrainsMono-Medium.ttf");
