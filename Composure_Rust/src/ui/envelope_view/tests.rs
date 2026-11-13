use super::*;

fn test_values() -> EnvelopeValues {
    EnvelopeValues {
        attack_ms: 10.0,
        hold_ms: 0.0,
        release_ms: 100.0,
        attack_curve: 0.0,
        release_curve: 0.0,
    }
}

#[test]
fn attack_snap_is_narrow_and_modifiers_bypass_it() {
    let values = EnvelopeValues {
        attack_ms: 1000.0,
        hold_ms: 100.0,
        release_ms: 1000.0,
        ..test_values()
    };
    let laid = layout(Viewport::from_tuple(GRAPH), values);
    let x = laid.attack_end.0 + 0.5;
    assert_eq!(lookahead_drag_ms(x, &laid, values, false), 1000.0);
    assert!(lookahead_drag_ms(x, &laid, values, true) > 1000.0);
    assert!(lookahead_drag_ms(laid.attack_end.0 + 1.0, &laid, values, false) > 1000.0);
}

#[test]
fn graph_lookahead_drag_round_trips_visually_through_scaled_stages() {
    for values in [
        EnvelopeValues {
            attack_ms: 0.1,
            hold_ms: 500.0,
            release_ms: 1500.0,
            ..test_values()
        },
        EnvelopeValues {
            attack_ms: 500.0,
            hold_ms: 10.0,
            release_ms: 100.0,
            ..test_values()
        },
        test_values(),
    ] {
        let laid = layout(Viewport::from_tuple(GRAPH), values);
        let max_x = lookahead_marker_x(2000.0, &laid, values);
        for i in 0..=100 {
            let x = laid.start.0 + (max_x - laid.start.0) * i as f32 / 100.0;
            let ms = lookahead_graph_ms(x, &laid, values);
            assert!((lookahead_marker_x(ms, &laid, values) - x).abs() < 0.001);
        }
    }
    let values = test_values();
    let laid = layout(Viewport::from_tuple(GRAPH), values);
    assert!(
        lookahead_graph_ms((laid.attack_end.0 + laid.hold_end.0) * 0.5, &laid, values)
            > values.attack_ms
    );
    assert_eq!(lookahead_graph_ms(laid.start.0 - 20.0, &laid, values), 0.0);
}

#[test]
fn lookahead_line_has_a_small_hover_buffer_and_no_numeric_entry() {
    let laid = envelope_layout();
    let x = laid.attack_end.0;
    let y = laid.viewport.y + 10.0;
    assert!(lookahead_line_hit(&laid, (x - 5.0, y), x));
    assert!(lookahead_line_hit(&laid, (x + 5.0, y), x));
    assert!(!lookahead_line_hit(&laid, (x + 7.0, y), x));
    assert!(!lookahead_line_hit(&laid, (x, laid.viewport.y - 1.0), x));
    assert_eq!(EnvelopeView::label_rect(EnvelopeHit::Lookahead), None);
    let attack = EnvelopeView::label_rect(EnvelopeHit::AttackLength).unwrap();
    let release = EnvelopeView::label_rect(EnvelopeHit::ReleaseLength).unwrap();
    assert_eq!(attack.0, 10.0);
    assert!((release.0 + release.2 - (WIDTH - 10.0)).abs() < 0.001);
}

#[test]
fn autoscale_keeps_extreme_stages_usable_and_tracks_duration_shares() {
    for values in [
        test_values(),
        EnvelopeValues {
            attack_ms: 0.1,
            hold_ms: 0.0,
            release_ms: 5000.0,
            ..test_values()
        },
        EnvelopeValues {
            attack_ms: 5000.0,
            hold_ms: 1000.0,
            release_ms: 40.0,
            ..test_values()
        },
    ] {
        let laid = layout(Viewport::from_tuple(GRAPH), values);
        for width in [
            laid.attack_end.0 - laid.start.0,
            laid.hold_end.0 - laid.attack_end.0,
            laid.release_end.0 - laid.hold_end.0,
        ] {
            assert!(width >= GRAPH.2 / 14.0 - 0.001);
        }
        assert!((laid.release_end.0 - GRAPH.0 - GRAPH.2).abs() < 0.001);
    }
    let long_attack = layout(
        Viewport::from_tuple(GRAPH),
        EnvelopeValues {
            attack_ms: 500.0,
            ..test_values()
        },
    );
    assert!(long_attack.attack_end.0 > envelope_layout().attack_end.0);
}

#[test]
fn lookahead_tracks_each_stage_and_bar_round_trips_the_parameter_range() {
    let values = EnvelopeValues {
        hold_ms: 50.0,
        ..test_values()
    };
    let laid = layout(Viewport::from_tuple(GRAPH), values);
    for (time, x) in [
        (5.0, (laid.start.0 + laid.attack_end.0) * 0.5),
        (10.0, laid.attack_end.0),
        (35.0, (laid.attack_end.0 + laid.hold_end.0) * 0.5),
        (60.0, laid.hold_end.0),
        (110.0, (laid.hold_end.0 + laid.release_end.0) * 0.5),
        (160.0, laid.release_end.0),
        (2000.0, laid.release_end.0),
    ] {
        assert!((lookahead_marker_x(time, &laid, values) - x).abs() < 0.001);
    }
    for ms in [0.0, 1.0, 10.0, 500.0, 2000.0] {
        assert!((lookahead_bar_ms(lookahead_bar_x(ms)) - ms).abs() < 0.001);
    }
}

#[test]
fn curve_nodes_round_trip_and_follow_the_pointer_in_both_directions() {
    for curve in [-2.0, -1.0, 0.0, 1.0, 2.0] {
        let values = EnvelopeValues {
            attack_curve: curve,
            release_curve: curve,
            ..test_values()
        };
        let laid = layout(Viewport::from_tuple(GRAPH), values);
        for (hit, y) in [
            (StageHit::AttackCurve, laid.attack_curve.1),
            (StageHit::ReleaseCurve, laid.release_curve.1),
        ] {
            assert!((curve_drag(hit, &laid, y) - curve).abs() < 0.001);
            let up = curve_drag(hit, &laid, y - 5.0);
            let down = curve_drag(hit, &laid, y + 5.0);
            assert!(if hit == StageHit::AttackCurve {
                up <= curve && down >= curve
            } else {
                up >= curve && down <= curve
            });
        }
    }
}

#[test]
fn horizontal_time_drag_grows_each_stage_including_zero_hold() {
    for hit in [
        EnvelopeHit::AttackLength,
        EnvelopeHit::HoldLength,
        EnvelopeHit::ReleaseLength,
    ] {
        let plain = stage_time_drag(hit, test_values(), 10.0);
        let time = if hit == EnvelopeHit::HoldLength {
            plain
        } else {
            rate_to_ms(plain)
        };
        let start = match hit {
            EnvelopeHit::AttackLength => 10.0,
            EnvelopeHit::ReleaseLength => 100.0,
            _ => 0.0,
        };
        assert!(time > start);
    }
}

#[test]
fn vertical_time_drag_increases_up_and_keeps_its_axis() {
    for hit in [
        EnvelopeHit::AttackLength,
        EnvelopeHit::HoldLength,
        EnvelopeHit::ReleaseLength,
    ] {
        let mut axis = pleasant_ui::pointer::AxisLock::default();
        assert_eq!(axis.delta(1.0, -2.0, 1.0), 0.0);
        let up_delta = axis.delta(1.0, -10.0, 1.0);
        assert_eq!(axis.delta(100.0, -10.0, 1.0), up_delta);
        let up = stage_time_drag(hit, test_values(), up_delta);
        let down = stage_time_drag(hit, test_values(), axis.delta(100.0, 10.0, 1.0));
        if hit == EnvelopeHit::HoldLength {
            assert!(up > down);
        } else {
            assert!(rate_to_ms(up) > rate_to_ms(down));
        }
    }
}

fn envelope_layout() -> EnvelopeLayout {
    layout(
        Viewport::from_tuple(GRAPH),
        EnvelopeValues {
            attack_ms: 10.0,
            hold_ms: 0.0,
            release_ms: 100.0,
            attack_curve: 0.0,
            release_curve: 0.0,
        },
    )
}

#[test]
fn lookahead_has_a_graph_zone_and_a_monotonic_marker() {
    let laid = envelope_layout();
    let rect = EnvelopeView::zone_rect(&laid, EnvelopeHit::Lookahead);
    assert_eq!(
        EnvelopeView::graph_hit(&laid, (rect.0 + rect.2 * 0.5, rect.1 + 12.0)),
        Some(EnvelopeHit::Lookahead)
    );
    let values = test_values();
    assert_eq!(lookahead_marker_x(0.0, &laid, values), GRAPH.0);
    assert_eq!(
        lookahead_marker_x(values.attack_ms, &laid, values),
        laid.attack_end.0
    );
    assert!(lookahead_marker_x(10.0, &laid, values) < lookahead_marker_x(100.0, &laid, values));
    assert_eq!(
        lookahead_marker_x(2000.0, &laid, values),
        laid.release_end.0
    );
}

#[test]
fn rate_controls_snap_in_milliseconds() {
    let rate = EnvelopeView::snapped_plain(EnvelopeHit::AttackLength, ms_to_rate(247.0));
    assert!((rate_to_ms(rate) - 250.0).abs() < 0.001);
    let rate = EnvelopeView::snapped_plain(EnvelopeHit::ReleaseLength, ms_to_rate(4480.0));
    assert!((rate_to_ms(rate) - 4500.0).abs() < 0.001);
    assert_eq!(
        EnvelopeView::snapped_plain(EnvelopeHit::HoldLength, 78.0),
        80.0
    );
}

#[test]
fn stage_zones_cover_graph_even_with_zero_hold() {
    let laid = envelope_layout();
    for hit in [
        EnvelopeHit::AttackLength,
        EnvelopeHit::HoldLength,
        EnvelopeHit::ReleaseLength,
    ] {
        let rect = EnvelopeView::zone_rect(&laid, hit);
        assert!(rect.2 > 0.0);
        for y in [rect.1 + 2.0, rect.1 + rect.3 - 2.0] {
            assert_eq!(
                EnvelopeView::graph_hit(&laid, (rect.0 + rect.2 * 0.5, y)),
                Some(hit)
            );
        }
    }
    assert_eq!(
        EnvelopeView::graph_hit(&laid, (GRAPH.0 - 1.0, GRAPH.1)),
        None
    );
    assert_eq!(
        EnvelopeView::graph_hit(&laid, (GRAPH.0, GRAPH.1 + GRAPH.3 + 1.0)),
        None
    );
}

#[test]
fn curve_handles_take_priority_over_time_zones() {
    let laid = envelope_layout();
    assert_eq!(
        EnvelopeView::graph_hit(&laid, laid.attack_curve),
        Some(EnvelopeHit::AttackCurve)
    );
    assert_eq!(
        EnvelopeView::graph_hit(&laid, laid.release_curve),
        Some(EnvelopeHit::ReleaseCurve)
    );
}

#[test]
fn time_drags_are_relative_and_increase_displayed_time() {
    let origin = (100.0, 100.0);
    for hit in [
        EnvelopeHit::AttackLength,
        EnvelopeHit::HoldLength,
        EnvelopeHit::ReleaseLength,
    ] {
        assert_eq!(
            EnvelopeView::time_drag_delta(hit, origin, origin, false),
            0.0
        );
        let right = EnvelopeView::time_drag_delta(hit, origin, (110.0, 100.0), false);
        let up = EnvelopeView::time_drag_delta(hit, origin, (100.0, 90.0), false);
        assert_eq!(right, up);
        assert_eq!(right > 0.0, hit == EnvelopeHit::HoldLength);
        let fine = EnvelopeView::time_drag_delta(hit, origin, (110.0, 100.0), true);
        assert!(fine.abs() < right.abs());
    }
}

#[test]
fn labels_sit_under_the_graph_without_overlap() {
    let attack = EnvelopeView::label_rect(EnvelopeHit::AttackLength).unwrap();
    let hold = EnvelopeView::label_rect(EnvelopeHit::HoldLength).unwrap();
    let release = EnvelopeView::label_rect(EnvelopeHit::ReleaseLength).unwrap();
    assert!(attack.1 >= GRAPH.1 + GRAPH.3);
    assert!(attack.0 + attack.2 <= hold.0 + 0.01);
    assert!(hold.0 + hold.2 <= release.0 + 0.01);
    assert!(release.0 + release.2 <= WIDTH);
    assert_eq!(
        EnvelopeView::label_hit((attack.0 + 8.0, attack.1 + 8.0)),
        Some(EnvelopeHit::AttackLength)
    );
    assert_eq!(EnvelopeView::label_hit((0.0, 0.0)), None);
}
