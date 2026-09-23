mod config;
mod delay;
mod linear_phase;
mod oversampled;
mod path;

pub use config::{Config, ProcessingMode, Resolution, MODES, RESOLUTIONS};
pub use delay::Delay;
#[allow(unused_imports)]
pub use linear_phase::LinearPhase;
#[allow(unused_imports)]
pub use oversampled::Oversampled;
pub use path::EqPath;

#[cfg(test)]
use config::HOP;

#[cfg(test)]
mod tests;
