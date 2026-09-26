//! Attack / hold / release envelope geometry for graphical editors.
//!
//! Lengths are milliseconds. Curves use Composure's `[-2, 2]` range:
//! negative is aggressive (fast then slow), positive is smooth (slow then fast).
//! Segment widths use a log time map plus a trailing pad so the release end
//! stays draggable instead of gluing to the viewport edge.

use crate::graph::Viewport;
use crate::handles::{tag_contains, TagPointer};

pub const TIME_NUMERATOR: f32 = 10_000.0;
pub const ATTACK_MIN_MS: f32 = TIME_NUMERATOR / 100_000.0;
pub const ATTACK_MAX_MS: f32 = TIME_NUMERATOR / 2.0;
pub const RELEASE_MIN_MS: f32 = TIME_NUMERATOR / 250.0;
pub const RELEASE_MAX_MS: f32 = TIME_NUMERATOR / 2.0;
pub const HOLD_MIN_MS: f32 = 0.0;
pub const HOLD_MAX_MS: f32 = 1_000.0;
pub const CURVE_MIN: f32 = -2.0;
pub const CURVE_MAX: f32 = 2.0;

const MIN_WEIGHT: f32 = 0.16;
const PAD_WEIGHT: f32 = 0.10;
const HIT_RADIUS: f32 = 14.0;
const PATH_STEPS: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvelopeValues {
    pub attack_ms: f32,
    pub hold_ms: f32,
    pub release_ms: f32,
    pub attack_curve: f32,
    pub release_curve: f32,
}

impl EnvelopeValues {
    pub fn clamped(self) -> Self {
        Self {
            attack_ms: self.attack_ms.clamp(ATTACK_MIN_MS, ATTACK_MAX_MS),
            hold_ms: self.hold_ms.clamp(HOLD_MIN_MS, HOLD_MAX_MS),
            release_ms: self.release_ms.clamp(RELEASE_MIN_MS, RELEASE_MAX_MS),
            attack_curve: self.attack_curve.clamp(CURVE_MIN, CURVE_MAX),
            release_curve: self.release_curve.clamp(CURVE_MIN, CURVE_MAX),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnvelopeHit {
    AttackLength,
    HoldLength,
    ReleaseLength,
    AttackCurve,
    ReleaseCurve,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvelopeLayout {
    pub viewport: Viewport,
    pub rest_y: f32,
    pub hold_y: f32,
    pub start: (f32, f32),
    pub attack_end: (f32, f32),
    pub hold_end: (f32, f32),
    pub release_end: (f32, f32),
    pub attack_curve: (f32, f32),
    pub release_curve: (f32, f32),
}

impl EnvelopeLayout {
    pub fn graph_rect(self) -> (f32, f32, f32, f32) {
        (
            self.viewport.x,
            self.viewport.y,
            self.viewport.width,
            self.viewport.height,
        )
    }
}

pub fn rate_to_ms(rate: f32) -> f32 {
    TIME_NUMERATOR / rate.max(1.0e-6)
}

pub fn ms_to_rate(ms: f32) -> f32 {
    TIME_NUMERATOR / ms.max(1.0e-6)
}

pub fn ease(t: f32, curve: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if curve.abs() < 1.0e-4 {
        return t;
    }
    let amount = curve.abs() * 0.85;
    if curve < 0.0 {
        1.0 - (1.0 - t).powf(1.0 + amount)
    } else {
        t.powf(1.0 + amount)
    }
}

pub fn curve_from_mid_norm(mid_norm: f32) -> f32 {
    let mid = mid_norm.clamp(0.02, 0.98);
    if (mid - 0.5).abs() < 1.0e-4 {
        return 0.0;
    }
    let amount = if mid > 0.5 {
        (1.0 - mid).log(0.5) - 1.0
    } else {
        mid.log(0.5) - 1.0
    };
    let curve = (amount / 0.85).clamp(0.0, CURVE_MAX);
    if mid > 0.5 {
        -curve
    } else {
        curve
    }
}

fn log_norm(ms: f32, min_ms: f32, max_ms: f32) -> f32 {
    let min_ms = min_ms.max(1.0e-6);
    let max_ms = max_ms.max(min_ms);
    let ms = ms.clamp(min_ms, max_ms);
    ((ms.ln() - min_ms.ln()) / (max_ms.ln() - min_ms.ln()).max(1.0e-6)).clamp(0.0, 1.0)
}

fn from_log_norm(norm: f32, min_ms: f32, max_ms: f32) -> f32 {
    let min_ms = min_ms.max(1.0e-6);
    let max_ms = max_ms.max(min_ms);
    (min_ms.ln() + norm.clamp(0.0, 1.0) * (max_ms.ln() - min_ms.ln())).exp()
}

fn weight(norm: f32) -> f32 {
    MIN_WEIGHT + norm.clamp(0.0, 1.0)
}

fn weights(values: EnvelopeValues) -> (f32, f32, f32) {
    let values = values.clamped();
    (
        weight(log_norm(values.attack_ms, ATTACK_MIN_MS, ATTACK_MAX_MS)),
        weight((values.hold_ms / HOLD_MAX_MS).clamp(0.0, 1.0)),
        weight(log_norm(values.release_ms, RELEASE_MIN_MS, RELEASE_MAX_MS)),
    )
}

fn shares(values: EnvelopeValues) -> (f32, f32, f32) {
    let (a, h, r) = weights(values);
    let total = a + h + r + PAD_WEIGHT;
    (a / total, h / total, r / total)
}

fn point_on_segment(start: (f32, f32), end: (f32, f32), t: f32, curve: f32) -> (f32, f32) {
    let eased = ease(t, curve);
    (
        start.0 + (end.0 - start.0) * t,
        start.1 + (end.1 - start.1) * eased,
    )
}

pub fn layout(viewport: Viewport, values: EnvelopeValues) -> EnvelopeLayout {
    let values = values.clamped();
    let (sa, sh, sr) = shares(values);
    let rest_y = viewport.y + viewport.height * 0.16;
    let hold_y = viewport.y + viewport.height * 0.82;
    let start = (viewport.x, rest_y);
    let attack_end = (viewport.x + viewport.width * sa, hold_y);
    let hold_end = (attack_end.0 + viewport.width * sh, hold_y);
    let release_end = (hold_end.0 + viewport.width * sr, rest_y);
    EnvelopeLayout {
        viewport,
        rest_y,
        hold_y,
        start,
        attack_end,
        hold_end,
        release_end,
        attack_curve: point_on_segment(start, attack_end, 0.5, values.attack_curve),
        release_curve: point_on_segment(hold_end, release_end, 0.5, values.release_curve),
    }
}

pub fn path(layout: &EnvelopeLayout, values: EnvelopeValues) -> Vec<(f32, f32)> {
    let values = values.clamped();
    let mut points = Vec::with_capacity(PATH_STEPS * 2 + 4);
    for i in 0..=PATH_STEPS {
        points.push(point_on_segment(
            layout.start,
            layout.attack_end,
            i as f32 / PATH_STEPS as f32,
            values.attack_curve,
        ));
    }
    points.push(layout.hold_end);
    for i in 1..=PATH_STEPS {
        points.push(point_on_segment(
            layout.hold_end,
            layout.release_end,
            i as f32 / PATH_STEPS as f32,
            values.release_curve,
        ));
    }
    points
}

fn nearest(points: &[(EnvelopeHit, f32, f32)], x: f32, y: f32) -> Option<EnvelopeHit> {
    points
        .iter()
        .filter_map(|(hit, px, py)| {
            let distance = (px - x).hypot(py - y);
            (distance <= HIT_RADIUS).then_some((*hit, distance))
        })
        .min_by(|left, right| left.1.total_cmp(&right.1))
        .map(|(hit, _)| hit)
}

pub fn hit_test(layout: &EnvelopeLayout, x: f32, y: f32) -> Option<EnvelopeHit> {
    if let Some(hit) = nearest(
        &[
            (
                EnvelopeHit::AttackCurve,
                layout.attack_curve.0,
                layout.attack_curve.1,
            ),
            (
                EnvelopeHit::ReleaseCurve,
                layout.release_curve.0,
                layout.release_curve.1,
            ),
        ],
        x,
        y,
    ) {
        return Some(hit);
    }

    if tag_contains(
        layout.attack_end.0,
        layout.attack_end.1,
        TagPointer::Down,
        x,
        y,
    ) {
        return Some(EnvelopeHit::AttackLength);
    }
    if tag_contains(layout.hold_end.0, layout.hold_end.1, TagPointer::Down, x, y) {
        return Some(EnvelopeHit::HoldLength);
    }
    if tag_contains(
        layout.release_end.0,
        layout.release_end.1,
        TagPointer::None,
        x,
        y,
    ) {
        return Some(EnvelopeHit::ReleaseLength);
    }

    nearest(
        &[
            (
                EnvelopeHit::AttackLength,
                layout.attack_end.0,
                layout.attack_end.1,
            ),
            (
                EnvelopeHit::HoldLength,
                layout.hold_end.0,
                layout.hold_end.1,
            ),
            (
                EnvelopeHit::ReleaseLength,
                layout.release_end.0,
                layout.release_end.1,
            ),
        ],
        x,
        y,
    )
}

fn weight_from_share(share: f32, other: f32) -> f32 {
    let share = share.clamp(0.02, 0.92);
    share * (other + PAD_WEIGHT) / (1.0 - share)
}

fn norm_from_weight(value: f32) -> f32 {
    ((value - MIN_WEIGHT) / 1.0).clamp(0.0, 1.0)
}

pub fn drag_length(hit: EnvelopeHit, values: EnvelopeValues, viewport: Viewport, x: f32) -> f32 {
    let values = values.clamped();
    let (wa, wh, wr) = weights(values);
    let t = viewport.to_normalized_clamped(x, viewport.y).0;

    match hit {
        EnvelopeHit::AttackLength => {
            let attack = weight_from_share(t, wh + wr);
            from_log_norm(norm_from_weight(attack), ATTACK_MIN_MS, ATTACK_MAX_MS)
        }
        EnvelopeHit::HoldLength => {
            let sa = wa / (wa + wh + wr + PAD_WEIGHT);
            let hold_end = t.max(sa + 0.02);
            let hold = weight_from_share(hold_end, wr) - wa;
            norm_from_weight(hold.max(MIN_WEIGHT)) * HOLD_MAX_MS
        }
        EnvelopeHit::ReleaseLength => {
            let sa = wa / (wa + wh + wr + PAD_WEIGHT);
            let sh = wh / (wa + wh + wr + PAD_WEIGHT);
            let release_end = t.max(sa + sh + 0.02);
            let release = weight_from_share(release_end, 0.0) - wa - wh;
            from_log_norm(
                norm_from_weight(release.max(MIN_WEIGHT)),
                RELEASE_MIN_MS,
                RELEASE_MAX_MS,
            )
        }
        EnvelopeHit::AttackCurve | EnvelopeHit::ReleaseCurve => {
            unreachable!("curve drags use drag_curve")
        }
    }
}

pub fn drag_curve(hit: EnvelopeHit, layout: &EnvelopeLayout, y: f32) -> f32 {
    let (rest, hold) = (layout.rest_y, layout.hold_y);
    let mid_norm = match hit {
        EnvelopeHit::AttackCurve => ((y - rest) / (hold - rest).copysign(1.0)).clamp(0.02, 0.98),
        EnvelopeHit::ReleaseCurve => ((y - hold) / (rest - hold)).clamp(0.02, 0.98),
        _ => 0.5,
    };
    // Attack travels rest → hold (down). Release travels hold → rest (up).
    // Mid-norm is progress along that segment at t=0.5.
    match hit {
        EnvelopeHit::AttackCurve => curve_from_mid_norm(mid_norm),
        EnvelopeHit::ReleaseCurve => curve_from_mid_norm(mid_norm),
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewport() -> Viewport {
        Viewport::new(10.0, 20.0, 200.0, 100.0)
    }

    fn defaults() -> EnvelopeValues {
        EnvelopeValues {
            attack_ms: 10.0,
            hold_ms: 0.0,
            release_ms: 100.0,
            attack_curve: 0.0,
            release_curve: 0.0,
        }
    }

    #[test]
    fn rate_and_ms_round_trip() {
        for rate in [2.0, 100.0, 1000.0, 100_000.0] {
            assert!((ms_to_rate(rate_to_ms(rate)) - rate).abs() < 1.0e-3);
        }
    }

    #[test]
    fn linear_curve_is_identity() {
        for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
            assert_eq!(ease(t, 0.0), t);
        }
    }

    #[test]
    fn negative_curve_is_fast_then_slow() {
        assert!(ease(0.5, -2.0) > 0.5);
        assert!(ease(0.5, 2.0) < 0.5);
    }

    #[test]
    fn mid_norm_recovers_curve_sign() {
        assert!(curve_from_mid_norm(0.75) < 0.0);
        assert!(curve_from_mid_norm(0.25) > 0.0);
        assert_eq!(curve_from_mid_norm(0.5), 0.0);
    }

    #[test]
    fn default_layout_keeps_every_stage_visible() {
        let laid = layout(viewport(), defaults());
        assert!(laid.attack_end.0 - laid.start.0 > 20.0);
        assert!(laid.hold_end.0 - laid.attack_end.0 > 20.0);
        assert!(laid.release_end.0 - laid.hold_end.0 > 20.0);
        assert!(laid.release_end.0 < laid.viewport.x + laid.viewport.width - 8.0);
        assert_eq!(laid.attack_end.1, laid.hold_end.1);
        assert_eq!(laid.start.1, laid.release_end.1);
        assert!(laid.hold_y > laid.rest_y);
    }

    #[test]
    fn longer_times_widen_their_segments() {
        let short = layout(viewport(), defaults());
        let long_attack = layout(
            viewport(),
            EnvelopeValues {
                attack_ms: 500.0,
                ..defaults()
            },
        );
        let long_hold = layout(
            viewport(),
            EnvelopeValues {
                hold_ms: 400.0,
                ..defaults()
            },
        );
        let long_release = layout(
            viewport(),
            EnvelopeValues {
                release_ms: 800.0,
                ..defaults()
            },
        );
        assert!(long_attack.attack_end.0 > short.attack_end.0);
        assert!(
            long_hold.hold_end.0 - long_hold.attack_end.0 > short.hold_end.0 - short.attack_end.0
        );
        assert!(
            long_release.release_end.0 - long_release.hold_end.0
                > short.release_end.0 - short.hold_end.0
        );
    }

    #[test]
    fn hit_test_prefers_curve_handles() {
        let laid = layout(viewport(), defaults());
        assert_eq!(
            hit_test(&laid, laid.attack_curve.0, laid.attack_curve.1),
            Some(EnvelopeHit::AttackCurve)
        );
        assert_eq!(
            hit_test(&laid, laid.attack_end.0, laid.attack_end.1 - 8.0),
            Some(EnvelopeHit::AttackLength)
        );
        assert_eq!(
            hit_test(&laid, laid.release_end.0, laid.release_end.1),
            Some(EnvelopeHit::ReleaseLength)
        );
        assert_eq!(hit_test(&laid, 0.0, 0.0), None);
    }

    #[test]
    fn length_drag_round_trips_near_layout_edges() {
        let values = defaults();
        let laid = layout(viewport(), values);
        let attack = drag_length(
            EnvelopeHit::AttackLength,
            values,
            viewport(),
            laid.attack_end.0,
        );
        let hold = drag_length(EnvelopeHit::HoldLength, values, viewport(), laid.hold_end.0);
        let release = drag_length(
            EnvelopeHit::ReleaseLength,
            values,
            viewport(),
            laid.release_end.0,
        );
        assert!((attack - values.attack_ms).abs() / values.attack_ms < 0.08);
        assert!(hold.abs() < 8.0);
        assert!((release - values.release_ms).abs() / values.release_ms < 0.08);
    }

    #[test]
    fn dragging_length_right_makes_that_stage_longer() {
        let values = defaults();
        let laid = layout(viewport(), values);
        let slower_attack = drag_length(
            EnvelopeHit::AttackLength,
            values,
            viewport(),
            laid.attack_end.0 + 30.0,
        );
        let opened_hold = drag_length(
            EnvelopeHit::HoldLength,
            values,
            viewport(),
            laid.hold_end.0 + 40.0,
        );
        let slower_release = drag_length(
            EnvelopeHit::ReleaseLength,
            values,
            viewport(),
            laid.release_end.0 + 20.0,
        );
        assert!(slower_attack > values.attack_ms);
        assert!(opened_hold > values.hold_ms);
        assert!(slower_release > values.release_ms);
    }

    #[test]
    fn curve_drag_follows_segment_direction() {
        let laid = layout(viewport(), defaults());
        let aggressive = drag_curve(EnvelopeHit::AttackCurve, &laid, laid.hold_y - 8.0);
        let smooth = drag_curve(EnvelopeHit::AttackCurve, &laid, laid.rest_y + 8.0);
        assert!(aggressive < 0.0);
        assert!(smooth > 0.0);
        let release_fast = drag_curve(EnvelopeHit::ReleaseCurve, &laid, laid.rest_y + 8.0);
        let release_slow = drag_curve(EnvelopeHit::ReleaseCurve, &laid, laid.hold_y - 8.0);
        assert!(release_fast < 0.0);
        assert!(release_slow > 0.0);
    }

    #[test]
    fn path_starts_and_ends_on_rest() {
        let values = EnvelopeValues {
            attack_curve: -1.0,
            release_curve: 1.25,
            ..defaults()
        };
        let laid = layout(viewport(), values);
        let points = path(&laid, values);
        assert_eq!(points.first().copied(), Some(laid.start));
        assert_eq!(points.last().copied(), Some(laid.release_end));
        assert!(points.iter().any(|p| *p == laid.hold_end));
    }
}
