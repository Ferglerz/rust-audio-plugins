//! Compatibility exports for spectrum helpers.
//!
//! New DSP code should import these from `pleasant_dsp` directly.

pub use pleasant_dsp::spectrum::{
    peak_hold, smooth_bins, smooth_bins_f32, spectrum_fall_db, SPECTRUM_FALL_DB_PER_FRAME,
    SPECTRUM_FALL_REF_SAMPLES, SPECTRUM_SMOOTH_WEIGHTS,
};
