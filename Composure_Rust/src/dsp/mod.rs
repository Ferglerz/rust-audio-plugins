//! DSP processing chain ported from `09_audio_processing_chain.jsfx-inc`.

pub mod compression_lut;
pub mod constants;
pub mod core_math;
pub mod curve_alignment;
pub mod dsp_utils;
pub mod envelope;
pub mod filters;
pub mod gain_reduction;
pub mod graph;
#[cfg(test)]
mod graph_io;
pub mod graph_snapshot;
pub mod harmonics;
pub mod lookahead;
pub mod param_smooth;
pub mod param_sync;
pub mod rms;
#[cfg(test)]
pub(crate) mod test_fixtures;

pub mod chain;

pub use chain::{ChainParams, ProcessingChain};

pub mod detector_eq;
