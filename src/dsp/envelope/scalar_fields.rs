//! Single source of truth for envelope scalar fields used by param smoothing.
//!
//! Keep field list in sync with `EnvelopeParams` in `mod.rs`.

#[macro_export]
macro_rules! envelope_scalar_fields {
    ($mac:ident;) => {
        $mac!(attack);
        $mac!(attack_curve);
        $mac!(release_ms);
        $mac!(release_curve);
        $mac!(hold_ms);
        $mac!(strength);
        $mac!(prog_release_blend);
        $mac!(input_level_threshold_db);
        $mac!(input_level_threshold_2_db);
        $mac!(gr_blend_threshold_reduction_db);
        $mac!(gr_blend_threshold_reduction_knee_db);
        $mac!(gr_blend_threshold_addition_db);
        $mac!(gr_blend_threshold_addition_knee_db);
        $mac!(rate_change_sensitivity_db);
        $mac!(rate_change_threshold_modifier);
    };
    ($mac:ident; $($args:tt),+) => {
        $mac!($($args),+, attack);
        $mac!($($args),+, attack_curve);
        $mac!($($args),+, release_ms);
        $mac!($($args),+, release_curve);
        $mac!($($args),+, hold_ms);
        $mac!($($args),+, strength);
        $mac!($($args),+, prog_release_blend);
        $mac!($($args),+, input_level_threshold_db);
        $mac!($($args),+, input_level_threshold_2_db);
        $mac!($($args),+, gr_blend_threshold_reduction_db);
        $mac!($($args),+, gr_blend_threshold_reduction_knee_db);
        $mac!($($args),+, gr_blend_threshold_addition_db);
        $mac!($($args),+, gr_blend_threshold_addition_knee_db);
        $mac!($($args),+, rate_change_sensitivity_db);
        $mac!($($args),+, rate_change_threshold_modifier);
    };
}
