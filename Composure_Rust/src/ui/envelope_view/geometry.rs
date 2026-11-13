use super::*;

pub(super) const WIDTH: f32 = appearance::ENVELOPE_GRAPH_W;
pub(super) const HEIGHT: f32 = appearance::ENVELOPE_GRAPH_H;
pub(super) const LOOK_BAR: (f32, f32, f32, f32) = (12.0, 8.0, WIDTH - 24.0, 30.0);
pub(super) const GRAPH: (f32, f32, f32, f32) = (12.0, 50.0, WIDTH - 24.0, HEIGHT - 96.0);
pub(super) const LABEL_H: f32 = 36.0;

// After normalization this permits a 1/14 share, half the previous 1/7 minimum.
const MIN_STAGE_WEIGHT: f32 = 1.0 / 12.0;

// Lookahead belongs to this editor; shared envelope geometry stays attack/hold/release.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EnvelopeHit {
    Lookahead,
    AttackLength,
    HoldLength,
    ReleaseLength,
    AttackCurve,
    ReleaseCurve,
}

// Duration shares with a smaller touchability floor, then normalize.
// Even a zero hold stays visible and can be dragged open.
pub(super) fn layout(viewport: Viewport, values: EnvelopeValues) -> EnvelopeLayout {
    let values = values.clamped();
    let total = values.attack_ms + values.hold_ms + values.release_ms;
    let weights = [values.attack_ms, values.hold_ms, values.release_ms]
        .map(|ms| (ms / total).max(MIN_STAGE_WEIGHT));
    let sum = weights.iter().sum::<f32>();
    let rest_y = viewport.y + viewport.height - 24.0;
    let hold_y = viewport.y + 24.0;
    let start = (viewport.x, rest_y);
    let attack_end = (viewport.x + viewport.width * weights[0] / sum, hold_y);
    let hold_end = (attack_end.0 + viewport.width * weights[1] / sum, hold_y);
    let release_end = (viewport.x + viewport.width, rest_y);
    EnvelopeLayout {
        viewport,
        rest_y,
        hold_y,
        start,
        attack_end,
        hold_end,
        release_end,
        attack_curve: (
            (start.0 + attack_end.0) * 0.5,
            rest_y + (hold_y - rest_y) * ease(0.5, values.attack_curve),
        ),
        release_curve: (
            (hold_end.0 + release_end.0) * 0.5,
            hold_y + (rest_y - hold_y) * ease(0.5, values.release_curve),
        ),
    }
}

pub(super) fn lookahead_marker_x(ms: f32, laid: &EnvelopeLayout, values: EnvelopeValues) -> f32 {
    let values = values.clamped();
    let time = ms.max(0.0);
    if time <= values.attack_ms {
        return laid.start.0 + time / values.attack_ms * (laid.attack_end.0 - laid.start.0);
    }
    let time = time - values.attack_ms;
    // The zero-time hold is an editing affordance, not a range of lookahead times.
    // Spread release time across that empty space so dragging does not stick at attack.
    if values.hold_ms == 0.0 {
        return laid.attack_end.0
            + (time / values.release_ms).clamp(0.0, 1.0)
                * (laid.release_end.0 - laid.attack_end.0);
    }
    if values.hold_ms > 0.0 && time <= values.hold_ms {
        return laid.attack_end.0 + time / values.hold_ms * (laid.hold_end.0 - laid.attack_end.0);
    }
    let time = time - values.hold_ms;
    laid.hold_end.0
        + (time / values.release_ms).clamp(0.0, 1.0) * (laid.release_end.0 - laid.hold_end.0)
}

// Inverse of the graph's piecewise time scale; the slider keeps its own cubic scale.
pub(super) fn lookahead_graph_ms(x: f32, laid: &EnvelopeLayout, values: EnvelopeValues) -> f32 {
    let values = values.clamped();
    let x = x.clamp(laid.start.0, laid.release_end.0);
    let ms = if x <= laid.attack_end.0 {
        (x - laid.start.0) / (laid.attack_end.0 - laid.start.0) * values.attack_ms
    } else if values.hold_ms == 0.0 {
        values.attack_ms
            + (x - laid.attack_end.0) / (laid.release_end.0 - laid.attack_end.0) * values.release_ms
    } else if x <= laid.hold_end.0 {
        values.attack_ms
            + (x - laid.attack_end.0) / (laid.hold_end.0 - laid.attack_end.0) * values.hold_ms
    } else {
        values.attack_ms
            + values.hold_ms
            + (x - laid.hold_end.0) / (laid.release_end.0 - laid.hold_end.0) * values.release_ms
    };
    ms.clamp(0.0, 2000.0)
}

pub(super) fn lookahead_drag_ms(
    x: f32,
    laid: &EnvelopeLayout,
    values: EnvelopeValues,
    bypass_snap: bool,
) -> f32 {
    let ms = if !bypass_snap && (x - laid.attack_end.0).abs() <= 0.75 {
        values.attack_ms
    } else {
        lookahead_graph_ms(x, laid, values)
    };
    quantize_lookahead_ms(ms)
}

pub(super) fn lookahead_bar_x(ms: f32) -> f32 {
    LOOK_BAR.0 + LOOK_BAR.2 * (ms.clamp(0.0, 2000.0) / 2000.0).cbrt()
}

pub(super) fn lookahead_bar_ms(x: f32) -> f32 {
    2000.0 * ((x - LOOK_BAR.0) / LOOK_BAR.2).clamp(0.0, 1.0).powi(3)
}

pub(super) fn curve_drag(hit: StageHit, laid: &EnvelopeLayout, y: f32) -> f32 {
    let (start, end) = match hit {
        StageHit::AttackCurve => (laid.rest_y, laid.hold_y),
        _ => (laid.hold_y, laid.rest_y),
    };
    curve_from_mid_norm(((y - start) / (end - start)).clamp(0.02, 0.98))
}

pub(super) fn stage_time_drag(hit: EnvelopeHit, values: EnvelopeValues, dx: f32) -> f32 {
    let (start_ms, offset) = match hit {
        EnvelopeHit::AttackLength => (values.attack_ms, 0.0),
        EnvelopeHit::ReleaseLength => (values.release_ms, 0.0),
        _ => (values.hold_ms, 1.0),
    };
    let ms = ((start_ms + offset) * 1.03_f32.powf(dx) - offset).max(0.0);
    match hit {
        EnvelopeHit::AttackLength | EnvelopeHit::ReleaseLength => ms_to_rate(ms),
        _ => ms,
    }
}
