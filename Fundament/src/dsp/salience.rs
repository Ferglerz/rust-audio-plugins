//! Harmonic salience.
//!
//! Iterative estimate-and-cancel: score fundamental candidates against the
//! current peak magnitudes, keep the strongest, attenuate the peaks that
//! supported it, and repeat.

use super::types::{F0Candidate, Partial, Peak, MAX_HARMONICS, MAX_PEAKS, MAX_VOICES};

/// Cents half-width before the per-harmonic stretch.
const BASE_CENTS: f32 = 30.0;
/// Hertz half-width before the per-harmonic stretch. A peak matches when it
/// sits inside the wider of the cents window and this absolute window.
const BASE_HZ: f32 = 2.0;
/// Higher harmonics of strings run sharp. Harmonic `h` widens tolerance by
/// `1 + INHARM_STRETCH * (h - 1)`.
const INHARM_STRETCH: f32 = 0.1;
const MISSING_FUNDAMENTAL_GAIN: f32 = 0.8;
/// `1/sqrt(h)` alone scores a present octave above a missing fundamental.
/// Blend in how many of the requested harmonics actually matched:
/// `SUPPORT_FLOOR + (1 - SUPPORT_FLOOR) * matched / harmonics`.
const SUPPORT_FLOOR: f32 = 0.40;
const CANCEL_KEEP: f32 = 0.25;
const RELATIVE_SALIENCE: f32 = 0.15;
const MIN_SALIENCE: f32 = 1.0e-6;
const MIN_SEPARATION_CENTS: f32 = 60.0;
const MAX_CANDIDATES: usize = MAX_PEAKS * 3;

pub fn pick_f0s(
    peaks: &[Peak],
    harmonics: usize,
    low_hz: f32,
    high_hz: f32,
    max_voices: usize,
    out: &mut [F0Candidate; MAX_VOICES],
) -> usize {
    for slot in out.iter_mut() {
        *slot = F0Candidate::default();
    }

    let voices = max_voices.min(MAX_VOICES);
    let harmonics = harmonics.clamp(1, MAX_HARMONICS);
    if voices == 0 || !range_ok(low_hz, high_hz) {
        return 0;
    }

    let mut freqs = [0.0f32; MAX_PEAKS];
    let mut mags = [0.0f32; MAX_PEAKS];
    let n = load_peaks(peaks, &mut freqs, &mut mags);
    if n == 0 {
        return 0;
    }

    let mut candidates = [0.0f32; MAX_CANDIDATES];
    let n_cand = collect_candidates(n, &freqs, &mags, low_hz, high_hz, &mut candidates);
    if n_cand == 0 {
        return 0;
    }

    let mut n_out = 0usize;
    let mut first_salience = 0.0f32;
    while n_out < voices {
        let mut best_f0 = 0.0f32;
        let mut best_salience = 0.0f32;
        for &candidate in candidates.iter().take(n_cand) {
            if !separated(candidate, out, n_out) {
                continue;
            }
            let score = salience(candidate, harmonics, n, &freqs, &mags);
            if score > best_salience {
                best_salience = score;
                best_f0 = candidate;
            }
        }

        if best_salience < MIN_SALIENCE {
            break;
        }
        if n_out > 0 && best_salience < RELATIVE_SALIENCE * first_salience {
            break;
        }

        let f0 = refine_f0(best_f0, n, &freqs, &mags);
        if n_out == 0 {
            first_salience = best_salience;
        }
        out[n_out] = F0Candidate {
            f0_hz: f0,
            salience: best_salience,
        };
        n_out += 1;
        cancel(f0, harmonics, n, &freqs, &mut mags);
    }
    n_out
}

pub fn harmonics_for(
    f0_hz: f32,
    peaks: &[Peak],
    harmonics: usize,
    tol_cents: f32,
    out: &mut [Partial; MAX_HARMONICS],
) {
    if !(f0_hz > 0.0) || !f0_hz.is_finite() {
        for partial in out.iter_mut() {
            *partial = Partial::default();
        }
        return;
    }

    let mut freqs = [0.0f32; MAX_PEAKS];
    let mut mags = [0.0f32; MAX_PEAKS];
    let n = load_peaks(peaks, &mut freqs, &mut mags);
    let cents_base = if tol_cents.is_finite() {
        tol_cents.max(0.0)
    } else {
        0.0
    };

    for (index, partial) in out.iter_mut().enumerate() {
        let harmonic = index + 1;
        if harmonic > harmonics {
            *partial = Partial::default();
            continue;
        }
        let expected = f0_hz * harmonic as f32;
        if !expected.is_finite() || expected <= 0.0 {
            *partial = Partial::default();
            continue;
        }
        *partial = match best_peak(expected, harmonic, n, &freqs, &mags, Some(cents_base)) {
            Some((idx, mag)) => Partial {
                freq_hz: freqs[idx],
                mag,
            },
            None => Partial {
                freq_hz: expected,
                mag: 0.0,
            },
        };
    }
}

fn range_ok(low_hz: f32, high_hz: f32) -> bool {
    low_hz.is_finite() && high_hz.is_finite() && low_hz <= high_hz && high_hz > 0.0
}

fn load_peaks(peaks: &[Peak], freqs: &mut [f32; MAX_PEAKS], mags: &mut [f32; MAX_PEAKS]) -> usize {
    let n = peaks.len().min(MAX_PEAKS);
    for i in 0..n {
        let peak = peaks[i];
        if peak.freq_hz.is_finite() && peak.mag.is_finite() && peak.freq_hz > 0.0 && peak.mag > 0.0
        {
            freqs[i] = peak.freq_hz;
            mags[i] = peak.mag;
        } else {
            freqs[i] = 0.0;
            mags[i] = 0.0;
        }
    }
    n
}

fn collect_candidates(
    n: usize,
    freqs: &[f32],
    mags: &[f32],
    low_hz: f32,
    high_hz: f32,
    out: &mut [f32; MAX_CANDIDATES],
) -> usize {
    let mut count = 0usize;
    for i in 0..n {
        if !(mags[i] > 0.0) || !(freqs[i] > 0.0) {
            continue;
        }
        for divisor in 1..=3 {
            let f0 = freqs[i] / divisor as f32;
            if count < MAX_CANDIDATES && f0.is_finite() && f0 >= low_hz && f0 <= high_hz {
                out[count] = f0;
                count += 1;
            }
        }
    }
    count
}

fn salience(f0: f32, harmonics: usize, n: usize, freqs: &[f32], mags: &[f32]) -> f32 {
    if harmonics == 0 || !(f0 > 0.0) || !f0.is_finite() {
        return 0.0;
    }

    let mut sum = 0.0f32;
    let mut matched = 0usize;
    let mut low_hits = 0usize;
    let mut fundamental = false;
    let low_limit = harmonics.min(4);

    for harmonic in 1..=harmonics {
        let expected = f0 * harmonic as f32;
        let Some((_, mag)) = best_peak(expected, harmonic, n, freqs, mags, None) else {
            continue;
        };
        sum += mag / (harmonic as f32).sqrt();
        matched += 1;
        if harmonic == 1 {
            fundamental = true;
        }
        if harmonic <= low_limit {
            low_hits += 1;
        }
    }

    if !fundamental && low_hits < 2 {
        return 0.0;
    }
    if !fundamental {
        sum *= MISSING_FUNDAMENTAL_GAIN;
    }
    let coverage = matched as f32 / harmonics as f32;
    sum * (SUPPORT_FLOOR + (1.0 - SUPPORT_FLOOR) * coverage)
}

/// Report the measured fundamental peak when harmonic 1 is present.
/// Subharmonic candidates such as `3 * f0 / 2` of a neighbor otherwise win by a fraction of a hertz.
fn refine_f0(f0: f32, n: usize, freqs: &[f32], mags: &[f32]) -> f32 {
    if let Some((idx, _)) = best_peak(f0, 1, n, freqs, mags, None) {
        let peak_hz = freqs[idx];
        if peak_hz.is_finite() && peak_hz > 0.0 {
            return peak_hz;
        }
    }
    f0
}

fn cancel(f0: f32, harmonics: usize, n: usize, freqs: &[f32], mags: &mut [f32]) {
    let mut used = [false; MAX_PEAKS];
    for harmonic in 1..=harmonics {
        let Some((idx, _)) = best_peak(f0 * harmonic as f32, harmonic, n, freqs, mags, None) else {
            continue;
        };
        if !used[idx] {
            mags[idx] *= CANCEL_KEEP;
            used[idx] = true;
        }
    }
}

fn separated(f0: f32, picked: &[F0Candidate], n_picked: usize) -> bool {
    picked
        .iter()
        .take(n_picked)
        .all(|slot| cents_between(f0, slot.f0_hz) > MIN_SEPARATION_CENTS)
}

fn best_peak(
    expected: f32,
    harmonic: usize,
    n: usize,
    freqs: &[f32],
    mags: &[f32],
    cents_base: Option<f32>,
) -> Option<(usize, f32)> {
    if !(expected > 0.0) || !expected.is_finite() {
        return None;
    }

    let mut best_index = None;
    let mut best_mag = 0.0f32;
    let mut best_dist = f32::MAX;
    for index in 0..n {
        let freq = freqs[index];
        let mag = mags[index];
        if !(freq > 0.0) || !(mag > 0.0) || !accepts(freq, expected, harmonic, cents_base) {
            continue;
        }
        let dist = (freq - expected).abs();
        let tie = (mag - best_mag).abs() <= 1.0e-6 * best_mag.max(1.0);
        if mag > best_mag || (tie && dist < best_dist) {
            best_mag = mag;
            best_dist = dist;
            best_index = Some(index);
        }
    }
    best_index.map(|index| (index, best_mag))
}

fn accepts(freq: f32, expected: f32, harmonic: usize, cents_base: Option<f32>) -> bool {
    let widened = stretch(harmonic);
    if let Some(base) = cents_base {
        if !base.is_finite() {
            return false;
        }
        return cents_between(freq, expected) <= base.max(0.0) * widened;
    }
    if (freq - expected).abs() <= BASE_HZ * widened {
        return true;
    }
    cents_between(freq, expected) <= BASE_CENTS * widened
}

#[inline]
fn stretch(harmonic: usize) -> f32 {
    1.0 + INHARM_STRETCH * harmonic.saturating_sub(1) as f32
}

fn cents_between(a: f32, b: f32) -> f32 {
    if a <= 0.0 || b <= 0.0 {
        return f32::MAX;
    }
    let ratio = a / b;
    if !ratio.is_finite() || ratio <= 0.0 {
        return f32::MAX;
    }
    (1_200.0 * ratio.log2()).abs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_partials(dst: &mut [Peak], count: &mut usize, f0: f32, first: usize, last: usize) {
        for harmonic in first..=last {
            if *count >= dst.len() {
                break;
            }
            dst[*count] = Peak {
                freq_hz: f0 * harmonic as f32,
                mag: 1.0 / harmonic as f32,
            };
            *count += 1;
        }
    }

    fn sort_peaks(peaks: &mut [Peak]) {
        for i in 1..peaks.len() {
            let mut j = i;
            while j > 0 && peaks[j].freq_hz < peaks[j - 1].freq_hz {
                peaks.swap(j, j - 1);
                j -= 1;
            }
        }
    }

    fn pick(
        peaks: &[Peak],
        harmonics: usize,
        low_hz: f32,
        high_hz: f32,
        voices: usize,
    ) -> (usize, [F0Candidate; MAX_VOICES]) {
        let mut out = [F0Candidate::default(); MAX_VOICES];
        let n = pick_f0s(peaks, harmonics, low_hz, high_hz, voices, &mut out);
        (n, out)
    }

    fn assert_has(out: &[F0Candidate], n: usize, hz: f32, tol: f32) {
        let found = out[..n]
            .iter()
            .any(|candidate| (candidate.f0_hz - hz).abs() <= tol);
        assert!(found, "missing {hz} ±{tol}, got {out:?}");
    }

    #[test]
    fn saw_picks_fundamental_not_octave() {
        let mut peaks = [Peak::default(); 16];
        let mut count = 0usize;
        add_partials(&mut peaks, &mut count, 110.0, 1, 10);
        let (n, out) = pick(&peaks[..count], 10, 40.0, 2_000.0, 4);
        assert!(n >= 1);
        let f0 = out[0].f0_hz;
        assert!((f0 - 110.0).abs() <= 1.0, "{f0}");
        assert!((f0 - 55.0).abs() > 1.0, "{f0}");
        assert!((f0 - 220.0).abs() > 1.0, "{f0}");
    }

    #[test]
    fn missing_fundamental_beats_octave() {
        let mut peaks = [Peak::default(); 16];
        let mut count = 0usize;
        add_partials(&mut peaks, &mut count, 100.0, 2, 8);
        let (n, out) = pick(&peaks[..count], 8, 40.0, 2_000.0, 3);
        assert!(n >= 1);
        assert!((out[0].f0_hz - 100.0).abs() <= 1.0, "{}", out[0].f0_hz);
    }

    #[test]
    fn major_third_finds_both_notes() {
        let mut peaks = [Peak::default(); 32];
        let mut count = 0usize;
        add_partials(&mut peaks, &mut count, 110.0, 1, 8);
        add_partials(&mut peaks, &mut count, 138.6, 1, 8);
        sort_peaks(&mut peaks[..count]);
        let (n, out) = pick(&peaks[..count], 8, 40.0, 2_000.0, 2);
        assert!(n >= 2, "{out:?}");
        assert_has(&out, n, 110.0, 1.5);
        assert_has(&out, n, 138.6, 1.5);
        assert!(out[0].salience >= out[1].salience);
    }

    #[test]
    fn power_chord_finds_root_and_fifth() {
        let mut peaks = [Peak::default(); 32];
        let mut count = 0usize;
        add_partials(&mut peaks, &mut count, 82.41, 1, 6);
        add_partials(&mut peaks, &mut count, 123.47, 1, 6);
        add_partials(&mut peaks, &mut count, 164.81, 1, 6);
        sort_peaks(&mut peaks[..count]);
        let (n, out) = pick(&peaks[..count], 6, 35.0, 1_200.0, 3);
        assert_has(&out, n, 82.41, 1.0);
        assert_has(&out, n, 123.47, 1.0);
    }

    #[test]
    fn empty_peaks_and_low_cutoff_hide_fundamental() {
        let mut out = [F0Candidate::default(); MAX_VOICES];
        assert_eq!(pick_f0s(&[], 8, 40.0, 1_000.0, 3, &mut out), 0);
        assert_eq!(pick_f0s(&[], 8, 40.0, 1_000.0, 0, &mut out), 0);

        let mut peaks = [Peak::default(); 16];
        let mut count = 0usize;
        add_partials(&mut peaks, &mut count, 110.0, 1, 10);
        let (n, out) = pick(&peaks[..count], 10, 200.0, 2_000.0, 4);
        for candidate in &out[..n] {
            assert!((candidate.f0_hz - 110.0).abs() > 1.0, "{}", candidate.f0_hz);
        }
    }

    #[test]
    fn harmonics_for_exact_sharp_missing_and_beyond() {
        let f0 = 100.0;
        let sharp = 200.0 * 2.0f32.powf(15.0 / 1_200.0);
        let peaks = [
            Peak {
                freq_hz: 100.0,
                mag: 1.0,
            },
            Peak {
                freq_hz: sharp,
                mag: 0.5,
            },
            Peak {
                freq_hz: 400.0,
                mag: 0.25,
            },
            Peak {
                freq_hz: 500.0,
                mag: 0.9,
            },
        ];
        let mut out = [Partial::default(); MAX_HARMONICS];
        harmonics_for(f0, &peaks, 4, 30.0, &mut out);

        assert_eq!(out[0].freq_hz, 100.0);
        assert_eq!(out[0].mag, 1.0);
        assert!(
            (out[1].freq_hz - sharp).abs() < 1.0e-3,
            "{}",
            out[1].freq_hz
        );
        assert_eq!(out[1].mag, 0.5);
        assert!((out[2].freq_hz - 300.0).abs() < 1.0e-3);
        assert_eq!(out[2].mag, 0.0);
        assert_eq!(out[3].freq_hz, 400.0);
        assert_eq!(out[3].mag, 0.25);
        for partial in &out[4..] {
            assert_eq!(*partial, Partial::default());
        }

        harmonics_for(0.0, &peaks, 4, 30.0, &mut out);
        assert!(out.iter().all(|partial| *partial == Partial::default()));
        harmonics_for(-12.0, &peaks, 4, 30.0, &mut out);
        assert!(out.iter().all(|partial| *partial == Partial::default()));
    }
}
