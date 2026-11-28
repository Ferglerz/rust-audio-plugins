//! Host-independent numerical and DSP primitives.

pub mod axis;
pub mod beam;
pub mod circuit;
pub mod envelope;
pub mod filters;
pub mod instrument;
pub mod interpolation;
pub mod piezo;
pub mod spectrum;
#[cfg(feature = "struck-string")]
pub mod struck_string;
pub mod sympathetic;
pub mod tempo;
pub mod units;

pub mod struck_keys;
