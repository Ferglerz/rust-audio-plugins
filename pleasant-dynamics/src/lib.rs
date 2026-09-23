//! Host-independent broadband dynamics processors.

mod compressor;
mod pse;
mod wall;

pub use compressor::{CompSettings, VocalComp};
pub use pse::{
    pse_time_to_times, seconds_to_pse_time_pos, Pse, PseSettings, PseTimeConstant,
    PSE_PEAK_RELEASES, PSE_RMS_TIMES, VOICE_THRESH_SHIFT_DB,
};
pub use wall::{tick_wall, WallSettings};
