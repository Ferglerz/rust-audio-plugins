//! Prepared equalizer processors without host or editor dependencies.

mod band;
#[cfg(feature = "linear-phase")]
mod linear_phase;
#[cfg(feature = "natural-phase")]
mod oversampled;
mod processor;

pub use band::{BandSettings, EqShape};
#[cfg(feature = "linear-phase")]
pub use linear_phase::LinearPhaseEq;
#[cfg(feature = "natural-phase")]
pub use oversampled::OversampledEq;
pub use processor::{BandCoefficients, BandProcessor, StaticEqProcessor};
