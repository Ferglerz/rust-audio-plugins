pub mod draw;
pub mod math;
pub mod preferences;
pub mod theme;
pub mod value_edit;

pub use draw::Draw;
pub use preferences::AppearanceStore;
pub use theme::{transform_color, BG, COLORS, GOLD, LINE, MUTED, PANEL, TEAL, TEXT};
pub use value_edit::ValueEdit;

pub const FONT_JETBRAINS_MONO: &[u8] = include_bytes!("../assets/JetBrainsMono-Medium.ttf");
