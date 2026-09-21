//! HISE mixer fader law from `UI_CreateMixerStrips.js`.
//!
//! Fader position 1.0 is the top (+12 dB); 0.0 is the bottom (−72 dB).
//! Breakpoints are piecewise-linear in dB so 0 dB sits at 5/7 ≈ 0.714.

use crate::params::{FADER_MAX_DB, FADER_MIN_DB};

const DB_BREAKPOINTS: [f32; 8] = [
    FADER_MAX_DB,
    6.0,
    0.0,
    -6.0,
    -12.0,
    -24.0,
    -48.0,
    FADER_MIN_DB,
];

/// Convert a fader position (0 = bottom, 1 = top) to dB.
pub fn pos_to_db(pos: f32) -> f32 {
    let inverted = 1.0 - pos.clamp(0.0, 1.0);
    let num_segments = (DB_BREAKPOINTS.len() - 1) as f32;
    let mut segment = (inverted * num_segments).floor() as usize;
    segment = segment.min(DB_BREAKPOINTS.len() - 2);
    let frac = inverted * num_segments - segment as f32;
    let db_low = DB_BREAKPOINTS[segment];
    let db_high = DB_BREAKPOINTS[segment + 1];
    db_low + frac * (db_high - db_low)
}

/// Convert dB to a fader position (0 = bottom, 1 = top).
pub fn db_to_pos(db: f32) -> f32 {
    let db = db.clamp(FADER_MIN_DB, FADER_MAX_DB);
    let mut segment = 0;
    for i in 0..DB_BREAKPOINTS.len() - 1 {
        if db <= DB_BREAKPOINTS[i] && db >= DB_BREAKPOINTS[i + 1] {
            segment = i;
            break;
        }
    }
    let db_low = DB_BREAKPOINTS[segment];
    let db_high = DB_BREAKPOINTS[segment + 1];
    let span = db_high - db_low;
    let frac = if span.abs() < f32::EPSILON {
        0.0
    } else {
        (db - db_low) / span
    };
    let num_segments = (DB_BREAKPOINTS.len() - 1) as f32;
    (1.0 - (segment as f32 + frac) / num_segments).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn almost(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-4, "{a} != {b}");
    }

    #[test]
    fn poles_match_hise() {
        almost(db_to_pos(12.0), 1.0);
        almost(db_to_pos(FADER_MIN_DB), 0.0);
        almost(db_to_pos(0.0), 5.0 / 7.0);
        almost(pos_to_db(1.0), 12.0);
        almost(pos_to_db(0.0), FADER_MIN_DB);
        almost(pos_to_db(5.0 / 7.0), 0.0);
    }

    #[test]
    fn roundtrip_breakpoints() {
        for db in DB_BREAKPOINTS {
            almost(pos_to_db(db_to_pos(db)), db);
        }
    }
}
