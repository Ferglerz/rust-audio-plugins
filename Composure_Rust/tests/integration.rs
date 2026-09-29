//! Integration tests for plugin lifecycle and DSP chain.

use composure::dsp::{ChainParams, ProcessingChain};
use composure::params::{ComposureParams, GraphRangeMode};
use nih_plug::prelude::*;

#[test]
fn plugin_default_instantiation() {
    let params = ComposureParams::default();
    let chain = ProcessingChain::new(44100.0);
    assert!(params.attack.value() > 0.0);
    assert_eq!(chain.latency_samples(), 0);
}

#[test]
fn params_serialize_roundtrip() {
    let params = ComposureParams::default();
    let serialized = params.serialize_fields();
    assert!(!serialized.is_empty());

    let restored = ComposureParams::default();
    restored.deserialize_fields(&serialized);

    assert!((restored.attack.value() - params.attack.value()).abs() < 1.0);
    assert!((restored.strength.value() - params.strength.value()).abs() < 1.0);
}

#[test]
fn chain_processes_silence() {
    let mut chain = ProcessingChain::new(48000.0);
    chain.set_chain_params(ChainParams::default());
    let (out_l, out_r) = chain.process_sample(0.0, 0.0, None);
    assert!(out_l.abs() < 1e-6);
    assert!(out_r.abs() < 1e-6);
}

#[test]
fn chain_sidechain_detection_differs_from_main() {
    let mut chain = ProcessingChain::new(48000.0);
    chain.set_chain_params(ChainParams {
        use_sidechain: true,
        ..ChainParams::default()
    });
    chain.reset();
    for _ in 0..2000 {
        chain.process_sample(0.8, 0.8, Some((0.01, 0.01)));
    }
    let with_sc = chain.meter_detector_db();
    chain.reset();
    for _ in 0..2000 {
        chain.process_sample(0.8, 0.8, None);
    }
    let without_sc = chain.meter_detector_db();
    assert!(with_sc < without_sc - 1.0);
}

#[test]
fn graph_range_sync_updates_lut() {
    let params = ComposureParams::default();
    let thr_before = params.graph_store.load().lut.threshold();
    params.graph_store.sync_range_if_needed(40.0);
    let after = params.graph_store.load();
    assert!((after.graph.range_db - 40.0).abs() < 0.01);
    assert!(after.lut.threshold().is_finite());
    let _ = thr_before;
}

#[test]
fn lookahead_latency_matches_2000ms_at_48k() {
    let mut chain = ProcessingChain::new(48000.0);
    chain.set_chain_params(ChainParams {
        lookahead_ms: 2000.0,
        ..ChainParams::default()
    });
    assert_eq!(chain.latency_samples(), 96000);
}

#[test]
fn debug_overlay_shows_runtime_fields() {
    let display = composure::ui::UiDisplay::default();
    display.toggle_debug();
    display.update_block(-14.0, -3.5, -26.0);
    assert_eq!(
        display
            .input_meter_db
            .load(std::sync::atomic::Ordering::Relaxed),
        -26.0
    );
    display.update_debug_telemetry(-14.0, -3.5, -6.0, -18.5, 96000);
    let text = display.debug_overlay_text(-14.0, -3.5);
    assert!(text.contains("det"));
    assert!(text.contains("lut thr"));
    assert!(text.contains("96000"));
}

#[test]
fn graph_range_mode_param_matches_store_after_sync() {
    let params = ComposureParams::default();
    params
        .graph_store
        .sync_range_if_needed(GraphRangeMode::Range40.range_db());
    let graph = params.graph_store.load();
    assert!((graph.graph.range_db - 40.0).abs() < 0.01);
}
