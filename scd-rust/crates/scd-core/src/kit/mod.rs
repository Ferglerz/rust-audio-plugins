mod def;
mod generate;

pub use def::{load_stonehouse, stonehouse, ArticulationDef, KitDef, MAX_ARTICULATIONS};
pub use generate::{
    calculate_max_rr_groups, count_kit_piece_strikes, count_kit_strikes,
    generate_kit_piece_strikes, rr_group_size, sample_wav_name, velocity_range, StrikePaths,
};
