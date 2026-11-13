use super::*;

impl DetectorEqView {
    fn draw_curve(
        d: &mut Draw,
        response: &[f64],
        color: nih_plug_vizia::vizia::vg::Color,
        width: f32,
    ) {
        let points: [(f32, f32); 513] = std::array::from_fn(|i| {
            let x = GRAPH.0 + GRAPH.2 * i as f32 / 512.0;
            let value = response[i];
            let y = GRAPH.1 + GRAPH.3 * (0.5 - value as f32 / (GAIN_DB * 2.0));
            (x, y)
        });
        d.poly(&points, color, width);
    }

    pub(super) fn draw_view(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let mut d = super::super::appearance::painter(cx, canvas, WIDTH);
        d.text(
            16.0,
            22.0,
            "DETECTOR EQ",
            super::super::appearance::MODULE_TITLE_SIZE,
            pleasant_ui::TEXT,
        );
        let listening = self.params.sc_adjust_preview.value();
        super::super::appearance::button(&mut d, LISTEN_RECT, "", listening, GOLD);
        d.headphones(
            LISTEN_RECT.0 + LISTEN_RECT.2 * 0.5,
            LISTEN_RECT.1 + LISTEN_RECT.3 * 0.5,
            if listening { GOLD } else { MUTED },
        );
        super::super::appearance::button(
            &mut d,
            SIDECHAIN_RECT,
            "SIDECHAIN TO DETECTOR",
            self.params.use_sidechain.value(),
            pleasant_ui::TEAL,
        );
        for (frequency, label) in [
            (20.0, "20"),
            (50.0, "50"),
            (100.0, "100"),
            (200.0, "200"),
            (500.0, "500"),
            (1000.0, "1k"),
            (2000.0, "2k"),
            (5000.0, "5k"),
            (10000.0, "10k"),
            (20000.0, "20k"),
        ] {
            let x = frequency_x(frequency);
            d.line(x, GRAPH.1, x, GRAPH.1 + GRAPH.3, LINE, 0.7);
            d.text_centered(x, GRAPH.1 + GRAPH.3 + 18.0, label, 13.0, MUTED);
        }
        for gain in [-24.0, -12.0, 0.0, 12.0, 24.0] {
            let y = gain_y(gain);
            d.text_right(GRAPH.0 - 8.0, y + 4.0, &format!("{gain:.0}"), 13.0, MUTED);
            d.line(
                GRAPH.0,
                y,
                GRAPH.0 + GRAPH.2,
                y,
                LINE,
                if gain == 0.0 { 1.0 } else { 0.6 },
            );
        }
        let rate = (self.display.sample_rate.load(Ordering::Relaxed) as f64).max(1.0);
        let mut nodes = self.nodes();
        nodes.sort_by_key(|node| Some(*node) == self.selected);
        let enabled: Vec<_> = nodes
            .iter()
            .filter_map(|&node| {
                let settings = self.settings(node);
                settings
                    .enabled
                    .then(|| (node, BandCoefficients::prepare(&settings, rate)))
            })
            .collect();
        let coefficients: Vec<_> = enabled
            .iter()
            .map(|(_, coefficients)| *coefficients)
            .collect();
        let frequencies: [f64; 513] =
            std::array::from_fn(|i| response_frequency(GRAPH.0 + GRAPH.2 * i as f32 / 512.0, rate));
        let mut response_cache = self.response_cache.borrow_mut();
        response_cache.update(&coefficients, &frequencies, rate);
        d.c.save();
        d.c.intersect_scissor(
            d.ox + (GRAPH.0 + d.offset_x) * d.s,
            d.oy + GRAPH.1 * d.s,
            GRAPH.2 * d.s,
            GRAPH.3 * d.s,
        );
        for (index, (node, _)) in enabled.iter().enumerate() {
            let mut color = node_color(*node);
            color.a = 0.5;
            Self::draw_curve(&mut d, response_cache.band(index), color, 1.0);
        }
        Self::draw_curve(&mut d, response_cache.sum(), GOLD, 1.7);
        d.c.restore();
        for node in nodes {
            let settings = self.settings(node);
            let x = frequency_x(settings.frequency_hz as f32);
            let y = gain_y(if settings.shape.has_gain() {
                settings.gain_db as f32
            } else {
                0.0
            });
            let mut color = node_color(node);
            if !settings.enabled {
                color.a = 0.35;
            }
            d.eq_node(x, y, color, self.selected == Some(node));
        }
        if let Some(node) = self.selected {
            let color = node_color(node);
            match node {
                Node::Band(i) => {
                    let b = &self.params.detector_eq[i];
                    super::super::appearance::button(
                        &mut d,
                        SHAPE_RECT,
                        &format!("{} ▾", b.shape.value().label().to_uppercase()),
                        true,
                        color,
                    );
                    d.bypass_button(
                        BYPASS_RECT,
                        !b.enabled.value(),
                        color,
                        self.hover
                            .is_some_and(|p| point_in_rect(p.0, p.1, BYPASS_RECT)),
                        self.bypass_anim.step(),
                    );
                }
                Node::HighPass => {
                    super::super::appearance::button(&mut d, SHAPE_RECT, "LOW CUT", true, color)
                }
                Node::LowPass => {
                    super::super::appearance::button(&mut d, SHAPE_RECT, "HIGH CUT", true, color)
                }
            }
            super::super::appearance::button(&mut d, REMOVE_RECT, "×", false, MUTED);
            for field in self.fields(node) {
                let target = Target { node, field };
                let r = field_rect(field, &self.fields(node));
                let highlighted = self.hover.is_some_and(|p| point_in_rect(p.0, p.1, r))
                    || self.press.as_ref().is_some_and(|p| p.press.rect == r);
                if highlighted {
                    d.outline(r, color);
                }
                d.text(
                    r.0 + 4.0,
                    r.1 + 18.0,
                    match field {
                        Field::Frequency => "FREQ",
                        Field::Gain => "GAIN",
                        Field::Q => "Q",
                    },
                    11.0,
                    MUTED,
                );
                let p = self.param(target);
                let value = match field {
                    Field::Frequency if p.value() >= 1000.0 => {
                        format!("{:.2}k", p.value() / 1000.0)
                    }
                    Field::Frequency => format!("{:.0}", p.value()),
                    Field::Gain => format!("{:+.1} dB", p.value()),
                    Field::Q => format!("{:.2}", p.value()),
                };
                d.text_right(r.0 + r.2 - 4.0, r.1 + 18.0, &value, 13.0, color);
            }
        } else {
            d.text_centered(
                CENTER_X,
                ACTIONS_Y + 20.0,
                "CLICK TO ADD A BAND",
                11.0,
                MUTED,
            );
        }
        if let Some(edit) = &self.edit {
            d.value_edit(edit, node_color(edit.target.node));
        }
        if self.shape_menu {
            if let Some(Node::Band(i)) = self.selected {
                d.rect(
                    SHAPE_MENU.0,
                    SHAPE_MENU.1,
                    SHAPE_MENU.2,
                    SHAPE_MENU.3,
                    PANEL,
                );
                for (row, shape) in DetectorShape::ALL.iter().enumerate() {
                    let y = SHAPE_MENU.1 + row as f32 * MENU_ROW_H;
                    let selected = self.params.detector_eq[i].shape.value() == *shape;
                    if selected || self.hover.is_some_and(|p| menu_shape(p) == Some(*shape)) {
                        d.rect(SHAPE_MENU.0, y, SHAPE_MENU.2, MENU_ROW_H, LINE);
                    }
                    d.filter_curve(
                        shape.eq_shape().biquad_kind(),
                        (SHAPE_MENU.0 + 10.0, y + 5.0, 30.0, 16.0),
                        if selected { GOLD } else { TEXT },
                    );
                    d.text(
                        SHAPE_MENU.0 + 50.0,
                        y + MENU_ROW_H * 0.5 + 13.0 * 0.32,
                        shape.label(),
                        13.0,
                        if selected { GOLD } else { TEXT },
                    );
                }
                d.outline(SHAPE_MENU, LINE);
            }
        }
    }
}
