//! Graph input → output sampling, LUT lookup, and GR alignment tests.

#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::super::constants::{COMP_LUT_GRANULARITY, COMP_LUT_MIN_DB, COMP_LUT_SIZE};
    use super::super::core_math::db_to_linear;
    use super::super::graph::CompressionGraph;
    use super::super::test_fixtures::{
        boost_at_first_interior_graph, boost_at_middle_interior_graph, below_floor_unity_graph,
        compression_at_first_interior, cut_then_boost_graph, gr_at, run_peak_sine_until_settled,
        setup_expansion_chain, uniform_offset_graph,
    };
    use super::super::{ChainParams, ProcessingChain};
    use super::super::envelope::EnvelopeParams;

    use crate::graph_store::GraphSnapshot;

    const SAMPLE_TOL: f64 = 0.15;
    const LUT_TOL: f64 = 0.25;
    const GR_TOL: f64 = 0.35;

    fn lut_from_graph(graph: CompressionGraph) -> super::super::compression_lut::CompressionLUT {
        GraphSnapshot::from_graph(graph).lut
    }

    fn sweep_inputs(min_in: f64, max_in: f64, step: f64) -> Vec<f64> {
        let mut v = Vec::new();
        let mut x = min_in;
        while x <= max_in + 1e-9 {
            v.push(x);
            x += step;
        }
        v
    }

    #[test]
    fn below_graph_floor_is_unity_not_first_segment_slope() {
        let mut g = below_floor_unity_graph();
        for inp in [-120.0, -40.0, -25.0, -22.0, -20.01] {
            let out = g.sample_curve_at_db(inp);
            assert!(
                (out - inp).abs() < 0.01,
                "below floor should be 1:1: inp={inp} out={out}"
            );
        }

        let out_at_knot = g.sample_curve_at_db(-14.0);
        assert!(
            out_at_knot > -14.0 + 1.5,
            "on-graph boost at -14 dB should remain: out={out_at_knot}"
        );
    }

    #[test]
    fn above_graph_ceiling_still_extrapolates() {
        let mut g = CompressionGraph::new();
        g.adjust_interior_output_y(3, 6.0);
        g.adjust_interior_output_y(4, 10.0);
        g.finalize_after_edit_no_sort();
        let above = g.sample_curve_at_db(8.0);
        assert!(
            (above - 8.0).abs() > 0.5,
            "above ceiling should follow curve extrapolation, not 1:1 (in=8 out={above})"
        );
    }

    #[test]
    fn below_floor_detector_gets_zero_gr_not_cut() {
        let g = below_floor_unity_graph();
        let lut = lut_from_graph(g);
        let (gr, skipped) = gr_at(&lut, -22.0, 0.0, 1.0);
        assert!(
            gr.abs() < 0.1,
            "below-floor detector should be unity GR, got {gr} (skipped={skipped})"
        );
        let (gr14, _) = gr_at(&lut, -14.0, 0.0, 1.0);
        assert!(gr14 > 1.5, "on-graph -14 dB should still boost, got {gr14}");
    }

    #[test]
    fn interior_knots_sample_to_their_output_db() {
        for (name, mut g) in [
            ("boost first", boost_at_first_interior_graph()),
            ("boost middle", boost_at_middle_interior_graph()),
            ("cut then boost", cut_then_boost_graph()),
        ] {
            for idx in 1..g.num_points - 1 {
                let in_db = g.get_point_x(idx);
                let out_db = g.get_point_y(idx);
                let sampled = g.sample_curve_at_db(in_db);
                assert!(
                    (sampled - out_db).abs() < SAMPLE_TOL,
                    "graph={name} knot {idx} in={in_db} expected out={out_db} sampled={sampled}"
                );
            }
        }
    }

    #[test]
    fn boost_knot_output_exceeds_input() {
        let g = boost_at_first_interior_graph();
        let in_db = g.get_point_x(1);
        let out_db = g.get_point_y(1);
        assert!(
            out_db > in_db + 0.5,
            "boost knot should sit above unity: in={in_db} out={out_db}"
        );
        let sampled = {
            let mut g = g;
            g.sample_curve_at_db(in_db)
        };
        assert!(
            sampled > in_db + 0.5,
            "sampled boost should exceed input: in={in_db} out={sampled}"
        );
    }

    #[test]
    fn compression_knot_output_below_input() {
        let mut g = compression_at_first_interior(6.0);
        let in_db = g.get_point_x(1);
        let out_db = g.get_point_y(1);
        assert!(out_db < in_db - 0.5);
        let sampled = g.sample_curve_at_db(in_db);
        assert!(sampled < in_db - 0.5, "in={in_db} out={sampled}");
    }

    #[test]
    fn lut_lookup_matches_direct_graph_sample() {
        let cases = [
            boost_at_first_interior_graph(),
            boost_at_middle_interior_graph(),
            cut_then_boost_graph(),
            uniform_offset_graph(3.0),
            compression_at_first_interior(6.0),
        ];
        for mut g in cases {
            let lut = lut_from_graph(g);
            for inp in sweep_inputs(g.min_db, g.max_db, 0.5) {
                let direct = g.sample_curve_at_db(inp);
                let lut_out = lut.lookup(inp);
                assert!(
                    (lut_out - direct).abs() < LUT_TOL,
                    "inp={inp} direct={direct} lut={lut_out}"
                );
            }
        }
    }

    #[test]
    fn gr_matches_output_minus_input_when_engaged() {
        let cases = [
            boost_at_first_interior_graph(),
            boost_at_middle_interior_graph(),
            cut_then_boost_graph(),
            uniform_offset_graph(3.0),
            compression_at_first_interior(6.0),
        ];
        for mut g in cases {
            let lut = lut_from_graph(g);
            for inp in sweep_inputs(g.min_db, g.max_db, 0.5) {
                let out = g.sample_curve_at_db(inp);
                let delta = out - inp;
                let (gr, skipped) = gr_at(&lut, inp, 0.0, 1.0);
                if skipped {
                    assert!(
                        delta.abs() < 0.1,
                        "skipped GR at inp={inp} but curve delta={delta}"
                    );
                    continue;
                }
                assert!(
                    (gr - delta).abs() < GR_TOL,
                    "inp={inp} out={out} delta={delta} gr={gr}"
                );
            }
        }
    }

    #[test]
    fn gr_sign_matches_boost_vs_cut_direction() {
        let (boost_g, cut_g) = (
            boost_at_first_interior_graph(),
            compression_at_first_interior(6.0),
        );
        let (boost_lut, cut_lut) = (
            lut_from_graph(boost_g),
            lut_from_graph(cut_g),
        );

        let boost_in = boost_g.get_point_x(1);
        let (boost_gr, boost_skip) = gr_at(&boost_lut, boost_in, 0.0, 1.0);
        assert!(!boost_skip, "boost knot should engage GR");
        assert!(
            boost_gr > 0.5,
            "boost knot should yield positive GR, got {boost_gr}"
        );

        let cut_in = cut_g.get_point_x(1);
        let (cut_gr, cut_skip) = gr_at(&cut_lut, cut_in, 0.0, 1.0);
        assert!(!cut_skip, "compression knot should engage GR");
        assert!(
            cut_gr < -0.5,
            "compression knot should yield negative GR, got {cut_gr}"
        );
    }

    #[test]
    fn cut_then_boost_regions_have_correct_sign() {
        let mut g = cut_then_boost_graph();
        let lut = lut_from_graph(g);

        let (cut_gr, _) = gr_at(&lut, -15.0, 0.0, 1.0);
        assert!(
            cut_gr < -0.5,
            "cut knot at -15 dB should compress, got gr={cut_gr}"
        );

        let (boost_gr, _) = gr_at(&lut, -12.0, 0.0, 1.0);
        assert!(
            boost_gr > 0.5,
            "boost knot at -12 dB should expand, got gr={boost_gr}"
        );

        // Between knots the curve can cut while approaching the boost section.
        let between_out = g.sample_curve_at_db(-14.0);
        assert!(
            between_out < -14.0,
            "between cut and boost knot at -14 dB still on cut segment: out={between_out}"
        );
    }

    #[test]
    fn uniform_boost_never_cuts_inside_graph_range() {
        let mut g = uniform_offset_graph(3.0);
        let lut = lut_from_graph(g);
        for inp in sweep_inputs(g.min_db, g.max_db, 0.25) {
            let out = g.sample_curve_at_db(inp);
            let (gr, skipped) = gr_at(&lut, inp, 0.0, 1.0);
            if skipped {
                assert_abs_diff_eq!(out, inp, epsilon = 0.1);
                continue;
            }
            assert!(
                out >= inp - 0.15,
                "uniform +3 dB graph should not cut at inp={inp}: out={out} gr={gr}"
            );
            assert!(
                gr >= -0.15,
                "uniform +3 dB graph should not report cut GR at inp={inp}: gr={gr}"
            );
        }
    }

    #[test]
    fn pixel_db_roundtrip_on_graph_bounds() {
        let g = CompressionGraph::new();
        let w = 400.0_f32;
        let h = 400.0_f32;
        for &(in_db, out_db) in &[(-20.0, -20.0), (-16.0, -12.0), (-10.0, -10.0), (0.0, 0.0)] {
            let (px, py) = crate::ui::graph_display::db_to_pixel_with_pad(in_db, out_db, w, h, g.min_db, g.range_db, 0.0);
            let (rin, rout) = crate::ui::graph_display::pixel_to_db_with_pad(px, py, w, h, g.min_db, g.max_db, g.range_db, 0.0);
            assert_abs_diff_eq!(rin, in_db, epsilon = 0.05);
            assert_abs_diff_eq!(rout, out_db, epsilon = 0.05);
        }
    }

    #[test]
    fn lut_covers_full_table_without_nan() {
        let lut = lut_from_graph(cut_then_boost_graph());
        for i in 0..COMP_LUT_SIZE {
            let inp = COMP_LUT_MIN_DB + (i as f64) * COMP_LUT_GRANULARITY;
            let out = lut.lookup(inp);
            assert!(out.is_finite(), "LUT entry at {inp} dB is not finite: {out}");
        }
    }

    #[test]
    fn chain_audio_level_rises_with_boost_graph() {
        let mut chain = setup_expansion_chain(48000.0);
        run_peak_sine_until_settled(&mut chain, -16.0, 8000);

        let target_gr = chain.meter_target_gr_db();
        assert!(
            target_gr > 2.0,
            "boost graph at -16 dBFS should request upward GR, got {target_gr}"
        );

        let amp = db_to_linear(-16.0);
        let out_amp = chain.process_sample(amp, amp, None).0.abs();
        assert!(
            out_amp > amp * 1.1,
            "boost graph should raise level: in={amp} out={out_amp}"
        );
    }

    #[test]
    fn chain_audio_level_falls_with_compression_graph() {
        let mut chain = ProcessingChain::new(48000.0);
        let mut g = compression_at_first_interior(6.0);
        chain.set_test_snapshot(GraphSnapshot::from_graph(g));
        chain.set_envelope_params(EnvelopeParams {
            attack: 50000.0,
            release_ms: 50.0,
            strength: 400.0,
            ..EnvelopeParams::default()
        });
        chain.set_chain_params(ChainParams {
            strength_multiplier: 1.0,
            ..ChainParams::default()
        });
        chain.reset();

        let amp = db_to_linear(-14.0);
        for _ in 0..8000 {
            chain.process_sample(amp, amp, None);
        }

        let target_gr = chain.meter_target_gr_db();
        assert!(
            target_gr < -0.5,
            "compression graph should request downward GR, got {target_gr}"
        );
    }
}
