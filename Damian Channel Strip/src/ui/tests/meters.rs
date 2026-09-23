use super::*;

#[test]
fn gr_overflow_extends_past_range_and_depth() {
    let (actual, uncapped) = gr_meter_bar_heights(5.0, 12.0, 100.0);
    assert!((actual - 100.0 * 5.0 / 30.0).abs() < 0.001);
    assert!((uncapped - 100.0 * 12.0 / 30.0).abs() < 0.001);
    assert!(uncapped > actual);
    let (full, clipped) = gr_meter_bar_heights(30.0, 48.0, 100.0);
    assert!((full - 100.0).abs() < 0.001);
    assert_eq!(clipped, full);
    assert_eq!(overflow_reduction(6.0, 6.0), None);
    assert_eq!(overflow_reduction(6.0, 9.5), Some(9.5));
    assert_eq!(overflow_reduction(-6.0, -9.5), Some(-9.5));
    assert_eq!(overflow_reduction(-6.0, -2.0), None);
    assert_eq!(uncapped_for(3, &[(1, 2.0), (3, 11.5)]), Some(11.5));
    assert_eq!(uncapped_for(2, &[(1, 2.0)]), None);
    assert_eq!(catch_gr_to_depth(20.0, 10.0), 10.0);
    assert_eq!(catch_gr_to_depth(5.0, 10.0), 5.0);
    assert_eq!(catch_gr_to_depth(10.0, 10.0), 10.0);
    assert_eq!(catch_gr_to_depth(8.0, -1.0), 0.0);
}

#[test]
fn gr_meter_readout_peaks_until_cap_drag() {
    assert!(playback_started(false, true));
    assert!(!playback_started(true, true));
    assert!(!playback_started(true, false));
    assert!(!playback_started(false, false));

    assert_eq!(update_playback_gr_peak(0.0, 4.5), 4.5);
    assert_eq!(update_playback_gr_peak(4.5, 2.0), 4.5);
    assert_eq!(update_playback_gr_peak(4.5, 7.25), 7.25);
    assert_eq!(update_playback_gr_peak(3.0, -1.0), 3.0);

    assert_eq!(gr_meter_readout_db(7.25, 12.0, false), 7.25);
    assert_eq!(gr_meter_readout_db(7.25, 12.0, true), 12.0);
    // Drag ends: peak hold unchanged.
    assert_eq!(gr_meter_readout_db(7.25, 12.0, false), 7.25);
    assert_eq!(format_gr_meter_readout(6.0), "6.0");
    assert_eq!(format_knee_readout(6.0), "Knee: 6.0");
    assert_eq!(format_knee_readout(12.5), "Knee: 12.5");
}

#[test]
fn dyn_meter_inline_when_stem_tall_and_detaches_by_free_space() {
    let gx = GX;
    let gw = GW;
    let tall = Band {
        freq: 1000.0,
        gain: 6.0,
        dynamic: true,
        range: 12.0,
        threshold: -24.0,
        ..Band::default()
    };
    assert!(
        (DYN_METER_MIN_INLINE - 80.0).abs() < 0.01,
        "inline/detached change-point is 80px (was 40px)"
    );
    let tall_g = dyn_meter_geom(&tall, 24.0, gx, gw);
    let tall_stem = (tall_g.range_y - tall_g.node_y).abs();
    assert!(tall_stem >= DYN_METER_MIN_INLINE);
    assert!(tall_g.inline);
    assert!(!tall_g.detached_above);
    assert!((tall_g.y0 - (tall_g.node_y + DYN_METER_PAD)).abs() < 0.5);

    // Stem below the doubled threshold stays detached (was inline at the old 40px).
    let mid = Band {
        freq: 1000.0,
        gain: 6.0,
        dynamic: true,
        range: 5.0, // 5 dB * 8 px/dB = 40px stem
        threshold: -24.0,
        ..Band::default()
    };
    let mid_g = dyn_meter_geom(&mid, 24.0, gx, gw);
    let mid_stem = (mid_g.range_y - mid_g.node_y).abs();
    assert!(mid_stem < DYN_METER_MIN_INLINE);
    assert!(!mid_g.inline);
    assert!(
        (tall_g.y60 - tall_g.range_y).abs() < 0.5,
        "inline -60 end sits on the range-end cap"
    );
    assert!(
        (tall_g.rule_half_w - (DYN_PILL_R + DYN_RANGE_W * DYN_THRESH_OVERHANG)).abs() < 0.01,
        "threshold extends 20% of range width past each pill edge"
    );
    assert!(DYN_METER_TRACK_W > 5.0);
    assert_eq!(
        band_chrome_layout(&tall, 24.0, gx, gw, true),
        NodeChromeLayout::Above
    );
    assert!(threshold_handle_hit(
        &tall,
        tall_g.x,
        tall_g.thresh_y,
        24.0,
        gx,
        gw,
        Some(tall.id)
    ));
    assert!(!threshold_handle_hit(
        &tall,
        tall_g.x,
        tall_g.thresh_y,
        24.0,
        gx,
        gw,
        None
    ));

    let low = Band {
        freq: 1000.0,
        gain: -18.0,
        dynamic: true,
        range: 1.0,
        threshold: -30.0,
        ..Band::default()
    };
    let low_g = dyn_meter_geom(&low, 24.0, gx, gw);
    assert!(!low_g.inline);
    assert!(low_g.detached_above);
    assert!(
        (low_g.node_y - low_g.y0 - DYN_DETACHED_NODE_GAP).abs() < 0.5,
        "detached-above starts one node-gap above the node"
    );
    let center_y = db_y(0.0, 24.0);
    let edge_top = GY + DYN_METER_PAD;
    let expected_far_above = if (low_g.y0 - edge_top) >= (low_g.y0 - center_y).max(0.0) {
        edge_top
    } else {
        center_y
    };
    assert!(
        (low_g.y60 - expected_far_above).abs() < 0.5,
        "detached-above uses the longer of edge vs center span"
    );
    assert!(
        (low_g.y0 - low_g.y60) > 52.0,
        "detached meter is no longer a fixed short stub"
    );
    assert_eq!(
        band_chrome_layout(&low, 24.0, gx, gw, true),
        NodeChromeLayout::Flank
    );
    assert_eq!(
        band_chrome_layout(&low, 24.0, gx, gw, false),
        NodeChromeLayout::Above
    );

    let high = Band {
        freq: 1000.0,
        gain: 18.0,
        dynamic: true,
        range: -1.0,
        threshold: -30.0,
        ..Band::default()
    };
    let high_g = dyn_meter_geom(&high, 24.0, gx, gw);
    assert!(!high_g.inline);
    assert!(!high_g.detached_above);
    assert!(
        (high_g.y0 - high_g.node_y - DYN_DETACHED_NODE_GAP).abs() < 0.5,
        "detached-below starts one node-gap below the node"
    );
    let edge_bot = GRAPH_BOTTOM - DYN_METER_PAD;
    let expected_far_below = if (edge_bot - high_g.y0) >= (center_y - high_g.y0).max(0.0) {
        edge_bot
    } else {
        center_y
    };
    assert!(
        (high_g.y60 - expected_far_below).abs() < 0.5,
        "detached-below uses the longer of edge vs center span"
    );
    assert!(
        (high_g.y60 - high_g.y0) > 52.0,
        "detached-below spans the open room, not a fixed stub"
    );
    assert_ne!(
        band_chrome_layout(&high, 24.0, gx, gw, true),
        NodeChromeLayout::Flank
    );

    // Same-side detach: positive range (stem down) + meter below starts past range end.
    let same_below = Band {
        freq: 1000.0,
        gain: 18.0,
        dynamic: true,
        range: 4.0,
        threshold: -30.0,
        ..Band::default()
    };
    let same_below_g = dyn_meter_geom(&same_below, 24.0, gx, gw);
    assert!(!same_below_g.inline);
    assert!(!same_below_g.detached_above);
    assert!(
        (same_below_g.y0 - (same_below_g.range_y + DYN_METER_PAD)).abs() < 0.5,
        "same-side below meter starts at range end + graph-edge pad"
    );
    assert!(
        same_below_g.y0 > same_below_g.range_y,
        "same-side below meter must not intersect the range pill"
    );
    let edge_bot_same = GRAPH_BOTTOM - DYN_METER_PAD;
    assert!(
        (same_below_g.y60 - edge_bot_same).abs() < 0.5
            || same_below_g.y60 >= db_y(0.0, 24.0) - 0.5,
        "outer end still uses graph-edge / center span"
    );

    // Same-side detach: negative range (stem up) + meter above starts past range top.
    let same_above = Band {
        freq: 1000.0,
        gain: -18.0,
        dynamic: true,
        range: -4.0,
        threshold: -30.0,
        ..Band::default()
    };
    let same_above_g = dyn_meter_geom(&same_above, 24.0, gx, gw);
    assert!(!same_above_g.inline);
    assert!(same_above_g.detached_above);
    assert!(
        (same_above_g.range_y - same_above_g.y0 - DYN_METER_PAD).abs() < 0.5,
        "same-side above meter starts at range end - graph-edge pad"
    );
    assert!(
        same_above_g.y0 < same_above_g.range_y,
        "same-side above meter must not intersect the range pill"
    );

    let level = tall_g.level_y(-60.0);
    assert!((level - tall_g.y60).abs() < 0.5);
    assert!((tall_g.level_y(0.0) - tall_g.y0).abs() < 0.5);
    assert!((tall_g.y_to_threshold(tall_g.y0) - 0.0).abs() < 0.2);
    assert!((tall_g.y_to_threshold(tall_g.y60) + 60.0).abs() < 0.2);
}
