// Revised paired f64x2 reed experiment; see docs/simd-investigation-2026-11-22.
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
use wide::f64x2;

/// Quadrature renormalization interval. Every 1024 samples, correct the
/// radius of each (s,c) pair to prevent drift from accumulated FP error.
/// Cost: 7 sqrt + 7 div per 1024 samples = 0.014 transcendentals/sample.
const RENORM_INTERVAL: u64 = 1024;

/// Seven modal states stored by field for adjacent f64x2 loads.
/// Mode 0 stays scalar for air drag; modes 1–6 use consistent pairs in every stage.
pub struct ModalReed {
    s: [f64; NUM_MODES],
    c: [f64; NUM_MODES],
    cos_inc: [f64; NUM_MODES],
    sin_inc: [f64; NUM_MODES],
    amplitude: [f64; NUM_MODES],
    decay_mult: [f64; NUM_MODES],
    envelope: [f64; NUM_MODES],
    damper_rate: [f64; NUM_MODES],
    damper_mult: [f64; NUM_MODES],
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
        let phase_inc: [f64; NUM_MODES] =
            std::array::from_fn(|i| TAU * (fundamental_hz * mode_ratios[i]) / sample_rate);
        let cos_inc = phase_inc.map(f64::cos);
        let sin_inc = phase_inc.map(f64::sin);
        let decay_mult = std::array::from_fn(|i| {
            let alpha_nepers = decay_rates_db[i] / 8.686;
            let decay_per_sample = alpha_nepers / sample_rate;
            (-decay_per_sample).exp()
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
            s: [0.0; NUM_MODES],
            c: [1.0; NUM_MODES],
            cos_inc,
            sin_inc,
            amplitude: *amplitudes,
            decay_mult,
            envelope: [1.0; NUM_MODES],
            damper_rate: [0.0; NUM_MODES],
            damper_mult: [1.0; NUM_MODES],
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
        for (i, &ph) in phases.iter().enumerate() {
            self.s[i] = ph.sin();
            self.c[i] = ph.cos();
        }
    }

    /// Enable amplitude-dependent loss on mode 1: quadratic air drag on the moving reed, which
    /// decays the mode as `dA/dt = -beta |A| A`. `beta_db_per_mm` is the extra decay in dB/s per
    /// mm of swing at the pickup's finger region; `swing_mm` is the swing that one unit of model
    /// displacement stands for there. The mode's current amplitude (initial amplitude x envelope)
    /// sets the loss, so a loud note loses energy faster early and slower once it has decayed.
    pub fn set_air_drag(&mut self, beta_db_per_mm: f64, swing_mm: f64, sample_rate: f64) {
        let beta_nepers = beta_db_per_mm / 8.686;
        self.drag_k = beta_nepers * self.amplitude[0].abs() * swing_mm / sample_rate;
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
        for m in 0..NUM_MODES {
            let factor = (base_rate * 3.0_f64.powi(m as i32)).min(2000.0);
            self.damper_rate[m] = factor / sample_rate;
            self.damper_mult[m] = (-self.damper_rate[m]).exp();
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
                        let inst_rate = self.damper_rate[0] * t / ramp;
                        self.envelope[0] *= (-inst_rate).exp();
                        for i in (1..NUM_MODES).step_by(2) {
                            let a = self.damper_rate[i] * t / ramp;
                            let b = self.damper_rate[i + 1] * t / ramp;
                            let multiplier = f64x2::new([(-a).exp(), (-b).exp()]);
                            let envelope = f64x2::new([self.envelope[i], self.envelope[i + 1]]);
                            self.envelope[i..i + 2]
                                .copy_from_slice(&(envelope * multiplier).to_array());
                        }
                    }
                }
                if self.damper_ramp_done {
                    self.envelope[0] *= self.damper_mult[0];
                    for i in (1..NUM_MODES).step_by(2) {
                        let envelope = f64x2::new([self.envelope[i], self.envelope[i + 1]]);
                        let multiplier = f64x2::new([self.damper_mult[i], self.damper_mult[i + 1]]);
                        self.envelope[i..i + 2]
                            .copy_from_slice(&(envelope * multiplier).to_array());
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

            let i = 0;
            sum += self.amplitude[i] * self.s[i] * onset * self.envelope[i];
            let s_new = self.s[i] * self.cos_inc[i] + self.c[i] * self.sin_inc[i];
            let c_new = self.c[i] * self.cos_inc[i] - self.s[i] * self.sin_inc[i];
            self.s[i] = s_new;
            self.c[i] = c_new;
            self.envelope[i] *= self.decay_mult[i];

            // Preserve each lane's arithmetic and reduce in mode order.
            let onset_pair = f64x2::splat(onset);
            for i in (1..NUM_MODES).step_by(2) {
                let sine = f64x2::new([self.s[i], self.s[i + 1]]);
                let cosine = f64x2::new([self.c[i], self.c[i + 1]]);
                let cos_inc = f64x2::new([self.cos_inc[i], self.cos_inc[i + 1]]);
                let sin_inc = f64x2::new([self.sin_inc[i], self.sin_inc[i + 1]]);
                let envelope = f64x2::new([self.envelope[i], self.envelope[i + 1]]);
                let amplitude = f64x2::new([self.amplitude[i], self.amplitude[i + 1]]);
                let contribution = (amplitude * sine * onset_pair * envelope).to_array();
                sum += contribution[0];
                sum += contribution[1];
                self.s[i..i + 2].copy_from_slice(&(sine * cos_inc + cosine * sin_inc).to_array());
                self.c[i..i + 2].copy_from_slice(&(cosine * cos_inc - sine * sin_inc).to_array());
                let decay = f64x2::new([self.decay_mult[i], self.decay_mult[i + 1]]);
                self.envelope[i..i + 2].copy_from_slice(&(envelope * decay).to_array());
            }
            // Amplitude-dependent loss on mode 1 (air drag): the decay rate grows with the
            // mode's current amplitude. Per-sample change is tiny, so the linearised update is
            // exact to well below audibility.
            if self.drag_k > 0.0 {
                let env = self.envelope[0];
                self.envelope[0] = env * (1.0 - self.drag_k * env);
            }

            // Renormalize quadrature radius every 1024 samples
            if self.sample & (RENORM_INTERVAL - 1) == 0 && self.sample > 0 {
                let r_sq = self.s[0] * self.s[0] + self.c[0] * self.c[0];
                let r_inv = 1.0 / r_sq.sqrt();
                self.s[0] *= r_inv;
                self.c[0] *= r_inv;
                for i in (1..NUM_MODES).step_by(2) {
                    let sine = f64x2::new([self.s[i], self.s[i + 1]]);
                    let cosine = f64x2::new([self.c[i], self.c[i + 1]]);
                    let radius = sine * sine + cosine * cosine;
                    let inverse = f64x2::splat(1.0) / radius.sqrt();
                    self.s[i..i + 2].copy_from_slice(&(sine * inverse).to_array());
                    self.c[i..i + 2].copy_from_slice(&(cosine * inverse).to_array());
                }
            }

            *sample += sum;
            self.sample += 1;
        }
    }

    /// Check if the reed has decayed below a threshold (all modes).
    pub fn is_silent(&self, threshold_db: f64) -> bool {
        let threshold_linear = 10.0_f64.powf(threshold_db / 20.0);
        self.amplitude
            .iter()
            .zip(&self.envelope)
            .all(|(amplitude, envelope)| (amplitude * envelope).abs() <= threshold_linear)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_air_drag_is_amplitude_dependent() {
        // Quadratic drag: dA/dt = -beta |A| A. A loud reed loses more dB per second than a soft
        // one, and the loss rate falls as the reed decays. With drag off, the decay in dB is the
        // same at every amplitude.
        let sr = 44100.0;
        let ratios = [1.0, 6.27, 17.55, 34.39, 56.84, 84.91, 118.6];
        let decays = [4.0, 30.0, 60.0, 90.0, 120.0, 150.0, 180.0];
        let fall_db = |amp: f64, beta: f64| {
            let amps = [amp, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
            let mut reed = ModalReed::new(440.0, &ratios, &amps, &decays, 0.0, 1.0, sr);
            reed.set_air_drag(beta, 1.2, sr);
            let mut buf = vec![0.0f64; sr as usize];
            reed.render(&mut buf);
            let pk = |s: &[f64]| s.iter().fold(0.0f64, |a, x| a.max(x.abs()));
            20.0 * (pk(&buf[..2000]) / pk(&buf[buf.len() - 2000..])).log10()
        };
        let (loud, soft) = (fall_db(1.0, 1.6), fall_db(0.1, 1.6));
        let (loud_off, soft_off) = (fall_db(1.0, 0.0), fall_db(0.1, 0.0));
        assert!(
            (loud_off - soft_off).abs() < 0.05,
            "no drag: {loud_off} vs {soft_off}"
        );
        // Expected extra over ~1 s at A0 1.2 mm, beta 1.6 dB/s/mm: roughly 1.5-1.9 dB.
        assert!(
            loud - loud_off > 1.2 && loud - loud_off < 2.2,
            "loud extra {}",
            loud - loud_off
        );
        assert!(soft - soft_off < 0.25, "soft extra {}", soft - soft_off);
    }

    #[test]
    fn test_single_mode_sine() {
        let mut amps = [0.0f64; NUM_MODES];
        amps[0] = 1.0;
        let ratios = [1.0, 6.267, 17.547, 34.386, 56.842, 85.1, 119.3];
        let decays = [0.0f64; NUM_MODES];

        let mut reed = ModalReed::new(440.0, &ratios, &amps, &decays, 0.0, 1.0, 44100.0);
        let mut buf = vec![0.0f64; 44100];
        reed.render(&mut buf);

        let mut crossings = 0u32;
        for i in 1..buf.len() {
            if buf[i - 1] < 0.0 && buf[i] >= 0.0 {
                crossings += 1;
            }
        }
        assert!(
            (crossings as f64 - 440.0).abs() < 3.0,
            "Average frequency should be ~440 Hz, got {crossings} crossings"
        );
    }

    #[test]
    fn test_decay() {
        let mut amps = [0.0f64; NUM_MODES];
        amps[0] = 1.0;
        let ratios = [1.0, 6.267, 17.547, 34.386, 56.842, 85.1, 119.3];
        let mut decays = [0.0f64; NUM_MODES];
        decays[0] = 60.0;

        let sr = 44100.0;
        let mut reed = ModalReed::new(440.0, &ratios, &amps, &decays, 0.0, 1.0, sr);

        let half_sec = (sr * 0.5) as usize;
        let mut buf = vec![0.0f64; half_sec];
        reed.render(&mut buf);

        let peak = buf[buf.len() - 200..]
            .iter()
            .map(|x| x.abs())
            .fold(0.0f64, f64::max);
        assert!(peak < 0.1, "expected decay, got peak {peak}");
        assert!(peak > 0.01, "decayed too much, got peak {peak}");
    }

    #[test]
    fn test_onset_ramp_shapes_attack() {
        // With a 20ms ramp (bass ring-up), the onset should build up smoothly.
        let sr = 44100.0;
        let ramp = 0.020;
        let mut amps = [0.0f64; NUM_MODES];
        amps[0] = 1.0;
        let ratios = [1.0, 6.267, 17.547, 34.386, 56.842, 85.1, 119.3];
        let decays = [0.0f64; NUM_MODES];

        let mut reed = ModalReed::new(440.0, &ratios, &amps, &decays, ramp, 1.0, sr);
        let n = (sr * 0.050) as usize;
        let mut buf = vec![0.0f64; n];
        reed.render(&mut buf);

        assert!(
            buf[0].abs() < 0.01,
            "First sample should be near zero, got {:.6}",
            buf[0]
        );

        let mid_ramp = (ramp * 0.5 * sr) as usize;
        let mid_peak = buf[mid_ramp.saturating_sub(5)..mid_ramp + 5]
            .iter()
            .map(|x| x.abs())
            .fold(0.0f64, f64::max);
        assert!(
            mid_peak < 0.8,
            "Mid-ramp peak should be < 0.8, got {mid_peak:.4}"
        );

        let late_start = (sr * 0.030) as usize;
        let late_peak = buf[late_start..late_start + 200]
            .iter()
            .map(|x| x.abs())
            .fold(0.0f64, f64::max);
        assert!(
            late_peak > 0.85,
            "Post-ramp peak should be ~1.0, got {late_peak:.4}"
        );
    }

    #[test]
    fn test_onset_ramp_ff_vs_pp() {
        let sr = 44100.0;
        let mut amps = [0.0f64; NUM_MODES];
        amps[0] = 1.0;
        let ratios = [1.0, 6.267, 17.547, 34.386, 56.842, 85.1, 119.3];
        let decays = [0.0f64; NUM_MODES];

        let mut reed_ff = ModalReed::new(440.0, &ratios, &amps, &decays, 0.001, 1.0, sr);
        let mut reed_pp = ModalReed::new(440.0, &ratios, &amps, &decays, 0.005, 0.0, sr);

        let n = (sr * 0.010) as usize;
        let mut buf_ff = vec![0.0f64; n];
        let mut buf_pp = vec![0.0f64; n];
        reed_ff.render(&mut buf_ff);
        reed_pp.render(&mut buf_pp);

        let t2ms = (sr * 0.002) as usize;
        let ff_energy: f64 = buf_ff[..t2ms].iter().map(|x| x * x).sum();
        let pp_energy: f64 = buf_pp[..t2ms].iter().map(|x| x * x).sum();
        assert!(
            ff_energy > pp_energy * 1.5,
            "ff should be louder than pp at 2ms: ff={ff_energy:.6}, pp={pp_energy:.6}"
        );
    }

    #[test]
    fn test_onset_zero_dwell_is_instant() {
        let sr = 44100.0;
        let mut amps = [0.0f64; NUM_MODES];
        amps[0] = 1.0;
        let ratios = [1.0, 6.267, 17.547, 34.386, 56.842, 85.1, 119.3];
        let decays = [0.0f64; NUM_MODES];

        let mut reed = ModalReed::new(440.0, &ratios, &amps, &decays, 0.0, 1.0, sr);
        let mut buf = vec![0.0f64; 100];
        reed.render(&mut buf);

        let early_peak = buf[..10].iter().map(|x| x.abs()).fold(0.0f64, f64::max);
        assert!(
            early_peak > 0.05,
            "Zero dwell should give immediate amplitude, got {early_peak:.6}"
        );
    }

    #[test]
    fn test_mode_holds_exact_frequency() {
        // No frequency wander: a single undamped mode stays on the analytic sine to within
        // floating-point error over a second (the quadrature step is renormalised, not jittered).
        let sr = 44100.0;
        let f = 440.0;
        let mut amps = [0.0f64; NUM_MODES];
        amps[0] = 1.0;
        let ratios = [1.0, 6.267, 17.547, 34.386, 56.842, 85.1, 119.3];
        let decays = [0.0f64; NUM_MODES];
        let mut reed = ModalReed::new(f, &ratios, &amps, &decays, 0.0, 1.0, sr);
        let mut buf = vec![0.0f64; sr as usize];
        reed.render(&mut buf);
        let worst = buf
            .iter()
            .enumerate()
            .map(|(i, &y)| (y - (TAU * f * i as f64 / sr).sin()).abs())
            .fold(0.0f64, f64::max);
        assert!(
            worst < 1e-6,
            "mode drifted off the analytic sine by {worst:e}"
        );
    }
}
