use super::*;

#[test]
fn graph_mapping_and_band_limits_support_full_display_range() {
    for range in SCALES {
        assert_eq!(db_y(range, range), GY);
        assert_eq!(db_y(-range, range), GY + GH);
        assert_eq!(db_y(0.0, range), GY + GH * 0.5);
        for gain in [-24.0_f64, -12.0, 0.0, 12.0, 24.0] {
            if gain.abs() <= range {
                assert!((y_db(db_y(gain, range), range) - gain).abs() < 0.0001);
            }
        }
        assert!((y_db(GY - GH, range) - range).abs() < 0.0001);
        assert!((y_db(GY + GH * 2.0, range) + range).abs() < 0.0001);
    }

    let mut band = Band {
        gain: 72.0,
        ..Band::default()
    };
    band.sanitize();
    assert_eq!(band.gain, 72.0);
    band.gain = -72.0;
    band.sanitize();
    assert_eq!(band.gain, -72.0);
}

#[test]
fn node_chrome_follows_gain_side_of_zero() {
    let gx = GX;
    let gw = GW;
    let node_x = freq_x_at(1000.0, gx, gw);
    let above_y = db_y(6.0, 24.0);
    let below_y = db_y(-6.0, 24.0);
    assert!(node_chrome_above(6.0, None));
    assert!(!node_chrome_above(-6.0, None));
    assert!(node_chrome_above(0.0, None));
    assert!(node_chrome_above(-6.0, Some(4.0)));
    assert!(!node_chrome_above(6.0, Some(-4.0)));
    assert!(!node_chrome_above(-6.0, Some(0.0)));
    let dyn_band = Band {
        gain: -6.0,
        dynamic: true,
        range: 8.0,
        ..Band::default()
    };
    assert!(band_chrome_above(&dyn_band));
    let cut_band = Band {
        gain: 6.0,
        dynamic: true,
        range: -8.0,
        ..Band::default()
    };
    assert!(!band_chrome_above(&cut_band));
    let (solo_up, close_up) = node_chrome_rects(node_x, above_y, NodeChromeLayout::Above, gx, gw);
    let (solo_dn, close_dn) = node_chrome_rects(node_x, below_y, NodeChromeLayout::Below, gx, gw);
    assert!(solo_up.1 + solo_up.3 <= above_y);
    assert!(close_up.1 + close_up.3 <= above_y);
    assert!(solo_dn.1 >= below_y);
    assert!(close_dn.1 >= below_y);
    assert!(close_up.0 > solo_up.0);
    let (solo_flank, close_flank) =
        node_chrome_rects(node_x, above_y, NodeChromeLayout::Flank, gx, gw);
    assert!(solo_flank.0 + solo_flank.2 <= node_x);
    assert!(close_flank.0 >= node_x);
    assert!((solo_flank.1 + solo_flank.3 * 0.5 - above_y).abs() < 1.0);
    assert!((close_flank.1 + close_flank.3 * 0.5 - above_y).abs() < 1.0);

    let gap_mid_x = (solo_up.0 + solo_up.2 + close_up.0) * 0.5;
    let gap_mid_y = solo_up.1 + solo_up.3 * 0.5;
    assert!(node_chrome_gap_hit(
        gap_mid_x,
        gap_mid_y,
        node_x,
        above_y,
        NodeChromeLayout::Above,
        gx,
        gw,
    ));
    assert!(node_chrome_hit(
        gap_mid_x,
        gap_mid_y,
        node_x,
        above_y,
        NodeChromeLayout::Above,
        gx,
        gw,
    )
    .is_none());

    let flank_gap_x = (solo_flank.0 + solo_flank.2 + node_x) * 0.5;
    assert!(node_chrome_gap_hit(
        flank_gap_x,
        above_y,
        node_x,
        above_y,
        NodeChromeLayout::Flank,
        gx,
        gw,
    ));
}

#[test]
fn range_end_and_threshold_hover_labels_are_distinct_hits() {
    let gx = GX;
    let gw = GW;
    let b = Band {
        id: 1,
        dynamic: true,
        gain: 6.0,
        range: 12.0,
        threshold: -24.0,
        ..Band::default()
    };
    let geom = dyn_meter_geom(&b, 24.0, gx, gw);
    assert!(threshold_handle_hit(
        &b,
        geom.x,
        geom.thresh_y,
        24.0,
        gx,
        gw,
        Some(1),
    ));
    assert!(!range_end_hit(
        &b,
        geom.x,
        geom.thresh_y,
        24.0,
        gx,
        gw,
        Some(1),
    ));
    let y_end = db_y(b.gain - b.range, 24.0);
    assert!(range_end_hit(&b, geom.x, y_end, 24.0, gx, gw, Some(1)));
    assert!(!threshold_handle_hit(
        &b,
        geom.x,
        y_end,
        24.0,
        gx,
        gw,
        Some(1),
    ));
}

#[test]
fn y_db_clamps_to_graph_range_not_hardcoded_24() {
    let top = GY;
    let mid = GY + GH * 0.5;
    assert!((y_db(mid, 48.0)).abs() < 0.01);
    assert!((y_db(top, 48.0) - 48.0).abs() < 0.01);
    assert!((y_db(top, 18.0) - 18.0).abs() < 0.01);
}

#[test]
fn solo_shade_covers_sides_when_applicable() {
    let gx = 100.0;
    let gw = 800.0;
    let (bell_l, bell_r) = solo_shade_rects(Shape::Bell, 1000.0, 1.0, gx, gw);
    assert!(bell_l.is_some() && bell_r.is_some());
    let (low_l, low_r) = solo_shade_rects(Shape::LowCut, 1000.0, 0.707, gx, gw);
    assert!(low_l.is_some() && low_r.is_none());
    let (high_l, high_r) = solo_shade_rects(Shape::HighCut, 1000.0, 0.707, gx, gw);
    assert!(high_l.is_none() && high_r.is_some());
    if let Some((x, w)) = bell_l {
        assert!((x - gx).abs() < 0.01);
        assert!(w > 1.0);
        assert!(x + w < gx + gw);
    }
    let gold = GOLD;
    let grey = greyscale_darken(gold, 0.32);
    assert!((grey.r - grey.g).abs() < 1e-5 && (grey.g - grey.b).abs() < 1e-5);
    assert!(grey.r < gold.r && grey.r < gold.g);
    let hit = intersect_rect((100.0, 10.0, 50.0, 20.0), (120.0, 0.0, 80.0, 40.0));
    assert_eq!(hit, Some((120.0, 10.0, 30.0, 20.0)));
}

#[test]
fn test_lift_bin_curve_fit_smoothness() {
    let mut bins = [0.0_f64; 256];
    // Set a peak at bin 128
    bins[128] = 12.0;

    // Curve fit at bin center 128 (t = 128.5 / 256.0)
    let t_center = 128.5 / 256.0;
    let v_center = lift_bin_curve_fit(&bins, t_center);
    assert!((v_center - 12.0).abs() < 1e-6);

    // At intermediate points, curve should be continuous and non-negative
    for step in 0..=100 {
        let t = 0.45 + 0.10 * (step as f64 / 100.0);
        let val = lift_bin_curve_fit(&bins, t);
        assert!(val >= 0.0);
        assert!(val <= 12.01);
    }
}

#[test]
fn hover_cursor_preview_centering_and_deselect_label() {
    for shape in Shape::ALL {
        let upper = shape.uppercase_name();
        assert!(
            !upper.starts_with('+'),
            "Uppercase name must not have '+': {upper}"
        );
        assert_eq!(upper, upper.to_uppercase());
    }
    assert_eq!(hover_preview_label(true, Shape::Bell), "DESELECT");
    assert_eq!(hover_preview_label(false, Shape::Bell), "BELL");
    assert_eq!(hover_preview_label(false, Shape::LowCut), "LOW CUT");
    assert_eq!(hover_preview_text_y(200.0), 200.0);
    assert_eq!(axis_db_text(18), "18");
    assert!((module_header_ctrl_y() + 24.0 * 0.5 - module_header_mid()).abs() < 0.01);
    assert!(module_title_y(16.0) > module_header_mid());
}

#[test]
fn hover_preview_band_curve_and_fading_ranges() {
    let gx = GX;
    let gw = GW;
    let sr = 48000.0;
    let _graph_db = 24.0;

    // Test preview band inference
    let f_mid = x_freq_at(gx + gw * 0.5, gx, gw);
    assert!((f_mid - 632.45).abs() < 1.0);

    // Bell filter preview response & fading spans
    let bell_shape = Shape::Bell;
    let bell_gain = 6.0;
    let bell_q = 1.0;
    let preview_bell = Band {
        id: 0,
        shape: bell_shape,
        freq: 1000.0,
        gain: bell_gain,
        q: bell_q,
        order: 2,
        ..Band::default()
    };
    let coeff_bell = BandCoeffs::make(&preview_bell, sr);
    assert!((coeff_bell.response(1000.0, sr) - 6.0).abs() < 0.01);
    assert!(coeff_bell.response(20.0, sr).abs() < 0.1);
    assert!(coeff_bell.response(20000.0, sr).abs() < 0.1);

    let oct_span_bell = (2.0 / bell_q.clamp(0.15, 18.0)).clamp(0.5, 4.0);
    let f_left = (preview_bell.freq * 2.0_f64.powf(-oct_span_bell)).max(20.0);
    let f_right = (preview_bell.freq * 2.0_f64.powf(oct_span_bell)).min(20000.0);
    assert_eq!(f_left, 250.0);
    assert_eq!(f_right, 4000.0);
    assert!(freq_x_at(f_left, gx, gw) < freq_x_at(preview_bell.freq, gx, gw));
    assert!(freq_x_at(preview_bell.freq, gx, gw) < freq_x_at(f_right, gx, gw));

    // LowCut filter preview response & fading spans
    let preview_lowcut = Band {
        id: 0,
        shape: Shape::LowCut,
        freq: 100.0,
        gain: 0.0,
        q: 0.707,
        order: 2,
        ..Band::default()
    };
    let coeff_lowcut = BandCoeffs::make(&preview_lowcut, sr);
    assert!(coeff_lowcut.response(20.0, sr) < -12.0);
    assert!((coeff_lowcut.response(10000.0, sr)).abs() < 0.05);

    let oct_span_cut = (1.5 / preview_lowcut.q.clamp(0.15, 18.0)).clamp(0.5, 3.0);
    let f_fade_start = (preview_lowcut.freq * 2.0_f64.powf(-oct_span_cut * 0.5)).max(20.0);
    let f_fade_end = (preview_lowcut.freq * 2.0_f64.powf(oct_span_cut)).min(20000.0);
    assert!(f_fade_start < preview_lowcut.freq);
    assert!(f_fade_end > preview_lowcut.freq);
    assert!(freq_x_at(f_fade_start, gx, gw) < freq_x_at(f_fade_end, gx, gw));

    // HighCut filter preview response & fading spans
    let preview_highcut = Band {
        id: 0,
        shape: Shape::HighCut,
        freq: 5000.0,
        gain: 0.0,
        q: 0.707,
        order: 2,
        ..Band::default()
    };
    let coeff_highcut = BandCoeffs::make(&preview_highcut, sr);
    assert!((coeff_highcut.response(100.0, sr)).abs() < 0.05);
    assert!(coeff_highcut.response(20000.0, sr) < -12.0);

    // HighShelf & LowShelf response
    let preview_highshelf = Band {
        id: 0,
        shape: Shape::HighShelf,
        freq: 2000.0,
        gain: 6.0,
        q: 1.0,
        order: 2,
        ..Band::default()
    };
    let coeff_highshelf = BandCoeffs::make(&preview_highshelf, sr);
    assert!((coeff_highshelf.response(100.0, sr)).abs() < 0.1);
    assert!((coeff_highshelf.response(15000.0, sr) - 6.0).abs() < 0.2);

    let preview_lowshelf = Band {
        id: 0,
        shape: Shape::LowShelf,
        freq: 200.0,
        gain: 6.0,
        q: 1.0,
        order: 2,
        ..Band::default()
    };
    let coeff_lowshelf = BandCoeffs::make(&preview_lowshelf, sr);
    assert!((coeff_lowshelf.response(30.0, sr) - 6.0).abs() < 0.2);
    assert!((coeff_lowshelf.response(10000.0, sr)).abs() < 0.1);
}

#[test]
fn shift_drag_raises_q_downward_and_threshold_tracks_drag() {
    let q = q_from_shift_drag(1.0, -40.0);
    assert!(q > 1.0, "drag down raises Q, got {q}");
    let q_up = q_from_shift_drag(1.0, 40.0);
    assert!(q_up < 1.0, "drag up lowers Q, got {q_up}");
    assert!(threshold_from_drag(-24.0, 20.0) > -24.0);
    assert!(threshold_from_drag(-24.0, -20.0) < -24.0);
}

#[test]
fn dynamic_eq_range_drag_and_bipolar_normalization() {
    let graph_db = 18.0;
    let gain = 4.0;
    let y_node = db_y(gain, graph_db);

    let target_atten = -2.0;
    let y_atten = db_y(target_atten, graph_db);
    assert!(y_atten > y_node, "Attenuation drags downward (higher y)");
    let range_atten = snap_dyn_range(gain, y_atten, graph_db);
    assert!((range_atten - 6.0).abs() < 1e-4);

    let target_boost = 10.0;
    let y_boost = db_y(target_boost, graph_db);
    assert!(y_boost < y_node, "Boost drags upward (lower y)");
    let range_boost = snap_dyn_range(gain, y_boost, graph_db);
    assert!((range_boost - (-6.0)).abs() < 1e-4);

    let y_near_zero = db_y(gain - 0.15, graph_db);
    assert_eq!(snap_dyn_range(gain, y_near_zero, graph_db), 0.0);

    let norm_fn = |range: f64| (range + MAX_RANGE_DB) / (2.0 * MAX_RANGE_DB);
    let denorm_fn = |n: f64| -MAX_RANGE_DB + 2.0 * MAX_RANGE_DB * n;
    assert_eq!(norm_fn(-MAX_RANGE_DB), 0.0);
    assert_eq!(norm_fn(0.0), 0.5);
    assert_eq!(norm_fn(MAX_RANGE_DB), 1.0);
    assert_eq!(denorm_fn(0.0), -MAX_RANGE_DB);
    assert_eq!(denorm_fn(0.5), 0.0);
    assert_eq!(denorm_fn(1.0), MAX_RANGE_DB);

    let mut b = Band {
        freq: 1000.0,
        gain: 4.0,
        dynamic: true,
        range: 6.0,
        ..Band::default()
    };
    let gx = GX;
    let gw = GW;
    let x_node = freq_x_at(b.freq, gx, gw);
    let node_y = db_y(b.gain, graph_db);
    let y_range = db_y(b.gain - b.range, graph_db);
    assert!(range_handle_hit(
        &b,
        x_node,
        y_range,
        graph_db,
        gx,
        gw,
        Some(b.id)
    ));
    assert!(!range_handle_hit(
        &b,
        x_node,
        node_y,
        graph_db,
        gx,
        gw,
        Some(b.id)
    ));
    assert!(!range_handle_hit(
        &b,
        x_node + 80.0,
        y_range,
        graph_db,
        gx,
        gw,
        Some(b.id)
    ));
    b.dynamic = false;
    assert!(!range_handle_hit(
        &b,
        x_node + 10.0,
        node_y,
        graph_db,
        gx,
        gw,
        Some(b.id)
    ));
    assert!(!range_handle_hit(
        &b,
        x_node + 10.0,
        node_y,
        graph_db,
        gx,
        gw,
        None
    ));

    b.dynamic = true;
    assert_eq!(capped_reduction(6.0, 9.0), 6.0);
    assert_eq!(capped_reduction(6.0, 2.0), 2.0);
    assert_eq!(capped_reduction(-6.0, -9.0), -6.0);
    assert_eq!(capped_reduction(-6.0, -2.0), -2.0);
    let hud = hud_rect_for_at(&b, graph_db, gx, gw);
    assert_eq!(hud.3, AXIS_STRIP_H);
    for i in 0..3 {
        let value = hud_value_rect_at(&b, graph_db, i, gx, gw);
        assert!(inside(value.0, value.1, hud));
        assert!(inside(value.0 + value.2, value.1 + value.3, hud));
    }
    for i in 1..5 {
        let value = hud_dyn_value_rect_at(&b, graph_db, i, gx, gw);
        let field = hud_dyn_field_rect_at(&b, graph_db, i, gx, gw);
        assert!(inside(value.0, value.1, field));
        assert!(inside(value.0 + value.2, value.1 + value.3, field));
        assert!(inside(field.0, field.1, hud));
    }
    let live = plot_band(&b, &[(b.id, 9.0)]);
    assert!((live.gain - (b.gain - 6.0)).abs() < 1e-9);
    let idle = plot_band(&b, &[]);
    assert_eq!(idle.gain, b.gain);
}

#[test]
fn threshold_drag_wins_over_range_at_overlapping_y() {
    let gx = GX;
    let gw = GW;
    let b = Band {
        freq: 1000.0,
        gain: 6.0,
        dynamic: true,
        range: 12.0,
        threshold: -60.0,
        ..Band::default()
    };
    let geom = dyn_meter_geom(&b, 24.0, gx, gw);
    // Threshold parked at the -60 end, same neighborhood as the range tip.
    assert!(threshold_handle_hit(
        &b,
        geom.x,
        geom.thresh_y,
        24.0,
        gx,
        gw,
        Some(b.id)
    ));
    assert!(range_handle_hit(
        &b,
        geom.x,
        geom.range_y,
        24.0,
        gx,
        gw,
        Some(b.id)
    ));
}

#[test]
fn eq_freq_axis_drops_edges_and_uses_two_point_five_k() {
    assert_eq!(FREQ_AXIS[0], (50.0, "50"));
    assert_eq!(FREQ_AXIS[5], (2500.0, "2.5k"));
    assert_eq!(FREQ_AXIS.last(), Some(&(10000.0, "10k")));
    assert!(FREQ_AXIS.iter().all(|(freq, label)| {
        *freq != 20.0 && *freq != 20000.0 && *label != "0" && *label != "20k" && *label != "2k"
    }));
}

#[test]
fn eq_curve_samples_notch_zero_instead_of_skipping_it() {
    let sr = 48000.0;
    let gx = 0.0;
    let gw = 768.0;
    let x_mid = gw * 200.5 / 419.0;
    let freq = x_freq_at(x_mid, gx, gw);
    let band = Band {
        shape: Shape::Notch,
        freq,
        q: 1.0,
        ..Band::default()
    };
    let coeff = BandCoeffs::make(&band, sr);
    let coarse_min = (0..420)
        .map(|i| {
            let x = gw * i as f32 / 419.0;
            coeff.response(x_freq_at(x, gx, gw).min(sr * 0.49), sr)
        })
        .fold(0.0_f64, f64::min);
    let xs = eq_curve_xs(gx, gw, [freq]);
    let dbs = eq_db_on_xs(&coeff, &xs, gx, gw, sr, sr);
    let sampled_min = dbs.iter().copied().fold(0.0_f64, f64::min);
    assert!(
        sampled_min < -80.0,
        "curve must hit the notch zero, got {sampled_min}"
    );
    assert!(
        coarse_min > sampled_min + 10.0,
        "coarse grid stayed at {coarse_min}, sampled {sampled_min}"
    );
}

#[test]
fn dynamic_range_curve_mirrors_boost_and_cut_at_the_endpoint() {
    for (gain, range) in [(6.0, 12.0), (-6.0, -12.0)] {
        let b = Band {
            freq: 1000.0,
            gain,
            range,
            dynamic: true,
            ..Band::default()
        };
        let original = BandCoeffs::make(&b, 48000.0);
        let endpoint = BandCoeffs::make(&range_band(&b), 48000.0);
        for freq in [100.0, 500.0, 1000.0, 2000.0, 10000.0] {
            assert!(
                (original.response(freq, 48000.0) + endpoint.response(freq, 48000.0)).abs() < 1e-6
            );
        }
        assert_eq!(b.gain, gain);
    }
}

#[test]
fn dynamic_range_curve_supports_zero_gain_and_both_range_directions() {
    for (gain, range) in [
        (0.0, 8.0),
        (0.0, -8.0),
        (6.0, 3.0),
        (-6.0, -3.0),
        (6.0, 0.0),
    ] {
        let b = Band {
            freq: 1000.0,
            gain,
            range,
            dynamic: true,
            ..Band::default()
        };
        let endpoint = range_band(&b);
        let coeff = BandCoeffs::make(&endpoint, 48000.0);
        assert!((coeff.response(b.freq, 48000.0) - (gain - range)).abs() < 1e-6);
        assert_eq!(endpoint.freq, b.freq);
        assert_eq!(endpoint.q, b.q);
    }
}

#[test]
fn node_outer_circle_is_not_a_range_or_threshold_handle() {
    for dynamic in [false, true] {
        let b = Band {
            id: 1,
            freq: 1000.0,
            gain: 6.0,
            range: 12.0,
            dynamic,
            threshold: 0.0,
            ..Band::default()
        };
        let x = freq_x_at(b.freq, GX, GW);
        let y = db_y(b.gain, 24.0);
        for (dx, dy) in [
            (12.0, 0.0),
            (-12.0, 0.0),
            (0.0, 12.0),
            (0.0, -12.0),
            (15.5, 0.0),
        ] {
            assert!(node_handle_hit(&b, x + dx, y + dy, 24.0, GX, GW));
            assert!(
                !range_handle_hit(&b, x + dx, y + dy, 24.0, GX, GW, Some(b.id)),
                "range must not steal the node's outer circle"
            );
            assert!(
                !threshold_handle_hit(&b, x + dx, y + dy, 24.0, GX, GW, Some(b.id)),
                "threshold must not steal the node's outer circle"
            );
        }
    }
}

#[test]
fn disabled_dynamics_has_no_range_drag_target() {
    let b = Band {
        id: 1,
        freq: 1000.0,
        gain: 6.0,
        range: 12.0,
        dynamic: false,
        ..Band::default()
    };
    let x = freq_x_at(b.freq, GX, GW);
    let node_y = db_y(b.gain, 24.0);
    let range_y = db_y(b.gain - b.range, 24.0);
    for selected in [None, Some(b.id)] {
        for y in [node_y, (node_y + range_y) * 0.5, range_y] {
            for dx in [0.0, 10.0, NODE_HIT_R] {
                assert!(!range_handle_hit(&b, x + dx, y, 24.0, GX, GW, selected));
                assert!(!range_end_hit(&b, x + dx, y, 24.0, GX, GW, selected));
            }
        }
    }
}

#[test]
fn range_drag_reaches_both_graph_edges() {
    for scale in SCALES {
        for gain in [-scale, 0.0, scale] {
            for edge in [-scale, scale] {
                let range = snap_dyn_range(gain, db_y(edge, scale), scale);
                let mut band = Band {
                    gain,
                    range,
                    ..Band::default()
                };
                band.sanitize();
                assert!((band.gain - band.range - edge).abs() < 0.0001);
            }
        }
    }
}

#[test]
fn shift_ratio_uses_threshold_meter_travel_without_changing_threshold() {
    let b = Band {
        id: 1,
        dynamic: true,
        gain: -18.0,
        range: 1.0,
        threshold: -42.0,
        ratio: 10.5,
        ..Band::default()
    };
    let control = dynamics_control_band(&b, true);
    let geom = dyn_meter_geom(&control, 24.0, GX, GW);
    assert!((control.threshold + 30.0).abs() < 1e-9);
    assert_eq!(b.threshold, -42.0);
    assert!(threshold_handle_hit(
        &control,
        geom.x,
        geom.thresh_y,
        24.0,
        GX,
        GW,
        Some(b.id)
    ));
    assert!((1.0 + 19.0 * (geom.y_to_threshold(geom.y60) + 60.0) / 60.0 - 1.0).abs() < 1e-9);
    assert!((1.0 + 19.0 * (geom.y_to_threshold(geom.y0) + 60.0) / 60.0 - 20.0).abs() < 1e-9);
    assert_eq!(dynamics_control_band(&b, false), b);
}

#[test]
fn curved_range_grips_are_grabbable_at_high_q() {
    let b = Band {
        id: 1,
        dynamic: true,
        gain: 18.0,
        range: 36.0,
        q: 18.0,
        ..Band::default()
    };
    let graph = (GX, GW, 24.0);
    let rates = (48000.0, 48000.0);
    let lines = range_grip_lines(&b, graph, rates);
    for line in &lines {
        assert!(line.len() >= 3);
        assert!(line.windows(2).all(|pair| pair[1].0 > pair[0].0));
        for &(x, y) in line {
            assert!(range_grip_hit(&b, x, y, graph, rates));
        }
    }
    let off = Band {
        dynamic: false,
        ..b
    };
    assert!(!range_grip_hit(
        &off,
        lines[0][16].0,
        lines[0][16].1,
        graph,
        rates
    ));
}

#[test]
fn band_meter_keeps_fixed_node_edge_padding_for_all_shapes_and_ranges() {
    for shape in [Shape::Bell, Shape::LowShelf, Shape::HighShelf] {
        for q in [0.5, 3.0, 18.0] {
            for gain in [-22.0, -18.0, 0.0, 18.0, 22.0] {
                for range in [-44.0, -12.0, -1.0, 0.0, 1.0, 12.0, 44.0] {
                    let b = Band {
                        shape,
                        q,
                        gain,
                        range,
                        dynamic: true,
                        ..Band::default()
                    };
                    let geom = dyn_meter_geom(&b, 24.0, GX, GW);
                    let gap = (geom.y0 - geom.node_y).abs() - DYN_PILL_R;
                    assert!((gap - DYN_METER_PAD).abs() < 0.01,
                        "node edge gap changed: {shape:?}, Q={q}, gain={gain}, range={range}, gap={gap}");
                }
            }
        }
    }
}

#[test]
fn range_grip_columns_keep_tangent_spacing_on_a_steep_curve() {
    let curve = |x: f32| 3.0 * x;
    let spacing = 7.5;
    let anchors: [(f32, f32); 4] = std::array::from_fn(|col| {
        let x = curve_x_at_arc_distance(&curve, 0.0, (col as f32 - 1.5) * spacing);
        (x, curve(x))
    });
    for pair in anchors.windows(2) {
        assert!(((pair[1].0 - pair[0].0).hypot(pair[1].1 - pair[0].1) - spacing).abs() < 0.01);
        assert!((pair[1].0 - pair[0].0) < spacing * 0.5);
    }

    let bend = |x: f32| 0.4 * x * x;
    let bent_xs: [f32; 4] = std::array::from_fn(|col| {
        curve_x_at_arc_distance(&bend, 0.0, (col as f32 - 1.5) * spacing)
    });
    for pair in bent_xs.windows(2) {
        let step = (pair[1] - pair[0]) / 128.0;
        let length: f32 = (0..128)
            .map(|i| {
                let x = pair[0] + i as f32 * step;
                step.hypot(bend(x + step) - bend(x))
            })
            .sum();
        assert!((length - spacing).abs() < 0.05);
    }
}

#[test]
fn range_grip_endpoints_follow_response_normals() {
    for shape in [Shape::Bell, Shape::LowShelf, Shape::HighShelf] {
        for q in [0.5, 3.0, 18.0] {
            for gain in [-18.0, 18.0] {
                let b = Band {
                    dynamic: true,
                    shape,
                    q,
                    gain,
                    range: gain * 2.0,
                    ..Band::default()
                };
                let coeff = BandCoeffs::make(&range_band(&b), 48000.0);
                let response_y = |x| {
                    db_y(
                        coeff.response(x_freq_at(x, GX, GW).min(48000.0 * 0.49), 48000.0),
                        24.0,
                    )
                };
                let lines = range_grip_lines(&b, (GX, GW, 24.0), (48000.0, 48000.0));
                let shift = db_y(b.gain - b.range, 24.0) - response_y(freq_x_at(b.freq, GX, GW));
                for line in &lines {
                    for &(x, y) in line {
                        let distance = (-400..=400)
                            .map(|i| {
                                let cx = x + i as f32 * 0.02;
                                (x - cx).hypot(y - response_y(cx) - shift)
                            })
                            .fold(f32::INFINITY, f32::min);
                        assert!((distance - 4.0).abs() < 0.12, "grip is not equidistant: {shape:?}, Q={q}, gain={gain}, distance={distance}");
                    }
                }
                for end in [false, true] {
                    let a = lines[0][if end { lines[0].len() - 1 } else { 0 }];
                    let c = lines[1][if end { lines[1].len() - 1 } else { 0 }];
                    let x = (a.0 + c.0) * 0.5;
                    let slope = (response_y(x + 0.05) - response_y(x - 0.05)) / 0.1;
                    let tangent_component =
                        ((c.0 - a.0) + (c.1 - a.1) * slope) / (1.0 + slope * slope).sqrt();
                    assert!(tangent_component.abs() < 0.03, "endpoint misses normal: {shape:?}, Q={q}, gain={gain}, along-tangent={tangent_component}");
                    assert!(((c.0 - a.0).hypot(c.1 - a.1) - 8.0).abs() < 0.01);
                }
            }
        }
    }
}

#[test]
fn band_meter_padding_clears_inline_nodes_and_matches_detached_edges() {
    for (gain, range) in [(22.0, 44.0), (-22.0, -44.0)] {
        let b = Band {
            dynamic: true,
            gain,
            range,
            threshold: -24.0,
            ..Band::default()
        };
        let geom = dyn_meter_geom(&b, 24.0, GX, GW);
        assert!(geom.inline, "large range should contain its meter");
        assert!(((geom.y0 - geom.node_y).abs() - DYN_INLINE_NODE_PAD).abs() < 0.01);
        assert!(((geom.y60 - geom.range_y).abs() - DYN_METER_PAD).abs() < 0.01);
        assert!((geom.y_to_threshold(geom.thresh_y) - b.threshold).abs() < 0.001);
    }
    for gain in [-18.0, 18.0] {
        let b = Band {
            dynamic: true,
            gain,
            range: 1.0,
            ..Band::default()
        };
        let geom = dyn_meter_geom(&b, 24.0, GX, GW);
        assert!(!geom.inline);
        let gap = (geom.y0 - geom.node_y).abs() - DYN_PILL_R;
        assert!((gap - DYN_METER_PAD).abs() < 0.01);
    }
}
