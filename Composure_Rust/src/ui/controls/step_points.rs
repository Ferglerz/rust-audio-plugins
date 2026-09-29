//! Step-point snap tables from JSFX `04_UI_Controls/02_step_points.jsfx-inc`.

use nih_plug_vizia::vizia::prelude::EventContext;
use nih_plug_vizia::widgets::param_base::ParamWidgetBase;

/// Which JSFX step-point table applies to a control (avoids `&'static` in widget signatures).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepSet {
    None,
    Attack,
    Release,
    HpFreq,
    LpFreq,
    LookaheadMs,
    RmsMs,
    Percent0_100,
    Percent0_200,
}

impl StepSet {
    pub fn steps(self) -> Option<&'static [f64]> {
        match self {
            Self::None => None,
            Self::Attack => Some(ATTACK),
            Self::Release => Some(RELEASE),
            Self::HpFreq => Some(HP_FREQ),
            Self::LpFreq => Some(LP_FREQ),
            Self::LookaheadMs => Some(LOOKAHEAD_MS),
            Self::RmsMs => Some(RMS_MS),
            Self::Percent0_100 => Some(PERCENT_0_100),
            Self::Percent0_200 => Some(PERCENT_0_200),
        }
    }
}

/// Attack slider (JSFX slider 1): dB/s, slow→fast left-to-right in UI.
pub const ATTACK: &[f64] = &[
    100_000.0, 50_000.0, 33_333.0, 25_000.0, 20_000.0, 16_667.0, 14_286.0, 12_500.0, 11_111.0,
    10_000.0, 5_000.0, 3_333.0, 2_500.0, 2_000.0, 1_667.0, 1_429.0, 1_250.0, 1_111.0, 1_000.0,
    667.0, 500.0, 400.0, 333.0, 286.0, 250.0, 222.0, 200.0, 167.0, 143.0, 125.0, 111.0, 100.0,
    80.0, 67.0, 57.0, 50.0, 44.0, 40.0, 33.0, 29.0, 25.0, 22.0, 20.0, 17.0, 14.0, 12.0, 11.0, 10.0,
    8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0,
];

/// Release slider (JSFX slider 3): dB/s.
pub const RELEASE: &[f64] = &[
    250.0, 232.0, 214.0, 199.0, 184.0, 170.0, 158.0, 146.0, 135.0, 125.0, 116.0, 108.0, 100.0,
    92.0, 85.0, 79.0, 73.0, 68.0, 63.0, 58.0, 54.0, 50.0, 46.0, 43.0, 40.0, 37.0, 34.0, 32.0, 29.0,
    27.0, 25.0, 23.0, 22.0, 20.0, 18.0, 17.0, 16.0, 15.0, 14.0, 13.0, 12.0, 11.0, 10.0, 9.0, 8.0,
    7.0, 6.0, 5.0, 4.0, 3.0, 2.0,
];

pub const HP_FREQ: &[f64] = &[
    0.0, 20.0, 30.0, 40.0, 60.0, 80.0, 100.0, 120.0, 150.0, 200.0, 250.0, 300.0, 350.0, 400.0,
    500.0, 600.0, 750.0, 1000.0, 1250.0, 1500.0, 1750.0, 2000.0, 2500.0, 3000.0, 3500.0, 4000.0,
    5000.0, 6000.0,
];

pub const LP_FREQ: &[f64] = &[
    20.0, 30.0, 40.0, 60.0, 80.0, 100.0, 120.0, 150.0, 200.0, 250.0, 300.0, 350.0, 400.0, 500.0,
    600.0, 750.0, 1000.0, 1250.0, 1500.0, 1750.0, 2000.0, 2500.0, 3000.0, 3500.0, 4000.0, 5000.0,
    6000.0, 8000.0, 10_000.0, 12_000.0, 14_000.0, 0.0,
];

pub const LOOKAHEAD_MS: &[f64] = &[
    0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 15.0, 20.0, 25.0, 30.0, 40.0, 50.0,
    60.0, 70.0, 80.0, 90.0, 100.0, 150.0, 200.0, 250.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0,
    900.0, 1000.0, 1250.0, 1500.0, 1750.0, 2000.0,
];

/// RMS window uses the same increasing millisecond resolution as envelope time readouts.
pub const RMS_MS: &[f64] = &[
    0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0,
    45.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0, 150.0, 200.0, 250.0, 300.0, 350.0, 400.0, 450.0,
    500.0, 600.0, 700.0, 800.0, 900.0, 1000.0,
];

pub const PERCENT_0_100: &[f64] = &[
    0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0,
];

pub const PERCENT_0_200: &[f64] = &[
    0.0, 20.0, 40.0, 60.0, 80.0, 100.0, 120.0, 140.0, 160.0, 180.0, 200.0,
];

/// Millisecond resolution grows with duration, as in Tape Stop's time control.
fn time_step_ms(ms: f32) -> f32 {
    if ms < 1.0 {
        0.1
    } else if ms < 10.0 {
        1.0
    } else if ms < 50.0 {
        5.0
    } else if ms < 100.0 {
        10.0
    } else if ms < 500.0 {
        50.0
    } else {
        100.0
    }
}

pub fn snap_time_ms(ms: f32) -> f32 {
    let step = time_step_ms(ms);
    (ms / step).round() * step
}

/// Move to the next duration grid point so wheel gestures cannot stick between steps.
pub fn next_time_ms(ms: f32, direction: f32) -> f32 {
    let step = time_step_ms(if direction < 0.0 {
        (ms - 0.01).max(0.0)
    } else {
        ms
    });
    let index = ms / step;
    let next = if direction > 0.0 {
        (index + 0.0001).floor() + 1.0
    } else {
        (index - 0.0001).ceil() - 1.0
    };
    (next * step).max(0.0)
}

pub fn find_step_index(value: f64, steps: &[f64]) -> usize {
    steps
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (value - **a)
                .abs()
                .partial_cmp(&(value - **b).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0)
}

pub fn snap_to_nearest_step(value: f64, steps: &[f64]) -> f64 {
    steps
        .get(find_step_index(value, steps))
        .copied()
        .unwrap_or(value)
}

/// JSFX `calculate_slider_normalized_pos_and_display_value`: step index → 0–1 UI position.
pub fn value_to_step_norm(value: f64, steps: &[f64]) -> f32 {
    let count = steps.len();
    if count <= 1 {
        return 0.5;
    }
    find_step_index(value, steps) as f32 / (count - 1) as f32
}

/// Inverse of [`value_to_step_norm`] (matches JSFX `update_slider_value` index selection).
pub fn step_norm_to_value(norm: f32, steps: &[f64]) -> f64 {
    let count = steps.len();
    if count == 0 {
        return 0.0;
    }
    if count == 1 {
        return steps[0];
    }
    let index = (norm.clamp(0.0, 1.0) * count as f32).floor() as usize;
    steps[index.min(count - 1)]
}

/// Snap the active parameter to the nearest JSFX step-point on drag release.
pub fn snap_param(cx: &mut EventContext, param_base: &ParamWidgetBase, set: StepSet) {
    if let Some(steps) = set.steps() {
        let plain = param_base.unmodulated_plain_value() as f64;
        let snapped = snap_to_nearest_step(plain, steps);
        let norm = param_base.preview_normalized(snapped as f32);
        param_base.set_normalized_value(cx, norm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_steps_match_the_envelope_time_grid_across_the_window_range() {
        assert_eq!(RMS_MS.first(), Some(&0.0));
        assert_eq!(RMS_MS.last(), Some(&1000.0));
        // RMS uses whole milliseconds; zero is the special instantaneous/peak setting.
        assert_eq!(RMS_MS[1], 1.0);
        for pair in RMS_MS[1..].windows(2) {
            assert_eq!(next_time_ms(pair[0] as f32, 1.0), pair[1] as f32);
            assert_eq!(next_time_ms(pair[1] as f32, -1.0), pair[0] as f32);
        }
        for (plain, expected) in [
            (7.4, 7.0),
            (23.0, 25.0),
            (78.0, 80.0),
            (247.0, 250.0),
            (960.0, 1000.0),
        ] {
            assert_eq!(snap_to_nearest_step(plain, RMS_MS), expected);
        }
    }

    #[test]
    fn duration_snapping_grows_with_time() {
        for (input, expected) in [
            (0.34, 0.3),
            (7.4, 7.0),
            (23.0, 25.0),
            (78.0, 80.0),
            (247.0, 250.0),
            (960.0, 1000.0),
            (4480.0, 4500.0),
        ] {
            assert!((snap_time_ms(input) - expected).abs() < 0.001);
        }
        for ms in [1.0, 10.0, 50.0, 100.0, 500.0, 1000.0, 4500.0] {
            assert!(next_time_ms(ms, 1.0) > ms);
            assert!(next_time_ms(ms, -1.0) < ms);
        }
    }

    #[test]
    fn snaps_to_nearest_attack_step() {
        assert_eq!(snap_to_nearest_step(999.0, ATTACK), 1000.0);
        assert_eq!(snap_to_nearest_step(2.5, ATTACK), 3.0);
        assert_eq!(snap_to_nearest_step(1_500_000.0, ATTACK), 100_000.0);
    }

    #[test]
    fn snaps_lookahead_ms() {
        assert_eq!(snap_to_nearest_step(12.0, LOOKAHEAD_MS), 10.0);
        assert_eq!(snap_to_nearest_step(17.0, LOOKAHEAD_MS), 15.0);
    }

    #[test]
    fn release_slow_on_right() {
        assert!((value_to_step_norm(250.0, RELEASE) - 0.0).abs() < f32::EPSILON);
        assert!((value_to_step_norm(2.0, RELEASE) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lp_freq_step_norm_order() {
        assert!(value_to_step_norm(4000.0, LP_FREQ) < value_to_step_norm(14_000.0, LP_FREQ));
        assert!((value_to_step_norm(0.0, LP_FREQ) - 1.0).abs() < f32::EPSILON);
        assert!((value_to_step_norm(20.0, LP_FREQ) - 0.0).abs() < f32::EPSILON);
    }
}
