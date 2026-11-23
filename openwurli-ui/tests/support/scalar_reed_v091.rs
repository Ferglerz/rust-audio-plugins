// Frozen scalar v0.9.1 reed, hal0zer0/openwurli c05403b, GPL-3.0-or-later.
//! Modal reed oscillator — 7 damped sinusoidal modes.
//!
//! Each mode uses a quadrature oscillator (sin/cos pair rotated per sample)
//! instead of computing sin(phase) per sample. This eliminates 7 transcendental
//! calls per sample per voice — the dominant CPU cost in v0.1.x.
//!
//! Each mode holds its frequency exactly: there is no random frequency
//! wander. An Ornstein-Uhlenbeck pitch jitter (sigma 0.04 %, ear-voiced, never
//! derived) was removed 2026-09-27 on evidence: the common-mode partial-frequency
//! wander of two real 200As (13 + 72 recordings, read through the same estimator
//! as the model, `tools/partial_wander.py`) is 12-18x below what the jitter
//! produced, at the estimator's noise, and the reed's deterministic
//! amplitude-dependent frequency shift is <= ~0.003 %. What the recordings DO show
//! is a deterministic pitch glide over the decay; that awaits a derived mechanism.

use std::f64::consts::{PI, TAU};

use crate::tables::NUM_MODES;

/// Quadrature renormalization interval. Every 1024 samples, correct the
/// radius of each (s,c) pair to prevent drift from accumulated FP error.
/// Cost: 7 sqrt + 7 div per 1024 samples = 0.014 transcendentals/sample.
const RENORM_INTERVAL: u64 = 1024;

/// Per-mode oscillator state — array-of-structs for sequential access.
///
/// 88 bytes/mode × 7 = 616 bytes. Fits in L1 cache.
struct Mode {
    /// Quadrature sine state (output signal).
    s: f64,
    /// Quadrature cosine state.
    c: f64,
    /// cos(base_phase_inc) — precomputed at note-on.
    cos_inc: f64,
    /// sin(base_phase_inc) — precomputed at note-on.
    sin_inc: f64,
    /// Initial amplitude for this mode.
    amplitude: f64,
    /// Per-sample multiplicative natural decay factor.
    decay_mult: f64,
    /// Current envelope level (starts at 1.0, decays each sample).
    envelope: f64,
    /// Damper rate in nepers/sample (set on note_off).
    damper_rate: f64,
    /// Precomputed exp(-damper_rate) for post-ramp phase.
    damper_mult: f64,
}

pub struct ModalReed {
    modes: [Mode; NUM_MODES],
    sample: u64,
    // Onset ramp
    onset_ramp_samples: u64,
    onset_ramp_inc: f64,
    onset_shape_exp: f64,
    // Damper state
    damper_active: bool,
    damper_ramp_samples: f64,
    damper_release_count: f64,
    damper_ramp_done: bool,
    /// Amplitude-dependent loss on mode 1 (air drag), per sample, per unit of mode-1 envelope:
    /// `envelope *= 1 - drag_k * envelope`. Zero = off. See [`ModalReed::set_air_drag`].
    drag_k: f64,
}

impl ModalReed {
    /// Create a new modal reed oscillator.
    ///
    /// - `fundamental_hz`: fundamental frequency after detuning
    /// - `mode_ratios`: f_n / f_1 for each mode
    /// - `amplitudes`: initial amplitude for each mode (post dwell-filter, post variation)
    /// - `decay_rates_db`: decay rate in dB/s for each mode
    /// - `onset_time_s`: onset ramp duration in seconds (reed mechanical inertia)
    /// - `velocity`: 0.0 (pp) to 1.0 (ff), controls onset ramp shape
    /// - `sample_rate`: audio sample rate in Hz
    pub fn new(
        fundamental_hz: f64,
        mode_ratios: &[f64; NUM_MODES],
        amplitudes: &[f64; NUM_MODES],
        decay_rates_db: &[f64; NUM_MODES],
        onset_time_s: f64,
        velocity: f64,
        sample_rate: f64,
    ) -> Self {
        // Build Mode structs with precomputed quadrature rotation coefficients
        let modes = std::array::from_fn(|i| {
            let freq = fundamental_hz * mode_ratios[i];
            let phase_inc = TAU * freq / sample_rate;
            let alpha_nepers = decay_rates_db[i] / 8.686;
            let decay_per_sample = alpha_nepers / sample_rate;

            Mode {
                s: 0.0,                   // sin(0) = 0
                c: 1.0,                   // cos(0) = 1
                cos_inc: phase_inc.cos(), // precomputed rotation
                sin_inc: phase_inc.sin(), // precomputed rotation
                amplitude: amplitudes[i],
                decay_mult: (-decay_per_sample).exp(),
                envelope: 1.0,
                damper_rate: 0.0,
                damper_mult: 1.0,
            }
        });

        // Onset ramp
        let ramp_samps = (onset_time_s * sample_rate).round() as u64;
        let ramp_inc = if ramp_samps > 0 {
            PI / ramp_samps as f64
        } else {
            0.0
        };

        let onset_shape_exp = 1.0 + (1.0 - velocity);

        Self {
            modes,
            sample: 0,
            onset_ramp_samples: ramp_samps,
            onset_ramp_inc: ramp_inc,
            onset_shape_exp,
            damper_active: false,
            damper_ramp_samples: 0.0,
            damper_release_count: 0.0,
            damper_ramp_done: false,
            drag_k: 0.0,
        }
    }

    /// Start each mode at a given phase (radians) instead of zero: the mode outputs
    /// `amplitude * sin(phase + 2 pi f t)`. Used to start the reed from a prescribed modal state
    /// (see [`crate::voice::Voice::note_on_modal`]); the shipping path never calls it.
    pub fn set_phases(&mut self, phases: &[f64; NUM_MODES]) {
        for (mode, &ph) in self.modes.iter_mut().zip(phases) {
            mode.s = ph.sin();
            mode.c = ph.cos();
        }
    }

    /// Enable amplitude-dependent loss on mode 1: quadratic air drag on the moving reed, which
    /// decays the mode as `dA/dt = -beta |A| A`. `beta_db_per_mm` is the extra decay in dB/s per
    /// mm of swing at the pickup's finger region; `swing_mm` is the swing that one unit of model
    /// displacement stands for there. The mode's current amplitude (initial amplitude x envelope)
    /// sets the loss, so a loud note loses energy faster early and slower once it has decayed.
    pub fn set_air_drag(&mut self, beta_db_per_mm: f64, swing_mm: f64, sample_rate: f64) {
        let beta_nepers = beta_db_per_mm / 8.686;
        self.drag_k = beta_nepers * self.modes[0].amplitude.abs() * swing_mm / sample_rate;
    }

    /// Start the damper (called on note_off).
    ///
    /// Three-phase progressive model: felt progressively contacts the reed,
    /// with higher modes damped more aggressively.
    ///
    /// - `midi_note`: for register-dependent ramp time
    /// - `sample_rate`: for time constant conversion
    /// - `damper_scale`: multiplier on the damper rates, 1.0 = shipping (the per-mode cap holds)
    pub fn start_damper(&mut self, midi_note: u8, sample_rate: f64, damper_scale: f64) {
        // Top 5 keys (MIDI 92-96): no damper pads; they ring as sympathetics on the A series
        // (Doc Wurly's reed-era history, docwurly.com). Natural decay only.
        if midi_note >= 92 {
            return;
        }

        let base_rate =
            (55.0 * 2.0_f64.powf((midi_note as f64 - 60.0) / 24.0)).max(0.5) * damper_scale;
        for (m, mode) in self.modes.iter_mut().enumerate() {
            let factor = (base_rate * 3.0_f64.powi(m as i32)).min(2000.0);
            mode.damper_rate = factor / sample_rate;
            mode.damper_mult = (-mode.damper_rate).exp();
        }

        let ramp_time = if midi_note < 48 {
            0.050
        } else if midi_note < 72 {
            0.025
        } else {
            0.008
        };

        self.damper_ramp_samples = ramp_time * sample_rate;
        self.damper_active = true;
        self.damper_release_count = 0.0;
        self.damper_ramp_done = false;
    }

    /// Render samples into the output buffer (additive, does NOT clear buffer).
    pub fn render(&mut self, output: &mut [f64]) {
        for sample in output.iter_mut() {
            let mut sum = 0.0f64;

            // Advance damper if active
            if self.damper_active {
                self.damper_release_count += 1.0;
                let t = self.damper_release_count;
                let ramp = self.damper_ramp_samples;
                if !self.damper_ramp_done {
                    if t > ramp {
                        self.damper_ramp_done = true;
                    } else {
                        // During ramp: apply instantaneous rate (still needs exp per mode)
                        for mode in &mut self.modes {
                            let inst_rate = mode.damper_rate * t / ramp;
                            mode.envelope *= (-inst_rate).exp();
                        }
                    }
                }
                if self.damper_ramp_done {
                    for mode in &mut self.modes {
                        mode.envelope *= mode.damper_mult;
                    }
                }
            }

            // Onset ramp: all modes ramp together during hammer contact.
            // Shape exponent: ff -> cosine^1 (standard), pp -> cosine^2 (softer)
            let onset = if self.sample < self.onset_ramp_samples {
                let n = self.sample as f64;
                let cosine = 0.5 * (1.0 - (n * self.onset_ramp_inc).cos());
                // Optimized powf: ff (exp~1.0) uses cosine, pp (exp~2.0) uses cosine²
                if self.onset_shape_exp <= 1.001 {
                    cosine
                } else if self.onset_shape_exp >= 1.999 {
                    cosine * cosine
                } else {
                    cosine.powf(self.onset_shape_exp)
                }
            } else {
                1.0
            };

            // Quadrature oscillator: 0 transcendentals per mode per sample
            for mode in &mut self.modes {
                sum += mode.amplitude * mode.s * onset * mode.envelope;

                let s_new = mode.s * mode.cos_inc + mode.c * mode.sin_inc;
                let c_new = mode.c * mode.cos_inc - mode.s * mode.sin_inc;
                mode.s = s_new;
                mode.c = c_new;

                // Natural decay
                mode.envelope *= mode.decay_mult;
            }

            // Amplitude-dependent loss on mode 1 (air drag): the decay rate grows with the
            // mode's current amplitude. Per-sample change is tiny, so the linearised update is
            // exact to well below audibility.
            if self.drag_k > 0.0 {
                let env = self.modes[0].envelope;
                self.modes[0].envelope = env * (1.0 - self.drag_k * env);
            }

            // Renormalize quadrature radius every 1024 samples
            if self.sample & (RENORM_INTERVAL - 1) == 0 && self.sample > 0 {
                for mode in &mut self.modes {
                    let r_sq = mode.s * mode.s + mode.c * mode.c;
                    let r_inv = 1.0 / r_sq.sqrt();
                    mode.s *= r_inv;
                    mode.c *= r_inv;
                }
            }

            *sample += sum;
            self.sample += 1;
        }
    }

    /// Check if the reed has decayed below a threshold (all modes).
    pub fn is_silent(&self, threshold_db: f64) -> bool {
        let threshold_linear = 10.0_f64.powf(threshold_db / 20.0);
        self.modes
            .iter()
            .all(|mode| (mode.amplitude * mode.envelope).abs() <= threshold_linear)
    }

    /// Check if damper is active.
    pub fn is_damping(&self) -> bool {
        self.damper_active
    }

    /// Get release time in seconds (for safety timeout).
    pub fn release_seconds(&self, sample_rate: f64) -> f64 {
        if self.damper_active {
            self.damper_release_count / sample_rate
        } else {
            0.0
        }
    }
}
