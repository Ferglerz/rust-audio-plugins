use crate::BandCoefficients;

#[derive(Default)]
struct CachedBand {
    coefficients: Option<BandCoefficients>,
    response: Vec<f64>,
}

/// UI/control-thread cache of individual and summed EQ responses.
///
/// Callers own the frequency grid (including any Nyquist clamp) and supply the
/// actual coefficient evaluation rate, which may differ from the host rate.
/// Changed grids/band counts can allocate; this is not an audio-thread API.
#[derive(Default)]
pub struct ResponseCache {
    frequencies: Vec<f64>,
    sample_rate: Option<f64>,
    bands: Vec<CachedBand>,
    sum: Vec<f64>,
}

impl ResponseCache {
    /// Returns whether responses changed. An unchanged update neither allocates
    /// nor evaluates filter responses. Unchanged bands reuse their sampled data.
    pub fn update(
        &mut self,
        coefficients: &[BandCoefficients],
        frequencies: &[f64],
        sample_rate: f64,
    ) -> bool {
        let grid_changed = self.sample_rate != Some(sample_rate) || self.frequencies != frequencies;
        if !grid_changed
            && self.bands.len() == coefficients.len()
            && self
                .bands
                .iter()
                .zip(coefficients)
                .all(|(cached, next)| cached.coefficients.as_ref() == Some(next))
        {
            return false;
        }
        if grid_changed {
            self.frequencies.clear();
            self.frequencies.extend_from_slice(frequencies);
            self.sample_rate = Some(sample_rate);
        }
        self.bands
            .resize_with(coefficients.len(), CachedBand::default);
        self.sum.resize(frequencies.len(), 0.0);
        self.sum.fill(0.0);
        for (cached, &next) in self.bands.iter_mut().zip(coefficients) {
            if grid_changed || cached.coefficients != Some(next) {
                cached.response.resize(frequencies.len(), 0.0);
                for (value, &frequency) in cached.response.iter_mut().zip(frequencies) {
                    *value = next.response_db(frequency, sample_rate);
                }
                cached.coefficients = Some(next);
            }
            // Preserve caller band order and the direct evaluator's sum order.
            for (sum, &value) in self.sum.iter_mut().zip(&cached.response) {
                *sum += value;
            }
        }
        true
    }

    pub fn band(&self, index: usize) -> &[f64] {
        &self.bands[index].response
    }

    pub fn sum(&self) -> &[f64] {
        &self.sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BandSettings, EqShape};

    fn coefficients(gain_db: f64, rate: f64) -> [BandCoefficients; 2] {
        [
            BandCoefficients::prepare(
                &BandSettings {
                    gain_db,
                    ..BandSettings::default()
                },
                rate,
            ),
            BandCoefficients::prepare(
                &BandSettings {
                    shape: EqShape::Notch,
                    frequency_hz: 2345.0,
                    q: 18.0,
                    ..BandSettings::default()
                },
                rate,
            ),
        ]
    }

    fn assert_direct(cache: &ResponseCache, bands: &[BandCoefficients], grid: &[f64], rate: f64) {
        for (i, &frequency) in grid.iter().enumerate() {
            let mut sum = 0.0;
            for (j, &band) in bands.iter().enumerate() {
                let response = band.response_db(frequency, rate);
                assert_eq!(cache.band(j)[i], response);
                sum += response;
            }
            assert_eq!(cache.sum()[i], sum);
        }
    }

    #[test]
    fn responses_match_direct_and_invalidate_every_input() {
        let mut cache = ResponseCache::default();
        // Exact notch frequency and host Nyquist clamp remain caller-owned.
        let mut grid = vec![20.0, 1000.0, 2345.0, 48000.0 * 0.49];
        let mut bands = coefficients(6.0, 96000.0);
        assert!(cache.update(&bands, &grid, 96000.0));
        assert_direct(&cache, &bands, &grid, 96000.0);
        let storage = (
            cache.band(0).as_ptr(),
            cache.band(1).as_ptr(),
            cache.sum().as_ptr(),
        );
        assert!(!cache.update(&bands, &grid, 96000.0));
        assert_eq!(
            storage,
            (
                cache.band(0).as_ptr(),
                cache.band(1).as_ptr(),
                cache.sum().as_ptr()
            )
        );
        let untouched = cache.band(1).to_vec();
        bands[0] = coefficients(-3.0, 96000.0)[0];
        assert!(cache.update(&bands, &grid, 96000.0));
        assert_eq!(cache.band(1), untouched);
        assert_direct(&cache, &bands, &grid, 96000.0);
        grid[1] = 500.0;
        assert!(cache.update(&bands, &grid, 96000.0));
        assert_direct(&cache, &bands, &grid, 96000.0);
        assert!(cache.update(&bands, &grid, 48000.0));
        assert_direct(&cache, &bands, &grid, 48000.0);
        assert!(cache.update(&bands[..1], &grid, 48000.0));
        assert_direct(&cache, &bands[..1], &grid, 48000.0);
        assert!(cache.update(&[], &grid, 48000.0));
        assert_eq!(cache.sum(), &[0.0; 4]);
        assert!(!cache.update(&[], &grid, 48000.0));
        cache.update(&bands, &[], 48000.0);
        assert!(cache.sum().is_empty());
        assert!(cache.band(0).is_empty());
    }

    #[test]
    #[ignore = "manual release timing; no wall-clock assertion"]
    fn response_cache_benchmark() {
        use std::{hint::black_box, time::Instant};
        let bands: Vec<_> = (0..8).map(|i| coefficients(i as f64, 48000.0)[0]).collect();
        let grid: Vec<_> = (0..513)
            .map(|i| 20.0 * 1000.0_f64.powf(i as f64 / 512.0))
            .collect();
        let start = Instant::now();
        for _ in 0..1000 {
            for &frequency in black_box(&grid) {
                black_box(
                    bands
                        .iter()
                        .map(|c| c.response_db(frequency, 48000.0))
                        .sum::<f64>(),
                );
            }
        }
        let direct = start.elapsed();
        let mut cache = ResponseCache::default();
        cache.update(&bands, &grid, 48000.0);
        let start = Instant::now();
        for _ in 0..1000 {
            cache.update(black_box(&bands), black_box(&grid), 48000.0);
            black_box(cache.sum());
        }
        eprintln!(
            "EQ response 1000 frames: direct={direct:?}, cached={:?}",
            start.elapsed()
        );
    }
}
