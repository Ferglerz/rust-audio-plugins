//! Gain reduction calculation ported from `06_gain_reduction.jsfx-inc`.

use super::compression_lut::CompressionLUT;
use super::constants::{clamp_gr_db, MAX_CURVE_INPUT_DB, MIN_DETECTOR_DB_FLOOR};

pub struct GainReductionResult {
    pub target_gr_db: f64,
    pub target_gr_db_before_strength: f64,
    pub curve_input_db: f64,
    pub skipped: bool,
}

pub fn calculate_gain_reduction_from_db(
    lut: &CompressionLUT,
    detector_level_db: f64,
    input_offset_db: f64,
    strength_multiplier: f64,
) -> GainReductionResult {
    let curve_input_db =
        (detector_level_db + input_offset_db).clamp(MIN_DETECTOR_DB_FLOOR, MAX_CURVE_INPUT_DB);

    let threshold_db = lut.threshold();
    if curve_input_db < threshold_db {
        return GainReductionResult {
            target_gr_db: 0.0,
            target_gr_db_before_strength: 0.0,
            curve_input_db,
            skipped: true,
        };
    }

    let target_output_db = lut.lookup(curve_input_db);
    let mut target_gr_db_before_strength = target_output_db - curve_input_db;
    let mut target_gr_db = target_gr_db_before_strength * strength_multiplier;
    target_gr_db = clamp_gr_db(target_gr_db);
    target_gr_db_before_strength = clamp_gr_db(target_gr_db_before_strength);
    GainReductionResult {
        target_gr_db,
        target_gr_db_before_strength,
        curve_input_db,
        skipped: false,
    }
}

#[cfg(test)]
mod tests {
    use super::super::graph::CompressionGraph;
    use super::*;

    #[test]
    fn below_threshold_skips_gr() {
        let graph = CompressionGraph::new();
        let mut lut = CompressionLUT::new();
        lut.build_lut(&graph);
        let result = calculate_gain_reduction_from_db(&lut, -50.0, 0.0, 1.0);
        assert!(result.skipped);
        assert_eq!(result.target_gr_db, 0.0);
    }

    #[test]
    fn at_threshold_applies_gr() {
        let mut graph = CompressionGraph::new();
        graph.adjust_interior_output_y(1, 6.0);
        graph.finalize_after_edit_no_sort();
        let mut lut = CompressionLUT::new();
        lut.build_lut(&graph);
        let thr = lut.threshold();
        let result = calculate_gain_reduction_from_db(&lut, thr, 0.0, 1.0);
        assert!(!result.skipped);
        assert!(result.target_gr_db.abs() > 0.0);
    }
}
