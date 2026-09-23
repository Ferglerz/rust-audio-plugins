//! Shared graph/chain fixtures for DSP tests.

use super::compression_lut::CompressionLUT;
use super::core_math::db_to_linear;
use super::envelope::EnvelopeParams;
use super::gain_reduction::calculate_gain_reduction_from_db;
use super::graph::CompressionGraph;
use super::{ChainParams, ProcessingChain};
use crate::graph_store::GraphSnapshot;

pub fn cut_then_boost_graph() -> CompressionGraph {
    let mut g = CompressionGraph::new();
    g.graph_points_mut()[2] = -15.0;
    g.graph_points_mut()[3] = -20.0;
    g.graph_points_mut()[4] = -12.0;
    g.graph_points_mut()[5] = -8.0;
    g.graph_points_mut()[6] = -5.0;
    g.graph_points_mut()[7] = 0.0;
    g.finalize_after_edit();
    g
}

pub fn cut_then_boost_with_lut() -> (CompressionGraph, CompressionLUT) {
    let snapshot = GraphSnapshot::from_graph(cut_then_boost_graph());
    (snapshot.graph, snapshot.lut)
}

pub fn boost_at_first_interior_graph() -> CompressionGraph {
    let mut g = CompressionGraph::new();
    g.graph_points_mut()[2] = -16.0;
    g.graph_points_mut()[3] = -12.0;
    g.finalize_after_edit_no_sort();
    g
}

pub fn boost_at_middle_interior_graph() -> CompressionGraph {
    let mut g = CompressionGraph::new();
    g.graph_points_mut()[4] = -14.0;
    g.graph_points_mut()[5] = -10.0;
    g.finalize_after_edit();
    g
}

pub fn uniform_offset_graph(offset_db: f64) -> CompressionGraph {
    let mut g = CompressionGraph::new();
    for i in 1..g.num_points - 1 {
        let x = g.get_point_x(i);
        g.graph_points_mut()[i * 2 + 1] = x + offset_db;
    }
    g.finalize_after_edit_no_sort();
    g
}

pub fn compression_at_first_interior(reduction_db: f64) -> CompressionGraph {
    let mut g = CompressionGraph::new();
    g.adjust_interior_output_y(1, reduction_db);
    g.finalize_after_edit_no_sort();
    g
}

pub fn below_floor_unity_graph() -> CompressionGraph {
    let mut g = CompressionGraph::new();
    g.graph_points_mut()[2] = -18.0;
    g.graph_points_mut()[3] = -14.0;
    g.graph_points_mut()[4] = -14.0;
    g.graph_points_mut()[5] = -12.0;
    g.finalize_after_edit();
    g
}

pub fn gr_at(
    lut: &CompressionLUT,
    detector_db: f64,
    offset_db: f64,
    strength: f64,
) -> (f64, bool) {
    let result = calculate_gain_reduction_from_db(lut, detector_db, offset_db, strength);
    (result.target_gr_db, result.skipped)
}

pub fn setup_expansion_chain(srate: f64) -> ProcessingChain {
    let mut chain = ProcessingChain::new(srate);
    chain.set_test_snapshot(GraphSnapshot::from_graph(boost_at_first_interior_graph()));
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
    chain
}

pub fn run_peak_sine_until_settled(chain: &mut ProcessingChain, input_db: f64, samples: usize) {
    let amp = db_to_linear(input_db);
    for _ in 0..samples {
        chain.process_sample(amp, amp, None);
    }
}
