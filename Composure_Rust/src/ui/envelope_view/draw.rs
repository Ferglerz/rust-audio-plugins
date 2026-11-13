use super::*;

fn dashed_line(
    d: &mut pleasant_ui::Draw<'_>,
    x: f32,
    top: f32,
    bottom: f32,
    color: nih_plug_vizia::vizia::vg::Color,
) {
    let mut y = top;
    while y < bottom {
        d.line(x, y, x, (y + 4.0).min(bottom), color, 1.0);
        y += 8.0;
    }
}

impl EnvelopeView {
    pub(super) fn draw_view(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        let mut d = appearance::painter(cx, canvas, WIDTH);
        let values = self.values();
        let laid = layout(Viewport::from_tuple(GRAPH), values);
        let points = path(&laid, values);
        let hover =
            pleasant_ui::idle_hover(self.hover, self.drag.is_some() || self.press.is_some());
        let active = self
            .drag
            .as_ref()
            .map(|drag| drag.hit)
            .or_else(|| self.press.as_ref().map(|press| press.press.target))
            .or_else(|| self.edit.as_ref().map(|edit| edit.target))
            .or_else(|| {
                hover.and_then(|(x, y)| {
                    self.interactive_hit(&laid, (x, y))
                        .or_else(|| Self::label_hit((x, y)))
                })
            });

        let lookahead_ms = self.lookahead.unmodulated_plain_value();
        let lookahead_x = lookahead_marker_x(lookahead_ms, &laid, values);
        let total = values.attack_ms + values.hold_ms + values.release_ms;
        let exceeds = lookahead_ms > total;
        let marker_color = GOLD;
        d.rect(GRAPH.0, GRAPH.1, GRAPH.2, GRAPH.3, PANEL);
        if let Some(hit) = active {
            let rect = Self::zone_rect(&laid, hit);
            let mut highlight = TEAL;
            highlight.a = 0.10;
            d.rect(rect.0, rect.1, rect.2, rect.3, highlight);
        }
        let mut anticipation = marker_color;
        anticipation.a = 0.10;
        d.rect(
            GRAPH.0,
            GRAPH.1,
            lookahead_x - GRAPH.0,
            GRAPH.3,
            anticipation,
        );
        for x in [
            laid.start.0,
            laid.attack_end.0,
            laid.hold_end.0,
            laid.release_end.0,
        ] {
            d.line(x, GRAPH.1, x, GRAPH.1 + GRAPH.3, LINE, 1.0);
        }
        d.outline(GRAPH, LINE);
        let attack_active = matches!(
            active,
            Some(EnvelopeHit::AttackLength | EnvelopeHit::AttackCurve)
        );
        let release_active = matches!(
            active,
            Some(EnvelopeHit::ReleaseLength | EnvelopeHit::ReleaseCurve)
        );
        // Draw stage paths separately so the active stage can brighten independently.
        for (left, right, stage_active) in [
            (laid.start.0, laid.attack_end.0, attack_active),
            (
                laid.attack_end.0,
                laid.hold_end.0,
                active == Some(EnvelopeHit::HoldLength),
            ),
            (laid.hold_end.0, laid.release_end.0, release_active),
        ] {
            let stage: Vec<_> = points
                .iter()
                .copied()
                .filter(|p| p.0 >= left && p.0 <= right)
                .collect();
            d.poly(
                &stage,
                if stage_active { GOLD } else { TEAL },
                if stage_active { 3.0 } else { 2.0 },
            );
        }
        for (hit, point) in [
            (EnvelopeHit::AttackCurve, laid.attack_curve),
            (EnvelopeHit::ReleaseCurve, laid.release_curve),
        ] {
            d.eq_node(
                point.0,
                point.1,
                if active == Some(hit) { GOLD } else { TEAL },
                active == Some(hit),
            );
        }
        for (left, right, hit) in [
            (laid.start.0, laid.attack_end.0, EnvelopeHit::AttackLength),
            (laid.attack_end.0, laid.hold_end.0, EnvelopeHit::HoldLength),
            (
                laid.hold_end.0,
                laid.release_end.0,
                EnvelopeHit::ReleaseLength,
            ),
        ] {
            if !hover.is_some_and(|point| self.interactive_hit(&laid, point) == Some(hit)) {
                continue;
            }
            let mut hint = MUTED;
            hint.a = 0.20;
            let center = (left + right) * 0.5;
            let y = GRAPH.1 + GRAPH.3 - 15.0;
            d.line(center - 7.0, y, center + 7.0, y, hint, 3.5);
            for direction in [-1.0, 1.0] {
                let tip = center + 7.0 * direction;
                let shoulder = center + 3.0 * direction;
                d.poly(
                    &[(shoulder, y - 3.5), (tip, y), (shoulder, y + 3.5)],
                    hint,
                    2.5,
                );
            }
        }
        // True duration proportions remain visible beneath the enlarged stage zones.
        let mut x = GRAPH.0;
        for (ms, color) in [
            (values.attack_ms, TEAL),
            (values.hold_ms, GOLD),
            (values.release_ms, TEAL),
        ] {
            let width = GRAPH.2 * ms / total;
            d.rect(x, GRAPH.1 + GRAPH.3 - 5.0, width, 3.0, color);
            x += width;
        }
        dashed_line(
            &mut d,
            lookahead_x,
            GRAPH.1,
            GRAPH.1 + GRAPH.3 - 8.0,
            marker_color,
        );
        let line_active = self
            .drag
            .as_ref()
            .is_some_and(|drag| drag.lookahead_on_graph)
            || hover.is_some_and(|point| {
                lookahead_line_hit(&laid, point, lookahead_x)
                    && self.interactive_hit(&laid, point) == Some(EnvelopeHit::Lookahead)
            });
        if line_active {
            let mut guide = GOLD;
            guide.a = 0.65;
            for x in [lookahead_x - 4.0, lookahead_x + 4.0] {
                d.line(x, GRAPH.1, x, GRAPH.1 + GRAPH.3 - 8.0, guide, 1.0);
            }
        }
        if exceeds {
            d.text_right(GRAPH.0 + GRAPH.2 - 5.0, GRAPH.1 + 14.0, ">>>", 10.0, MUTED);
        }
        let slider_active = active == Some(EnvelopeHit::Lookahead);
        d.rect(LOOK_BAR.0, LOOK_BAR.1, LOOK_BAR.2, LOOK_BAR.3, PANEL);
        let mut slider_highlight = GOLD;
        slider_highlight.a = if slider_active { 0.12 } else { 0.04 };
        d.rect(
            LOOK_BAR.0,
            LOOK_BAR.1,
            LOOK_BAR.2,
            LOOK_BAR.3,
            slider_highlight,
        );
        let mut slider_fill = GOLD;
        slider_fill.a = if slider_active { 0.28 } else { 0.18 };
        let bar_x = lookahead_bar_x(lookahead_ms);
        d.rect(
            LOOK_BAR.0,
            LOOK_BAR.1,
            bar_x - LOOK_BAR.0,
            LOOK_BAR.3,
            slider_fill,
        );
        for (ms, color) in [
            (values.attack_ms, MUTED),
            (values.attack_ms + values.hold_ms, MUTED),
            (total, MUTED),
        ] {
            if ms <= 2000.0 {
                dashed_line(
                    &mut d,
                    lookahead_bar_x(ms),
                    LOOK_BAR.1,
                    LOOK_BAR.1 + LOOK_BAR.3,
                    color,
                );
            }
        }
        let mut slider_border = GOLD;
        slider_border.a = if slider_active { 0.8 } else { 0.35 };
        d.outline(LOOK_BAR, slider_border);
        let thumb_x = bar_x.clamp(LOOK_BAR.0 + 3.0, LOOK_BAR.0 + LOOK_BAR.2 - 3.0);
        d.rect(thumb_x - 3.0, LOOK_BAR.1 + 4.0, 6.0, LOOK_BAR.3 - 8.0, GOLD);
        let text = format!(
            "LOOKAHEAD {}",
            self.lookahead
                .normalized_value_to_string(self.lookahead.unmodulated_normalized_value(), true)
        );
        if bar_x < LOOK_BAR.0 + LOOK_BAR.2 * 0.5 {
            d.text(bar_x + 10.0, LOOK_BAR.1 + 20.0, &text, 11.0, GOLD);
        } else {
            d.text_right(bar_x - 10.0, LOOK_BAR.1 + 20.0, &text, 11.0, GOLD);
        }

        for hit in [
            EnvelopeHit::AttackLength,
            EnvelopeHit::HoldLength,
            EnvelopeHit::ReleaseLength,
        ] {
            let Some(rect) = Self::label_rect(hit) else {
                continue;
            };
            let param = self.param(hit);
            let name = match hit {
                EnvelopeHit::AttackLength => "ATT",
                EnvelopeHit::HoldLength => "HOLD",
                EnvelopeHit::ReleaseLength => "REL",
                _ => "",
            };
            let value =
                param.normalized_value_to_string(param.unmodulated_normalized_value(), true);
            let accent = if active == Some(hit) { GOLD } else { TEXT };
            d.text(rect.0 + 4.0, rect.1 + 14.0, name, 11.0, MUTED);
            if self.edit.as_ref().is_some_and(|edit| edit.target == hit) {
                continue;
            }
            d.rect(rect.0 + 2.0, rect.1 + 17.0, rect.2 - 6.0, 18.0, PANEL);
            d.text(rect.0 + 4.0, rect.1 + 30.0, &value, 13.0, accent);
            if hover.is_some_and(|p| Self::label_hit(p) == Some(hit)) {
                d.value_underline(
                    (rect.0, rect.1 + 14.0, rect.2 - 8.0, 18.0),
                    rect.1 + 30.0,
                    GOLD,
                );
            }
        }
        if let Some(edit) = &self.edit {
            d.value_edit(edit, GOLD);
        }
    }
}
