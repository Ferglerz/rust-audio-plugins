use super::*;

#[test]
fn dropdown_selects_every_shape_and_rejects_outside_clicks() {
    for (row, shape) in DetectorShape::ALL.iter().enumerate() {
        assert_eq!(
            menu_shape((
                SHAPE_MENU.0 + 12.0,
                SHAPE_MENU.1 + (row as f32 + 0.5) * MENU_ROW_H
            )),
            Some(*shape)
        );
    }
    assert_eq!(menu_shape((SHAPE_RECT.0, ACTIONS_Y + 10.0)), None);
    assert_eq!(menu_shape((SHAPE_MENU.0 - 1.0, SHAPE_MENU.1)), None);
    assert!(SHAPE_MENU.1 >= GRAPH.1);
}

#[test]
fn graph_axes_round_trip_and_clamp_at_edges() {
    for frequency in [20.0, 100.0, 1000.0, 10_000.0, 20_000.0] {
        assert!((x_frequency(frequency_x(frequency)) / frequency - 1.0).abs() < 0.0001);
    }
    for gain in [-24.0, -12.0, 0.0, 12.0, 24.0] {
        assert!((y_gain(gain_y(gain)) - gain).abs() < 0.001);
    }
    assert_eq!(x_frequency(GRAPH.0 - 100.0), 20.0);
    assert_eq!(x_frequency(GRAPH.0 + GRAPH.2 + 100.0), 20_000.0);
    assert_eq!(y_gain(GRAPH.1 - 100.0), GAIN_DB);
    assert_eq!(y_gain(GRAPH.1 + GRAPH.3 + 100.0), -GAIN_DB);
}

#[test]
fn drag_fields_are_snapshotted_before_automated_shape_changes() {
    for (initial, automated, gain_started) in [
        (EqShape::Bell, EqShape::LowCut, true),
        (EqShape::LowCut, EqShape::Bell, false),
    ] {
        let mut settings = BandSettings {
            shape: initial,
            ..BandSettings::default()
        };
        let drag = NodeDrag {
            node: Node::Band(0),
            fields: node_fields(Node::Band(0), settings.shape),
            origin: (0.0, 0.0),
            frequency: 1000.0,
            gain: 0.0,
            q: 1.0,
        };
        settings.shape = automated;
        assert_eq!(drag.fields.contains(&Field::Gain), gain_started);
        assert_ne!(drag.fields, node_fields(drag.node, settings.shape));
        assert!(drag.fields.contains(&Field::Frequency));
        assert!(drag.fields.contains(&Field::Q));
    }
}

#[test]
fn response_sampling_stays_below_nyquist_at_low_sample_rates() {
    assert_eq!(
        response_frequency(frequency_x(20_000.0), 32_000.0),
        15_680.0
    );
    assert_eq!(
        response_frequency(frequency_x(20_000.0), 48_000.0),
        20_000.0
    );
}

#[test]
fn footer_readouts_fit_the_view_and_clear_the_graph() {
    for fields in [
        vec![Field::Frequency],
        vec![Field::Frequency, Field::Q],
        vec![Field::Frequency, Field::Gain, Field::Q],
    ] {
        for &field in &fields {
            let (x, y, w, h) = field_rect(field, &fields);
            assert!(x >= 0.0 && x + w <= WIDTH);
            assert_eq!(y, ACTIONS_Y);
            assert!(y + h <= HEIGHT);
            assert!(x >= SHAPE_RECT.0 + SHAPE_RECT.2 + 6.0);
            assert!(x + w <= REMOVE_RECT.0 - 6.0);
            assert!(GRAPH.1 + GRAPH.3 + 18.0 < y);
        }
        assert_eq!(
            ACTIONS_Y + super::super::appearance::PLEASANT_GRAPH_Y
                - 12.0
                - super::super::graph_pages::CURVE_HEADER_H,
            super::super::appearance::ENV_Y + super::super::appearance::SIDE_H - 36.0
        );
        assert!(LISTEN_RECT.1 >= 0.0 && LISTEN_RECT.1 + LISTEN_RECT.3 < GRAPH.1);
        assert_eq!(LISTEN_RECT.1, SIDECHAIN_RECT.1);
        assert_eq!(LISTEN_RECT.0 + LISTEN_RECT.2 + 8.0, SIDECHAIN_RECT.0);
        assert!(LISTEN_RECT.0 > 16.0 + "DETECTOR EQ".len() as f32 * 9.0);
        assert_eq!(SIDECHAIN_RECT.0 + SIDECHAIN_RECT.2, WIDTH - 16.0);
        assert!(SIDECHAIN_RECT.0 + SIDECHAIN_RECT.2 <= WIDTH);
    }
}
