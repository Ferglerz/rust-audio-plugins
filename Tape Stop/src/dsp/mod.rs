pub mod engine;
pub mod hermite;
pub mod ring_buffer;
mod trajectory;

pub use engine::{inv_s_curve, s_curve, PreparedTapeSettings, TapeStopEngine};
pub(crate) const DROP_RATE_SCALE: f32 = trajectory::SEMI_DROP as f32;
