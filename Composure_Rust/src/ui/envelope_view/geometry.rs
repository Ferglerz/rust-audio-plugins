use super::*;

pub(super) const WIDTH: f32 = appearance::ENVELOPE_GRAPH_W;
pub(super) const HEIGHT: f32 = appearance::ENVELOPE_GRAPH_H;
pub(super) const LOOK_BAR: (f32, f32, f32, f32) = (12.0, 8.0, WIDTH - 24.0, 30.0);
pub(super) const GRAPH: (f32, f32, f32, f32) = (12.0, 50.0, WIDTH - 24.0, HEIGHT - 96.0);
pub(super) const LABEL_H: f32 = 36.0;

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

pub(super) fn layout(viewport: Viewport, values: EnvelopeValues) -> EnvelopeLayout {
    let values = values.clamped();
    let third = 1.0 / 3.0;
    let rest_y = viewport.y + viewport.height - 24.0;
    let hold_y = viewport.y + 24.0;
    let start = (viewport.x, rest_y);
    let attack_end = (viewport.x + viewport.width * third, hold_y);
    let hold_end = (viewport.x + viewport.width * (2.0 * third), hold_y);
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

fn timing_limits(hit: EnvelopeHit) -> (f32, f32) {
    use pleasant_ui::envelope::{
        ATTACK_MAX_MS, ATTACK_MIN_MS, HOLD_MAX_MS, HOLD_MIN_MS, RELEASE_MAX_MS, RELEASE_MIN_MS,
    };
    match hit {
        EnvelopeHit::AttackLength => (ATTACK_MIN_MS, ATTACK_MAX_MS),
        EnvelopeHit::HoldLength => (HOLD_MIN_MS, HOLD_MAX_MS),
        EnvelopeHit::ReleaseLength => (RELEASE_MIN_MS, RELEASE_MAX_MS),
        _ => unreachable!(),
    }
}

pub(super) fn timing_slider_norm(hit: EnvelopeHit, values: EnvelopeValues) -> f32 {
    let ms = match hit {
        EnvelopeHit::AttackLength => values.attack_ms,
        EnvelopeHit::HoldLength => values.hold_ms,
        EnvelopeHit::ReleaseLength => values.release_ms,
        _ => unreachable!(),
    };
    let (min, max) = timing_limits(hit);
    // The offset gives the hold slider a logarithmic response that includes zero.
    let offset = if min == 0.0 { 1.0 } else { 0.0 };
    let base = min + offset;
    (((ms + offset) / base).ln() / ((max + offset) / base).ln()).clamp(0.0, 1.0)
}

pub(super) fn timing_slider_plain(hit: EnvelopeHit, norm: f32, fine: bool) -> f32 {
    let (min, max) = timing_limits(hit);
    let offset = if min == 0.0 { 1.0 } else { 0.0 };
    let base = min + offset;
    let ms = base * ((max + offset) / base).powf(norm.clamp(0.0, 1.0)) - offset;
    let ms = if fine { ms } else { snap_time_ms(ms) }.clamp(min, max);
    if hit == EnvelopeHit::HoldLength {
        ms
    } else {
        ms_to_rate(ms)
    }
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
