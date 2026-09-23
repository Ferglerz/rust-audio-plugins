//! Compression LUT ported from `05_compression_core.jsfx-inc`.

use super::constants::{
    COMP_LUT_GRANULARITY, COMP_LUT_MAX_DB, COMP_LUT_MAX_REDUCTION_DB, COMP_LUT_MIN_DB,
    COMP_LUT_SIZE, COMP_LUT_THRESHOLD_ONSET_DB,
};
use super::graph::{apply_extrapolation_caps, CompressionGraph};

/// Graph range fields cached at LUT build for extrapolation without graph access.
#[derive(Debug, Clone, Copy)]
pub struct ExtrapolationCaps {
    pub min_db: f64,
    pub max_db: f64,
    pub range_db: f64,
}

impl ExtrapolationCaps {
    #[inline]
    pub fn apply(&self, input_db: f64, output_db: f64) -> f64 {
        apply_extrapolation_caps(
            input_db,
            output_db,
            self.min_db,
            self.max_db,
            self.range_db,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CompressionLUT {
    table: [f64; COMP_LUT_SIZE],
    pub threshold_db: f64,
    extrapolation: ExtrapolationCaps,
}

impl Default for CompressionLUT {
    fn default() -> Self {
        Self {
            table: [0.0; COMP_LUT_SIZE],
            threshold_db: 1000.0,
            extrapolation: ExtrapolationCaps {
                min_db: -20.0,
                max_db: 0.0,
                range_db: 20.0,
            },
        }
    }
}

impl CompressionLUT {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build_lut(&mut self, graph: &CompressionGraph) {
        let mut graph = *graph;
        graph.ensure_segments_cached();

        self.extrapolation = ExtrapolationCaps {
            min_db: graph.min_db,
            max_db: graph.max_db,
            range_db: graph.range_db,
        };
        for i in 0..COMP_LUT_SIZE {
            let input_db = COMP_LUT_MIN_DB + (i as f64) * COMP_LUT_GRANULARITY;
            let sampled = super::graph::sample_curve_with_segments(
                input_db,
                graph.segments_ref(),
                graph.min_db,
            );
            let sampled = graph.apply_extrapolation_caps(input_db, sampled);
            self.table[i] = sampled.max(input_db - COMP_LUT_MAX_REDUCTION_DB);
        }
        self.calculate_threshold();
    }

    fn calculate_threshold(&mut self) {
        self.threshold_db = 1000.0;
        for i in 0..COMP_LUT_SIZE {
            let lut_in = COMP_LUT_MIN_DB + (i as f64) * COMP_LUT_GRANULARITY;
            let lut_out = self.table[i];
            if (lut_out - lut_in).abs() > COMP_LUT_THRESHOLD_ONSET_DB {
                self.threshold_db = lut_in;
                break;
            }
        }
    }

    #[inline]
    pub fn threshold(&self) -> f64 {
        self.threshold_db
    }

    #[inline]
    pub fn lookup(&self, input_db: f64) -> f64 {
        let output_db = if input_db < COMP_LUT_MIN_DB {
            self.table[0]
        } else if input_db >= COMP_LUT_MAX_DB {
            self.table[COMP_LUT_SIZE - 1]
        } else {
            let index_float = (input_db - COMP_LUT_MIN_DB) / COMP_LUT_GRANULARITY;
            let index_int = (index_float as usize).min(COMP_LUT_SIZE - 2);
            let index_frac = index_float - index_int as f64;
            let value1 = self.table[index_int];
            let value2 = self.table[index_int + 1];
            value1 + index_frac * (value2 - value1)
        };

        output_db
            .max(self.extrapolation.apply(input_db, output_db))
            .max(input_db - COMP_LUT_MAX_REDUCTION_DB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_graph_lut_is_unity_line() {
        let mut graph = CompressionGraph::new();
        let mut lut = CompressionLUT::new();
        lut.build_lut(&graph);
        assert!(lut.threshold() >= 500.0);
        for i in 0..COMP_LUT_SIZE {
            let inp = COMP_LUT_MIN_DB + (i as f64) * COMP_LUT_GRANULARITY;
            let out = lut.lookup(inp);
            if (-19.0..-0.5).contains(&inp) {
                assert!((out - inp).abs() < 0.2, "inp={inp} out={out}");
            }
        }
    }
}
