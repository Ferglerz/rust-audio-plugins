//! Spectral peak picking from STFT magnitudes and phases.

use super::types::{Peak, MAX_PEAKS};
use std::f32::consts::PI;

const MAG_FLOOR: f32 = 1e-12;
const DENOM_EPS: f32 = 1e-12;

#[inline]
fn wrap_to_pi(x: f32) -> f32 {
    let two_pi = 2.0 * PI;
    let mut y = (x + PI) % two_pi;
    if y < 0.0 {
        y += two_pi;
    }
    y - PI
}

#[inline]
fn ln_mag(m: f32) -> f32 {
    m.max(MAG_FLOOR).ln()
}

#[allow(clippy::too_many_arguments)]
pub fn pick_peaks(
    mag: &[f32],
    prev_phase: &[f32],
    phase: &[f32],
    bin_hz: f32,
    hop: usize,
    sample_rate: f32,
    threshold_db: f32,
    out: &mut [Peak; MAX_PEAKS],
) -> usize {
    if mag.len() < 5 {
        return 0;
    }

    let threshold = 10.0_f32.powf(threshold_db / 20.0);
    let fft_n = (mag.len() - 1) * 2;
    let n_f = fft_n as f32;
    let use_pv = hop > 0 && prev_phase.len() == mag.len() && phase.len() == mag.len();
    let nyquist = if sample_rate > 0.0 {
        sample_rate * 0.5
    } else {
        f32::INFINITY
    };

    let mut count = 0usize;

    for k in 2..mag.len() - 2 {
        let mk = mag[k];
        if !(mk > mag[k - 1] && mk >= mag[k + 1] && mk >= threshold) {
            continue;
        }

        let a = ln_mag(mag[k - 1]);
        let b = ln_mag(mk);
        let c = ln_mag(mag[k + 1]);
        let denom = a - 2.0 * b + c;
        let p = if denom.abs() <= DENOM_EPS {
            0.0
        } else {
            0.5 * (a - c) / denom
        };
        let f_par = (k as f32 + p) * bin_hz;
        let mag_corr = (b - 0.25 * (a - c) * p).exp();

        let freq_hz = if use_pv {
            let expected = 2.0 * PI * k as f32 * hop as f32 / n_f;
            let dev = wrap_to_pi(phase[k] - prev_phase[k] - expected);
            let f_pv = (k as f32 + dev * n_f / (2.0 * PI * hop as f32)) * bin_hz;
            if (f_pv - f_par).abs() <= bin_hz {
                f_pv
            } else {
                f_par
            }
        } else {
            f_par
        };

        let freq_hz = freq_hz.min(nyquist);
        let peak = Peak {
            freq_hz,
            mag: mag_corr,
        };

        if count < MAX_PEAKS {
            out[count] = peak;
            count += 1;
        } else {
            let mut weakest = 0usize;
            let mut weakest_mag = out[0].mag;
            for i in 1..MAX_PEAKS {
                if out[i].mag < weakest_mag {
                    weakest_mag = out[i].mag;
                    weakest = i;
                }
            }
            if peak.mag > weakest_mag {
                out[weakest] = peak;
            }
        }
    }

    for i in 1..count {
        let key = out[i];
        let mut j = i;
        while j > 0 && out[j - 1].freq_hz > key.freq_hz {
            out[j] = out[j - 1];
            j -= 1;
        }
        out[j] = key;
    }

    count
}

#[cfg(test)]
mod tests {
    use super::super::types::Peak;
    use super::*;

    const N: usize = 4096;
    const LEN: usize = 2049;
    const SR: f32 = 48_000.0;
    const BIN_HZ: f32 = SR / N as f32;
    const HOP: usize = 256;

    fn pick(
        mag: &[f32],
        prev_phase: &[f32],
        phase: &[f32],
        threshold_db: f32,
    ) -> ([Peak; MAX_PEAKS], usize) {
        let mut out = [Peak::default(); MAX_PEAKS];
        let n = pick_peaks(
            mag,
            prev_phase,
            phase,
            BIN_HZ,
            HOP,
            SR,
            threshold_db,
            &mut out,
        );
        (out, n)
    }

    #[test]
    fn single_symmetric_peak() {
        let mut mag = [0.0f32; LEN];
        mag[99] = 0.25;
        mag[100] = 0.5;
        mag[101] = 0.25;
        let (out, n) = pick(&mag, &[], &[], -20.0);
        assert_eq!(n, 1);
        assert!((out[0].freq_hz - 100.0 * BIN_HZ).abs() <= 0.01 * BIN_HZ);
        assert!(out[0].mag >= 0.5);
    }

    #[test]
    fn asymmetric_peak_shifts_up() {
        let mut mag = [0.0f32; LEN];
        mag[99] = 0.3;
        mag[100] = 0.5;
        mag[101] = 0.45;
        let (out, n) = pick(&mag, &[], &[], -20.0);
        assert_eq!(n, 1);
        assert!(out[0].freq_hz > 100.0 * BIN_HZ);
        assert!(out[0].freq_hz < 100.5 * BIN_HZ);
    }

    #[test]
    fn phase_vocoder_fractional_bin() {
        let mut mag = [0.0f32; LEN];
        mag[99] = 0.3;
        mag[100] = 0.5;
        mag[101] = 0.45;
        let mut prev_phase = [0.0f32; LEN];
        let mut phase = [0.0f32; LEN];
        prev_phase[100] = 0.0;
        phase[100] = wrap_to_pi(2.0 * PI * 100.3 * HOP as f32 / N as f32);
        let (out, n) = pick(&mag, &prev_phase, &phase, -20.0);
        assert_eq!(n, 1);
        assert!((out[0].freq_hz - 100.3 * BIN_HZ).abs() <= 0.02 * BIN_HZ);
    }

    #[test]
    fn below_threshold_excluded() {
        let mut mag = [0.0f32; LEN];
        mag[99] = 0.25;
        mag[100] = 0.5;
        mag[101] = 0.25;
        let (_out, n) = pick(&mag, &[], &[], 0.0);
        assert_eq!(n, 0);
    }

    #[test]
    fn keeps_strongest_and_sorts_by_freq() {
        let mut mag = [0.0f32; LEN];
        for i in 0..200 {
            mag[2 + i * 2] = 0.5 + i as f32 * 0.001;
        }
        let (out, n) = pick(&mag, &[], &[], -20.0);
        assert_eq!(n, 128);
        let mut min_mag = f32::INFINITY;
        let mut max_mag = 0.0f32;
        for i in 0..n {
            if i + 1 < n {
                assert!(out[i].freq_hz <= out[i + 1].freq_hz);
            }
            min_mag = min_mag.min(out[i].mag);
            max_mag = max_mag.max(out[i].mag);
        }
        // Strongest 128 of 200: i = 72..=199, mag 0.572 ..= 0.699.
        assert!((min_mag - 0.572).abs() < 1e-5);
        assert!((max_mag - 0.699).abs() < 1e-5);
        assert!((out[0].freq_hz - 146.0 * BIN_HZ).abs() <= 0.01 * BIN_HZ);
    }

    #[test]
    fn tiny_inputs_return_zero() {
        let mut out = [Peak::default(); MAX_PEAKS];
        assert_eq!(
            pick_peaks(&[], &[], &[], BIN_HZ, HOP, SR, -20.0, &mut out),
            0
        );
        assert_eq!(
            pick_peaks(&[0.0; 1], &[], &[], BIN_HZ, HOP, SR, -20.0, &mut out),
            0
        );
        assert_eq!(
            pick_peaks(&[0.0; 4], &[], &[], BIN_HZ, HOP, SR, -20.0, &mut out),
            0
        );
    }
}
