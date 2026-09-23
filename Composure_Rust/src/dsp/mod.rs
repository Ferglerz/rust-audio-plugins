//! DSP processing chain ported from `09_audio_processing_chain.jsfx-inc`.

pub mod compression_lut;
pub mod constants;
pub mod core_math;
pub mod curve_alignment;
#[cfg(test)]
mod graph_io;
#[cfg(test)]
pub(crate) mod test_fixtures;
pub mod dsp_utils;
pub mod envelope;
pub mod filters;
pub mod gain_reduction;
pub mod graph;
pub mod harmonics;
pub mod limiter;
pub mod lookahead;
pub mod param_smooth;
pub mod param_sync;
pub mod rms;

pub mod chain;

pub use chain::{ChainParams, ProcessingChain};
