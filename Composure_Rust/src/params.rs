//! Plugin parameter definitions.
//!
//! Maps all JSFX sliders from Composure.jsfx to nih-plug parameter types.

use std::sync::{atomic::AtomicU32, Arc};

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;

pub use crate::detector_eq::DetectorShape;
use crate::detector_eq::{DetectorBandParams, EQ_BANDS};
use crate::dsp::envelope::defaults as env_defaults;
use crate::graph_store::GraphStore;

pub(crate) fn quantize_lookahead_ms(ms: f32) -> f32 {
    let ms = ms.clamp(0.0, 2000.0);
    if ms < 1.0 {
        // Tolerate only float round-trip noise at an exact decimal boundary.
        ((ms * 10.0 + 0.00001).floor() / 10.0).min(0.9)
    } else {
        ms.round()
    }
}

fn format_lookahead_ms(ms: f32) -> String {
    let ms = quantize_lookahead_ms(ms);
    if ms < 1.0 {
        format!("{ms:.1}")
    } else {
        format!("{ms:.0}")
    }
}

fn format_envelope_curve(value: f32) -> String {
    format!("{:.1}", value * 50.0)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn parse_envelope_curve(text: &str) -> Option<f32> {
    text.trim()
        .trim_end_matches('%')
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
        .map(|value| value / 50.0)
}

const ENVELOPE_TIME_NUMERATOR: f32 = 10_000.0;
const ATTACK_MAX_DB_PER_SEC: f32 = 100_000.0; // 100 µs; roughly five samples at 48 kHz.

fn format_envelope_time(rate_db_per_sec: f32) -> String {
    let ms = ENVELOPE_TIME_NUMERATOR / rate_db_per_sec;
    if ms < 1.0 {
        format!("{:.0} µs", ms * 1000.0)
    } else if ms < 10.0 {
        format!("{ms:.2} ms")
    } else if ms < 100.0 {
        format!("{ms:.1} ms")
    } else if ms < 1000.0 {
        format!("{ms:.0} ms")
    } else {
        format!("{:.2} s", ms / 1000.0)
    }
}

fn parse_envelope_time(text: &str) -> Option<f32> {
    let text = text.trim().to_lowercase().replace('μ', "µ");
    if let Some(rate) = text.strip_suffix("db/s") {
        return rate
            .trim()
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite() && *v > 0.0);
    }
    let (number, scale) =
        if let Some(value) = text.strip_suffix("µs").or_else(|| text.strip_suffix("us")) {
            (value, 0.001)
        } else if let Some(value) = text.strip_suffix("ms") {
            (value, 1.0)
        } else if let Some(value) = text.strip_suffix('s') {
            (value, 1000.0)
        } else {
            (text.as_str(), 1.0)
        };
    let ms = number.trim().parse::<f32>().ok()? * scale;
    let rate = ENVELOPE_TIME_NUMERATOR / ms;
    (ms.is_finite() && ms > 0.0 && rate.is_finite()).then_some(rate)
}

/// Detection mode: feedforward uses the input signal, feedback uses the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum DetectionMode {
    #[name = "Feedback"]
    Feedback,
    #[name = "Feedforward"]
    Feedforward,
}

/// Harmonic saturation model type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum HarmonicType {
    #[name = "Tube"]
    Tube,
    #[name = "Transformer"]
    Transformer,
}

fn gr_blend_threshold_param(name: &str, default: f32, min: f32, max: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min, max })
        .with_unit(" dB")
        .with_step_size(0.1)
}

fn gr_blend_knee_param(name: &str, default: f32) -> FloatParam {
    FloatParam::new(
        name,
        default,
        FloatRange::Linear {
            min: 0.0,
            max: 12.0,
        },
    )
    .with_unit(" dB")
    .with_step_size(0.1)
}

/// Graph display range mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum GraphRangeMode {
    #[name = "20 dB"]
    Range20,
    #[name = "40 dB"]
    Range40,
    #[name = "60 dB"]
    Range60,
    #[name = "6 dB"]
    Range6,
    #[name = "12 dB"]
    Range12,
    #[name = "18 dB"]
    Range18,
    #[name = "24 dB"]
    Range24,
    #[name = "30 dB"]
    Range30,
    #[name = "36 dB"]
    Range36,
    #[name = "42 dB"]
    Range42,
    #[name = "48 dB"]
    Range48,
    #[name = "54 dB"]
    Range54,
    #[name = "66 dB"]
    Range66,
    #[name = "72 dB"]
    Range72,
}

impl GraphRangeMode {
    /// Damian ranges; legacy 20/40 dB variants retain their saved indices.
    pub const PLEASANT: [Self; 12] = [
        Self::Range6,
        Self::Range12,
        Self::Range18,
        Self::Range24,
        Self::Range30,
        Self::Range36,
        Self::Range42,
        Self::Range48,
        Self::Range54,
        Self::Range60,
        Self::Range66,
        Self::Range72,
    ];

    pub const ALL: [Self; 14] = [
        Self::Range20,
        Self::Range40,
        Self::Range60,
        Self::Range6,
        Self::Range12,
        Self::Range18,
        Self::Range24,
        Self::Range30,
        Self::Range36,
        Self::Range42,
        Self::Range48,
        Self::Range54,
        Self::Range66,
        Self::Range72,
    ];
    pub fn range_db(self) -> f64 {
        match self {
            Self::Range20 => 20.0,
            Self::Range40 => 40.0,
            Self::Range60 => 60.0,
            Self::Range6 => 6.0,
            Self::Range12 => 12.0,
            Self::Range18 => 18.0,
            Self::Range24 => 24.0,
            Self::Range30 => 30.0,
            Self::Range36 => 36.0,
            Self::Range42 => 42.0,
            Self::Range48 => 48.0,
            Self::Range54 => 54.0,
            Self::Range66 => 66.0,
            Self::Range72 => 72.0,
        }
    }
}

/// All Composure parameters, derived from JSFX slider definitions.
#[derive(Params)]
pub struct ComposureParams {
    // =========================================================================
    // GROUP 0: ENVELOPE (JSFX sliders 1–6)
    // =========================================================================
    /// Attack rate in dB/s. Higher = faster attack.
    #[id = "attack"]
    pub attack: FloatParam,

    /// Attack curve shaping. Negative = aggressive, positive = smooth S-curve.
    #[id = "attack_curve"]
    pub attack_curve: FloatParam,

    /// Release rate in dB/s. Higher = faster release.
    #[id = "release"]
    pub release: FloatParam,

    /// Release curve shaping. Negative = fast-then-slow, positive = slow-then-fast.
    #[id = "release_curve"]
    pub release_curve: FloatParam,

    /// Hold time in milliseconds.
    #[id = "hold_ms"]
    pub hold_ms: FloatParam,

    /// Compression strength as percentage (0–400%).
    #[id = "strength"]
    pub strength: FloatParam,

    // =========================================================================
    // GROUP 1: INPUT/DETECTION (JSFX sliders 7–15)
    // =========================================================================
    /// RMS detection window in milliseconds. 0 = peak mode.
    #[id = "rms_size_ms"]
    pub rms_size_ms: FloatParam,

    /// Adaptive RMS detection on/off.
    #[id = "rms_normalization"]
    pub rms_normalization: BoolParam,

    /// Detection mode: feedback or feedforward.
    #[id = "detection_mode"]
    pub detection_mode: EnumParam<DetectionMode>,

    /// Lookahead delay in milliseconds.
    #[id = "lookahead_ms"]
    pub lookahead_ms: FloatParam,

    /// Use external sidechain input.
    #[id = "use_sidechain"]
    pub use_sidechain: BoolParam,

    /// Hear filtered detection signal while dragging HP/LP (JSFX slider 12).
    #[id = "sc_adjust_preview"]
    pub sc_adjust_preview: BoolParam,

    /// High-pass filter frequency on detection signal. 0 = off.
    #[id = "hp_freq"]
    pub hp_freq: FloatParam,

    /// Low-pass filter frequency on detection signal. 0 = off.
    #[id = "lp_freq"]
    pub lp_freq: FloatParam,

    /// Additional static detector EQ bands. Legacy HP/LP parameters remain intact.
    #[nested(array, group = "Detector EQ")]
    pub detector_eq: [DetectorBandParams; EQ_BANDS],

    /// Input level offset in dB.
    #[id = "input_offset_db"]
    pub input_offset_db: FloatParam,

    // =========================================================================
    // GROUP 2: HARMONICS (JSFX sliders 16–21)
    // =========================================================================
    #[id = "harmonics_on"]
    pub harmonics_on: BoolParam,

    /// Harmonic saturation model: Tube or Transformer.
    #[id = "harmonic_type"]
    pub harmonic_type: EnumParam<HarmonicType>,

    /// Harmonic drive amount (0–100).
    #[id = "harmonic_drive"]
    pub harmonic_drive: FloatParam,

    /// Harmonic wet/dry mix (0–100%).
    #[id = "harmonic_mix"]
    pub harmonic_mix: FloatParam,

    /// Even harmonic boost (0–200%).
    #[id = "harmonic_even_boost"]
    pub harmonic_even_boost: FloatParam,

    /// Odd harmonic boost (0–200%).
    #[id = "harmonic_odd_boost"]
    pub harmonic_odd_boost: FloatParam,

    /// Makeup gain in dB.
    #[id = "makeup_gain_db"]
    pub makeup_gain_db: FloatParam,

    // =========================================================================
    // GROUP 3: PROGRAM RELEASE & SETTINGS (JSFX sliders 22–27)
    // =========================================================================
    /// Enable input-level program dependence.
    #[id = "program_input_enable"]
    pub program_input_enable: BoolParam,

    /// Enable gain-reduction program dependence.
    #[id = "program_gr_enable"]
    pub program_gr_enable: BoolParam,

    /// Inverse program release behavior.
    #[id = "prog_release_inverse"]
    pub prog_release_inverse: BoolParam,

    /// Program release blend (0=fixed, 100=fully program-dependent).
    #[id = "prog_release_blend"]
    pub prog_release_blend: FloatParam,

    /// Detector-rate influence scaled by program Blend; negative amounts invert its response.
    #[id = "input_rate_amount"]
    pub input_rate_amount: FloatParam,

    /// Harmonic modulation amount (0.0–1.0).
    #[id = "harmonic_amount"]
    pub harmonic_amount: FloatParam,

    /// Rate-of-change sensitivity in dB.
    #[id = "rate_change_sensitivity_db"]
    pub rate_change_sensitivity_db: FloatParam,

    // =========================================================================
    // GROUP 4: THRESHOLDS & ADVANCED (JSFX sliders 28–36)
    // =========================================================================
    /// Input level threshold for input-dependent release (dB).
    #[id = "input_level_threshold_db"]
    pub input_level_threshold_db: FloatParam,

    /// Secondary threshold for input-dependent knee (dB).
    #[id = "input_level_threshold_2_db"]
    pub input_level_threshold_2_db: FloatParam,

    /// GR-dependent threshold for reduction (dB).
    #[id = "gr_blend_threshold_reduction_db"]
    pub gr_blend_threshold_reduction_db: FloatParam,

    /// GR-dependent threshold reduction knee (dB).
    #[id = "gr_blend_threshold_reduction_knee_db"]
    pub gr_blend_threshold_reduction_knee_db: FloatParam,

    /// GR-dependent threshold for addition (dB).
    #[id = "gr_blend_threshold_addition_db"]
    pub gr_blend_threshold_addition_db: FloatParam,

    /// GR-dependent threshold addition knee (dB).
    #[id = "gr_blend_threshold_addition_knee_db"]
    pub gr_blend_threshold_addition_knee_db: FloatParam,

    /// Rate-of-change threshold modifier (0.5–4.0).
    #[id = "rate_change_threshold_modifier"]
    pub rate_change_threshold_modifier: FloatParam,

    /// Graph display range mode.
    #[id = "graph_range_mode"]
    pub graph_range_mode: EnumParam<GraphRangeMode>,

    /// Mid/Side processing mode.
    #[id = "mid_side_mode"]
    pub mid_side_mode: BoolParam,

    // =========================================================================
    // PERSISTENT STATE (non-parameter)
    // =========================================================================
    /// Editor window size and scale state.
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    /// Per-instance artwork width, shared with the editor sizing callback.
    pub editor_width: Arc<AtomicU32>,

    /// Published compression curve (RCU snapshot; persisted as graph points JSON).
    #[persist = "graph_points"]
    pub graph_store: GraphStore,
}

impl ComposureParams {
    pub fn program_input_enabled(&self) -> bool {
        self.program_input_enable.value()
    }

    pub fn program_gr_enabled(&self) -> bool {
        self.program_gr_enable.value()
    }
}

impl Default for ComposureParams {
    fn default() -> Self {
        let editor_width = Arc::new(AtomicU32::new(crate::ui::preferred_editor_width()));
        let graph_store = GraphStore::default();
        graph_store.select_range(GraphRangeMode::Range48);
        Self {
            // === ENVELOPE ===
            attack: FloatParam::new(
                "Attack",
                1000.0,
                FloatRange::Skewed {
                    min: 2.0,
                    max: ATTACK_MAX_DB_PER_SEC,
                    factor: FloatRange::skew_factor(-2.5),
                },
            )
            .with_value_to_string(Arc::new(format_envelope_time))
            .with_string_to_value(Arc::new(parse_envelope_time)),

            attack_curve: FloatParam::new(
                "Attack Curve",
                0.0,
                FloatRange::Linear {
                    min: -2.0,
                    max: 2.0,
                },
            )
            .with_step_size(0.25)
            .with_unit("%")
            .with_value_to_string(Arc::new(format_envelope_curve))
            .with_string_to_value(Arc::new(parse_envelope_curve)),

            release: FloatParam::new(
                "Release",
                100.0,
                FloatRange::Skewed {
                    min: 2.0,
                    max: 250.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_value_to_string(Arc::new(format_envelope_time))
            .with_string_to_value(Arc::new(parse_envelope_time)),

            release_curve: FloatParam::new(
                "Release Curve",
                0.0,
                FloatRange::Linear {
                    min: -2.0,
                    max: 2.0,
                },
            )
            .with_step_size(0.25)
            .with_unit("%")
            .with_value_to_string(Arc::new(format_envelope_curve))
            .with_string_to_value(Arc::new(parse_envelope_curve)),

            hold_ms: FloatParam::new(
                "Hold",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1000.0,
                },
            )
            .with_unit(" ms")
            .with_step_size(1.0),

            strength: FloatParam::new(
                "Strength",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 400.0,
                },
            )
            .with_unit("%")
            .with_step_size(5.0),

            // === DETECTION ===
            rms_size_ms: FloatParam::new(
                "RMS Window",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 1000.0,
                },
            )
            .with_unit(" ms")
            .with_step_size(1.0),

            rms_normalization: BoolParam::new("Adaptive", false),

            detection_mode: EnumParam::new("Detection Mode", DetectionMode::Feedforward),

            lookahead_ms: FloatParam::new(
                "Lookahead",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 2000.0,
                },
            )
            .with_unit(" ms")
            .with_value_to_string(Arc::new(format_lookahead_ms)),

            use_sidechain: BoolParam::new("SC", false),

            sc_adjust_preview: BoolParam::new("L", false),

            hp_freq: FloatParam::new(
                "High Pass",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 6000.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" Hz")
            .with_step_size(1.0),

            lp_freq: FloatParam::new(
                "Low Pass",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 14000.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" Hz")
            .with_step_size(1.0),

            input_offset_db: FloatParam::new(
                "Input Offset",
                0.0,
                FloatRange::Linear {
                    min: -30.0,
                    max: 30.0,
                },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            // === HARMONICS ===
            harmonics_on: BoolParam::new("Harmonics", true),
            harmonic_type: EnumParam::new("Harmonic Type", HarmonicType::Tube),

            harmonic_drive: FloatParam::new(
                "Drive",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_step_size(1.0),

            harmonic_mix: FloatParam::new(
                "Harmonic Mix",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit("%")
            .with_step_size(1.0),

            harmonic_even_boost: FloatParam::new(
                "Even",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 200.0,
                },
            )
            .with_step_size(1.0),

            harmonic_odd_boost: FloatParam::new(
                "Odd",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 200.0,
                },
            )
            .with_step_size(1.0),

            makeup_gain_db: FloatParam::new(
                "Makeup Gain",
                0.0,
                FloatRange::Linear {
                    min: -20.0,
                    max: 20.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            // === PROGRAM RELEASE ===
            program_input_enable: BoolParam::new("Program Input", true),
            program_gr_enable: BoolParam::new("Program GR", false),

            prog_release_inverse: BoolParam::new("Program Release Inverse", false),

            prog_release_blend: FloatParam::new(
                "Program Release Blend",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_step_size(1.0),

            input_rate_amount: FloatParam::new(
                "Input Rate",
                0.0,
                FloatRange::Linear {
                    min: -5.0,
                    max: 5.0,
                },
            )
            .with_unit("x")
            .with_step_size(0.1),

            harmonic_amount: FloatParam::new(
                "Harmonic Amount",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_step_size(0.01),

            rate_change_sensitivity_db: FloatParam::new(
                "Rate Change Sensitivity",
                env_defaults::RATE_CHANGE_SENSITIVITY_DB as f32,
                FloatRange::Linear {
                    min: 0.5,
                    max: 20.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            // === ADVANCED ===
            input_level_threshold_db: FloatParam::new(
                "Input Level Threshold",
                env_defaults::INPUT_LEVEL_THRESHOLD_DB as f32,
                FloatRange::Linear {
                    min: -80.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            input_level_threshold_2_db: FloatParam::new(
                "Input Dep. Knee Threshold",
                env_defaults::INPUT_LEVEL_THRESHOLD_2_DB as f32,
                FloatRange::Linear {
                    min: -80.0,
                    max: 0.0,
                },
            )
            .with_unit(" dB")
            .with_step_size(0.1),

            gr_blend_threshold_reduction_db: gr_blend_threshold_param(
                "GR Threshold Cut",
                env_defaults::GR_BLEND_THRESHOLD_DB as f32,
                1.0,
                24.0,
            ),

            gr_blend_threshold_reduction_knee_db: gr_blend_knee_param(
                "GR Threshold Cut Knee",
                env_defaults::GR_BLEND_THRESHOLD_KNEE_DB as f32,
            ),

            gr_blend_threshold_addition_db: gr_blend_threshold_param(
                "GR Threshold Boost",
                env_defaults::GR_BLEND_THRESHOLD_DB as f32,
                1.0,
                24.0,
            ),

            gr_blend_threshold_addition_knee_db: gr_blend_knee_param(
                "GR Threshold Boost Knee",
                env_defaults::GR_BLEND_THRESHOLD_KNEE_DB as f32,
            ),

            rate_change_threshold_modifier: FloatParam::new(
                "Rate Change Threshold Modifier",
                env_defaults::RATE_CHANGE_THRESHOLD_MODIFIER as f32,
                FloatRange::Linear { min: 0.5, max: 4.0 },
            )
            .with_step_size(0.1),

            graph_range_mode: EnumParam::new("Graph dB Range", GraphRangeMode::Range48),

            mid_side_mode: BoolParam::new("Mid/Side", false),

            editor_state: crate::ui::default_editor_state(editor_width.clone()),
            editor_width,
            graph_store,
            detector_eq: std::array::from_fn(|_| DetectorBandParams::default()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookahead_uses_integers_above_one_and_truncated_tenths_below() {
        for (ms, value, text) in [
            (0.19, 0.1, "0.1"),
            (0.99, 0.9, "0.9"),
            (0.9999999, 0.9, "0.9"),
            (1.0, 1.0, "1"),
            (10.6, 11.0, "11"),
            (2001.0, 2000.0, "2000"),
        ] {
            assert_eq!(quantize_lookahead_ms(ms), value);
            assert_eq!(format_lookahead_ms(ms), text);
        }
        let params = ComposureParams::default();
        for ms in [0.1, 0.3, 0.7, 0.9] {
            let round_trip = params
                .lookahead_ms
                .preview_plain(params.lookahead_ms.preview_normalized(ms));
            assert_eq!(quantize_lookahead_ms(round_trip), ms);
        }
    }

    #[test]
    fn lookahead_preserves_fractional_milliseconds_for_linear_graph_dragging() {
        let params = ComposureParams::default();
        for ms in [0.1, 0.5, 10.25, 999.75, 2000.0] {
            let norm = params.lookahead_ms.preview_normalized(ms);
            assert!((params.lookahead_ms.preview_plain(norm) - ms).abs() < 0.001);
        }
    }

    #[test]
    fn envelope_curve_percentages_round_trip_without_changing_dsp_values() {
        let params = ComposureParams::default();
        for param in [&params.attack_curve, &params.release_curve] {
            for (plain, text) in [
                (-2.0, "-100%"),
                (-0.25, "-12.5%"),
                (0.0, "0%"),
                (0.25, "12.5%"),
                (2.0, "100%"),
            ] {
                let norm = param.preview_normalized(plain);
                assert_eq!(param.normalized_value_to_string(norm, true), text);
                assert_eq!(param.string_to_normalized_value(text), Some(norm));
            }
            assert_eq!(
                param.string_to_normalized_value("50"),
                Some(param.preview_normalized(1.0))
            );
        }
    }

    #[test]
    fn offset_readout_limits_decimals_without_quantizing_the_parameter() {
        let params = ComposureParams::default();
        let offset = &params.input_offset_db;
        for (value, expected) in [(1.234567, "1.2 dB"), (-4.56789, "-4.6 dB"), (0.0, "0.0 dB")] {
            let norm = offset.preview_normalized(value);
            assert_eq!(offset.normalized_value_to_string(norm, true), expected);
            assert!((offset.preview_plain(norm) - value).abs() < 0.00001);
        }
    }

    #[test]
    fn new_instances_start_at_48_db_with_evenly_distributed_unity_points() {
        let params = ComposureParams::default();
        assert_eq!(params.graph_range_mode.value(), GraphRangeMode::Range48);
        let snapshot = params.graph_store.load();
        let graph = &snapshot.graph;
        assert_eq!(graph.min_db, -48.0);
        assert_eq!(graph.max_db, 0.0);
        for i in 1..graph.num_points - 1 {
            let expected = -48.0 + 48.0 * i as f64 / (graph.num_points - 1) as f64;
            assert!((graph.get_point_x(i) - expected).abs() < 0.001);
            assert_eq!(graph.get_point_x(i), graph.get_point_y(i));
        }
    }

    #[test]
    fn envelope_rates_show_editable_time_without_changing_the_dsp_unit() {
        let params = ComposureParams::default();
        assert_eq!(params.attack.preview_plain(1.0), ATTACK_MAX_DB_PER_SEC);
        assert_eq!(format_envelope_time(ATTACK_MAX_DB_PER_SEC), "100 µs");
        assert_eq!(format_envelope_time(1000.0), "10.0 ms");
        assert_eq!(format_envelope_time(100.0), "100 ms");
        assert_eq!(parse_envelope_time("100 us"), Some(100_000.0));
        assert_eq!(parse_envelope_time("100 µs"), Some(100_000.0));
        assert_eq!(parse_envelope_time("10 ms"), Some(1000.0));
        assert_eq!(parse_envelope_time("100 ms"), Some(100.0));
        assert_eq!(parse_envelope_time("1 s"), Some(10.0));
        assert_eq!(parse_envelope_time("1000 dB/s"), Some(1000.0));
        assert_eq!(parse_envelope_time("0 ms"), None);
    }

    #[test]
    fn detector_eq_host_ids_are_unique_and_keep_legacy_cuts() {
        let params = ComposureParams::default();
        let map = params.param_map();
        let ids: std::collections::HashSet<_> = map.iter().map(|(id, _, _)| id).collect();
        assert_eq!(ids.len(), map.len());
        assert!(ids.contains(&"hp_freq".to_owned()));
        assert!(ids.contains(&"lp_freq".to_owned()));
        for slot in 1..=EQ_BANDS {
            for field in ["active", "enabled", "shape", "freq", "gain", "q"] {
                let id = format!("det_eq_{field}_{slot}");
                let (_, _, group) = map.iter().find(|(key, _, _)| *key == id).unwrap();
                assert_eq!(group, &format!("Detector EQ {slot}"));
            }
        }
        assert!(params.detector_eq.iter().all(|band| !band.active.value()));
        assert!(params.detector_eq.iter().all(|band| band.enabled.value()));
    }

    #[test]
    fn harmonic_type_has_two_variants() {
        let params = ComposureParams::default();
        assert_eq!(params.harmonic_type.value(), HarmonicType::Tube);
        assert_eq!(
            params.harmonic_type.preview_plain(1.0),
            HarmonicType::Transformer
        );
    }

    #[test]
    fn graph_state_persist_roundtrip() {
        let params = ComposureParams::default();
        params.graph_store.mutate_and_publish(|graph| {
            graph.graph_points_mut()[3] -= 12.0;
            graph.update_corner_points();
            graph.invalidate();
        });

        let serialized = params.serialize_fields();
        assert!(serialized.contains_key("graph_points"));

        let restored = ComposureParams::default();
        restored.deserialize_fields(&serialized);

        let original = params.graph_store.load();
        let loaded = restored.graph_store.load();
        assert!(original.graph.content_eq(&loaded.graph));
    }
}
