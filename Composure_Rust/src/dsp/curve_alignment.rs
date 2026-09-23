//! Unit tests for transfer-curve ↔ detector ↔ GR alignment.

#[cfg(test)]
mod tests {
    use super::super::constants::{
        COMP_LUT_GRANULARITY, COMP_LUT_MIN_DB, COMP_LUT_SIZE, GRAPH_MIN_DB,
    };
    use super::super::core_math::linear_to_db;
    use super::super::graph::CompressionGraph;
    use super::super::test_fixtures::{
        boost_at_first_interior_graph, cut_then_boost_with_lut, gr_at, run_peak_sine_until_settled,
        setup_expansion_chain,
    };
    use super::super::{ProcessingChain};

    #[test]
    fn left_corner_y_stays_on_graph_floor() {
        let (graph, _) = cut_then_boost_with_lut();
        assert!(
            graph.get_point_y(0) >= GRAPH_MIN_DB - 0.01,
            "left corner Y={} should not sit below graph floor {}",
            graph.get_point_y(0),
            GRAPH_MIN_DB
        );
    }

    #[test]
    fn curve_passes_through_interior_knots() {
        let (mut graph, _) = cut_then_boost_with_lut();
        for idx in 1..graph.num_points - 1 {
            let x = graph.get_point_x(idx);
            let y = graph.get_point_y(idx);
            let sampled = graph.sample_curve_at_db(x);
            assert!(
                (sampled - y).abs() < 0.15,
                "knot {idx} ({x},{y}) sampled={sampled}"
            );
        }
    }

    #[test]
    fn gr_matches_curve_at_knot_abscissa() {
        let (_graph, lut) = cut_then_boost_with_lut();
        let knot_in = -12.0;
        let knot_out = -8.0;
        let expected_gr = knot_out - knot_in;
        let (gr, skipped) = gr_at(&lut, knot_in, 0.0, 1.0);
        assert!(!skipped, "GR should engage at boost knot input level");
        assert!(
            (gr - expected_gr).abs() < 0.35,
            "expected GR≈{expected_gr} at {knot_in} dBFS, got {gr}"
        );

        let cut_in = -15.0;
        let cut_out = -20.0;
        let (gr_cut, skipped_cut) = gr_at(&lut, cut_in, 0.0, 1.0);
        assert!(!skipped_cut);
        assert!(
            (gr_cut - (cut_out - cut_in)).abs() < 0.35,
            "cut knot GR expected {} got {gr_cut}",
            cut_out - cut_in
        );
    }

    #[test]
    fn expansion_drag_on_unity_line_aligns_with_point() {
        let mut graph = boost_at_first_interior_graph();
        let snapshot = crate::graph_store::GraphSnapshot::from_graph(graph);
        graph = snapshot.graph;
        let lut = snapshot.lut;

        let point_in = graph.get_point_x(1);
        let point_out = graph.get_point_y(1);
        assert!((point_in - (-16.0)).abs() < 0.01);
        assert!((point_out - (-12.0)).abs() < 0.01);

        let corner_y = graph.get_point_y(0);
        assert!(
            (corner_y - GRAPH_MIN_DB).abs() < 0.01,
            "expansion drag should unity-snap left corner, got y={corner_y}"
        );

        let expected_gr = point_out - point_in;
        let (gr, skipped) = gr_at(&lut, point_in, 0.0, 1.0);
        assert!(!skipped, "GR should engage at expansion point");
        assert!(
            (gr - expected_gr).abs() < 0.35,
            "expected GR≈{expected_gr} at point, got {gr}"
        );
    }

    #[test]
    fn input_offset_shifts_curve_abscissa_not_audio() {
        let mut graph = CompressionGraph::new();
        graph.adjust_interior_output_y(1, 6.0);
        graph.finalize_after_edit_no_sort();
        let snapshot = crate::graph_store::GraphSnapshot::from_graph(graph);
        let lut = snapshot.lut;

        let offset = 6.0;
        let (gr_off, skipped_off) = gr_at(&lut, -20.0, offset, 1.0);
        let (gr_no, skipped_no) = gr_at(&lut, -14.0, 0.0, 1.0);
        assert_eq!(skipped_off, skipped_no);
        if !skipped_off {
            assert!(
                (gr_off - gr_no).abs() < 0.5,
                "same curve abscissa should yield similar GR: off={gr_off} no={gr_no}"
            );
        }
    }

    #[test]
    fn strength_scales_gr_not_detector() {
        let (_graph, lut) = cut_then_boost_with_lut();
        let (gr_full, _) = gr_at(&lut, -12.0, 0.0, 1.0);
        let (gr_half, _) = gr_at(&lut, -12.0, 0.0, 0.5);
        assert!((gr_half - gr_full * 0.5).abs() < 0.2);
    }

    #[test]
    fn threshold_precedes_first_knot_deviation() {
        let (_graph, lut) = cut_then_boost_with_lut();
        let thr = lut.threshold();
        assert!(thr < 500.0, "bent curve should have finite LUT threshold, got {thr}");
        assert!(
            thr <= -15.0 + COMP_LUT_GRANULARITY,
            "threshold {thr} should not sit above first bent knot (-15 dB)"
        );
    }

    #[test]
    fn chain_peak_sine_tracks_graph_point() {
        let mut chain = setup_expansion_chain(48000.0);
        run_peak_sine_until_settled(&mut chain, -16.0, 8000);

        let curve_in = chain.meter_detector_db();
        assert!(
            (curve_in - (-16.0)).abs() < 0.5,
            "meter curve_input should track -16 dBFS peak, got {curve_in}"
        );

        let target_gr = chain.meter_target_gr_db();
        assert!(
            target_gr > 2.0,
            "expect upward GR at -16 dBFS with expansion point, got {target_gr}"
        );
    }

    #[test]
    fn detector_db_matches_peak_amplitude() {
        let mut chain = ProcessingChain::new(48000.0);
        chain.reset();
        let amp = 0.5;
        for _ in 0..1000 {
            chain.process_sample(amp, amp, None);
        }
        let expected = linear_to_db(amp);
        let curve_in = chain.meter_detector_db();
        assert!(
            (curve_in - expected).abs() < 0.2,
            "peak detector should read {} dBFS, got {curve_in}",
            expected
        );
    }

    #[test]
    fn unity_graph_has_high_threshold() {
        let graph = CompressionGraph::new();
        let snapshot = crate::graph_store::GraphSnapshot::from_graph(graph);
        let lut = snapshot.lut;
        let thr = lut.threshold();
        assert!(thr >= 500.0);
        for i in 0..COMP_LUT_SIZE {
            let inp = COMP_LUT_MIN_DB + (i as f64) * COMP_LUT_GRANULARITY;
            if (-19.0..-0.5).contains(&inp) {
                let out = lut.lookup(inp);
                assert!((out - inp).abs() < 0.25, "unity LUT inp={inp} out={out}");
            }
        }
    }
}
