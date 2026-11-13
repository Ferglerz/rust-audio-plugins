use super::*;

pub(super) fn lookahead_line_hit(laid: &EnvelopeLayout, point: (f32, f32), x: f32) -> bool {
    point.1 >= laid.viewport.y
        && point.1 <= laid.viewport.y + laid.viewport.height
        && point.0 >= laid.start.0
        && point.0 <= laid.release_end.0
        && (point.0 - x).abs() <= 6.0
}

impl EnvelopeView {
    pub(super) fn label_rect(hit: EnvelopeHit) -> Option<(f32, f32, f32, f32)> {
        let index = match hit {
            EnvelopeHit::AttackLength => 0,
            EnvelopeHit::HoldLength => 1,
            EnvelopeHit::ReleaseLength => 2,
            _ => return None,
        };
        let width = (WIDTH - 20.0) / 3.0;
        Some((
            10.0 + width * index as f32,
            HEIGHT - LABEL_H - 8.0,
            width,
            LABEL_H,
        ))
    }

    pub(super) fn label_hit(p: (f32, f32)) -> Option<EnvelopeHit> {
        [
            EnvelopeHit::AttackLength,
            EnvelopeHit::HoldLength,
            EnvelopeHit::ReleaseLength,
        ]
        .into_iter()
        .find(|hit| {
            Self::label_rect(*hit).is_some_and(|rect| {
                p.0 >= rect.0 && p.0 <= rect.0 + rect.2 && p.1 >= rect.1 && p.1 <= rect.1 + rect.3
            })
        })
    }

    pub(super) fn zone_rect(laid: &EnvelopeLayout, hit: EnvelopeHit) -> (f32, f32, f32, f32) {
        if hit == EnvelopeHit::Lookahead {
            return LOOK_BAR;
        }
        let (left, right) = match hit {
            EnvelopeHit::Lookahead => unreachable!(),
            EnvelopeHit::AttackLength | EnvelopeHit::AttackCurve => {
                (laid.start.0, laid.attack_end.0)
            }
            EnvelopeHit::HoldLength => (laid.attack_end.0, laid.hold_end.0),
            EnvelopeHit::ReleaseLength | EnvelopeHit::ReleaseCurve => {
                (laid.hold_end.0, laid.viewport.x + laid.viewport.width)
            }
        };
        (left, laid.viewport.y, right - left, laid.viewport.height)
    }

    pub(super) fn graph_hit(laid: &EnvelopeLayout, point: (f32, f32)) -> Option<EnvelopeHit> {
        if point.0 >= LOOK_BAR.0
            && point.0 <= LOOK_BAR.0 + LOOK_BAR.2
            && point.1 >= LOOK_BAR.1
            && point.1 <= LOOK_BAR.1 + LOOK_BAR.3
        {
            return Some(EnvelopeHit::Lookahead);
        }
        let rect = laid.graph_rect();
        if point.0 < rect.0
            || point.0 > rect.0 + rect.2
            || point.1 < rect.1
            || point.1 > rect.1 + rect.3
        {
            return None;
        }
        // Curve handles take priority over the stage's drag zone.
        let curve = [
            (EnvelopeHit::AttackCurve, laid.attack_curve),
            (EnvelopeHit::ReleaseCurve, laid.release_curve),
        ]
        .into_iter()
        .filter_map(|(hit, center)| {
            let distance = (center.0 - point.0).hypot(center.1 - point.1);
            (distance <= 14.0).then_some((hit, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((hit, _)) = curve {
            return Some(hit);
        }
        Some(if point.0 < laid.attack_end.0 {
            EnvelopeHit::AttackLength
        } else if point.0 < laid.hold_end.0 {
            EnvelopeHit::HoldLength
        } else {
            EnvelopeHit::ReleaseLength
        })
    }

    pub(super) fn interactive_hit(
        &self,
        laid: &EnvelopeLayout,
        point: (f32, f32),
    ) -> Option<EnvelopeHit> {
        let hit = Self::graph_hit(laid, point);
        // Keep the explicit curve nodes usable when the line crosses one.
        if matches!(
            hit,
            Some(EnvelopeHit::AttackCurve | EnvelopeHit::ReleaseCurve)
        ) {
            return hit;
        }
        let x = lookahead_marker_x(
            self.lookahead.unmodulated_plain_value(),
            laid,
            self.values(),
        );
        if lookahead_line_hit(laid, point, x) {
            Some(EnvelopeHit::Lookahead)
        } else {
            hit
        }
    }
}
