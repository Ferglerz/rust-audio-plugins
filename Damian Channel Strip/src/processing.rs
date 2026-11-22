mod config;
mod delay;
mod linear_phase;
mod oversampled;
mod path;

pub use config::{Config, ProcessingMode, Resolution, MODES, RESOLUTIONS};
pub use delay::Delay;
pub use path::EqPath;

#[cfg(test)]
use config::HOP;

#[cfg(test)]
mod tests;
