use super::*;

impl DereverbView {
    pub(super) fn node_at(&self, point: (f32, f32)) -> Option<usize> {
        let viewport = Viewport::from_tuple(CURVE);
        self.params
            .weighting()
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, &weight)| {
                let (x, y) = viewport.from_normalized(i as f32 / (NODES - 1) as f32, weight);
                let distance = (x - point.0).hypot(y - point.1);
                (distance <= 13.0).then_some((i, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index)
    }

    pub(super) fn set_curve(&mut self, cx: &mut EventContext, nodes: [f32; NODES]) {
        self.end_drag(cx);
        for (parameter, value) in self.params.weights().into_iter().zip(nodes) {
            param::set_normalized_once(cx, parameter.as_ptr(), value);
        }
    }

    pub(super) fn paint_curve(&self, d: &mut Draw) {
        let curve = self.params.weighting();
        let viewport = Viewport::from_tuple(CURVE);
        d.text(64.0, 104.0, "LEVEL WEIGHTING", 11.0, MUTED);
        d.button(
            RELATIVE,
            "RELATIVE",
            curve.mode == LevelMode::Relative,
            GOLD,
        );
        d.button(
            ABSOLUTE,
            "ABSOLUTE",
            curve.mode == LevelMode::Absolute,
            GOLD,
        );
        d.text(64.0, 130.0, "WEIGHT %", 10.0, MUTED);
        for value in [0, 25, 50, 75, 100] {
            let (_, y) = viewport.from_normalized(0.0, value as f32 / 100.0);
            d.line(CURVE.0, y, CURVE.0 + CURVE.2, y, LINE, 1.0);
            d.text_right(CURVE.0 - 12.0, y + 4.0, &value.to_string(), 10.0, MUTED);
        }
        for i in 0..NODES {
            let position = i as f32 / (NODES - 1) as f32;
            let (x, _) = viewport.from_normalized(position, 0.0);
            d.line(x, CURVE.1, x, CURVE.1 + CURVE.3, LINE, 1.0);
            d.text_centered(
                x,
                372.0,
                &format!("{:.0}", curve.mode.db(position)),
                10.0,
                MUTED,
            );
        }
        d.text_centered(
            CURVE.0 + CURVE.2 * 0.5,
            391.0,
            if curve.mode == LevelMode::Relative {
                "LEVEL RELATIVE TO AVERAGE · dB"
            } else {
                "INPUT LEVEL · dBFS"
            },
            10.0,
            MUTED,
        );
        let points: [(f32, f32); 129] = std::array::from_fn(|i| {
            let x = i as f32 / 128.0;
            viewport.from_normalized(x, curve.at(x))
        });
        let mut fill = GOLD;
        fill.a = 0.06;
        d.area(&points, CURVE.1 + CURVE.3, fill);
        d.poly(&points, GOLD, 2.0);
        for (i, &value) in curve.nodes.iter().enumerate() {
            let (x, y) = viewport.from_normalized(i as f32 / (NODES - 1) as f32, value);
            let selected = self.curve_drag.is_some_and(|(n, _, _)| n == i)
                || self.hover.is_some_and(|p| self.node_at(p) == Some(i));
            d.circle(x, y, if selected { 6.5 } else { 5.0 }, GOLD, true);
            if selected {
                d.circle(x, y, 10.0, GOLD, false);
                d.text_right(
                    560.0,
                    130.0,
                    &format!(
                        "{:.0} dB / {:.1} %",
                        curve.mode.db(i as f32 / 4.0),
                        value * 100.0
                    ),
                    10.0,
                    GOLD,
                );
            }
        }
        let db = match curve.mode {
            LevelMode::Relative => self.shared.relative_db.load(Ordering::Relaxed),
            LevelMode::Absolute => self.shared.absolute_db.load(Ordering::Relaxed),
        };
        let active = self.shared.active_level.load(Ordering::Relaxed) != 0;
        if active {
            let (x, y) = viewport.from_normalized(
                curve.mode.position(db),
                self.shared.weight.load(Ordering::Relaxed),
            );
            let mut glow = TEAL;
            glow.a = 0.16;
            d.circle(x, y, 11.0, glow, true);
            d.circle(x, y, 5.5, TEAL, true);
            d.circle(x, y, 8.0, TEAL, false);
        }
        d.button(FLAT, "FLAT", curve.nodes == [1.0; NODES], GOLD);
        d.button(
            LOUDER_BUTTON,
            "LOUDER",
            curve
                .nodes
                .iter()
                .zip(LOUDER)
                .all(|(a, b)| (a - b).abs() < 1e-5),
            GOLD,
        );
        d.text(600.0, 430.0, "EFFECTIVE STRENGTH", 10.0, MUTED);
        d.text(
            600.0,
            454.0,
            &format!(
                "{:.1} %",
                if active {
                    self.shared.effective.load(Ordering::Relaxed) * 100.0
                } else {
                    0.0
                }
            ),
            18.0,
            TEAL,
        );
    }
}
