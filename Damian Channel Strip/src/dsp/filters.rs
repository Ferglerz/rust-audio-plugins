use crate::band::{Band, Shape};
use pleasant_dsp::{
    filters::{BiquadCoefficients, BiquadKind, TransposedDirectForm2},
    units::{db_to_linear, linear_to_db_with_floor},
};
use pleasant_eq::{BandCoefficients, BandProcessor};

pub fn db_gain(db: f64) -> f64 {
    db_to_linear(db)
}

pub fn gain_db(x: f64) -> f64 {
    linear_to_db_with_floor(x, 1e-12)
}

fn biquad_kind(shape: Shape) -> BiquadKind {
    pleasant_eq::EqShape::from(shape).biquad_kind()
}

#[derive(Clone, Copy, Debug)]
pub struct Coeff(BiquadCoefficients);

impl Coeff {
    pub fn make(shape: Shape, freq: f64, gain: f64, q: f64, sr: f64) -> Self {
        Self(BiquadCoefficients::design(
            biquad_kind(shape),
            freq,
            gain,
            q,
            sr,
        ))
    }

    pub fn bandpass(freq: f64, q: f64, sr: f64) -> Self {
        Self(BiquadCoefficients::design(
            BiquadKind::BandPass,
            freq,
            0.0,
            q,
            sr,
        ))
    }

    pub fn response(self, f: f64, sr: f64) -> f64 {
        self.0.response_db(f, sr)
    }
}

impl std::ops::Deref for Coeff {
    type Target = BiquadCoefficients;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Fixed-size cascade shared by the audio path and plotted response.
#[derive(Clone, Copy)]
pub struct BandCoeffs(BandCoefficients);

impl BandCoeffs {
    pub fn make(b: &Band, sr: f64) -> Self {
        Self(BandCoefficients::prepare(&b.into(), sr))
    }

    pub fn response(self, f: f64, sr: f64) -> f64 {
        self.0.response_db(f, sr)
    }
}

#[derive(Clone, Copy, Default)]
pub struct Cascade {
    stages: [TransposedDirectForm2; 4],
}

impl Cascade {
    pub fn tick(&mut self, x: f64, coeffs: BandCoeffs) -> f64 {
        coeffs.0.process(&mut self.stages, x)
    }
}

#[derive(Clone, Copy, Default)]
pub struct Filter(TransposedDirectForm2);

impl Filter {
    pub fn tick(&mut self, x: f64, c: Coeff) -> f64 {
        self.0.process(x, c.0)
    }
}

#[derive(Clone)]
pub struct BandRuntime {
    pub band: Band,
    processor: BandProcessor,
    pub reduction: f64,
    pub reduction_uncapped: f64,
}

impl BandRuntime {
    pub fn new(mut b: Band, sr: f64) -> Self {
        b.sanitize();
        Self {
            processor: BandProcessor::new((&b).into(), sr),
            reduction: 0.0,
            reduction_uncapped: 0.0,
            band: b,
        }
    }

    pub fn inherit(&mut self, old: &Self) {
        self.processor.inherit_state_from(&old.processor);
        self.reduction = old.reduction;
        self.reduction_uncapped = old.reduction_uncapped;
    }

    pub fn reset(&mut self) {
        self.processor.reset();
        self.reduction = 0.0;
        self.reduction_uncapped = 0.0;
    }

    pub fn tick(&mut self, x: [f64; 2], sr: f64) -> [f64; 2] {
        let out = self.processor.process(x, sr);
        self.reduction = self.processor.reduction_db();
        self.reduction_uncapped = self.processor.uncapped_reduction_db();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn cut_orders_match_butterworth_response() {
        let sr = 48000.0;
        for shape in [Shape::LowCut, Shape::HighCut] {
            for order in 1..=8 {
                let band = Band {
                    shape,
                    order,
                    freq: 1000.0,
                    q: std::f64::consts::FRAC_1_SQRT_2,
                    ..Band::default()
                };
                let c = BandCoeffs::make(&band, sr);
                for f in [50.0, 500.0, 1000.0, 2000.0, 10000.0] {
                    let mut ratio = (PI * f / sr).tan() / (PI * band.freq / sr).tan();
                    if shape == Shape::LowCut {
                        ratio = 1.0 / ratio;
                    }
                    let expected = -10.0 * (1.0 + ratio.powi(2 * order as i32)).log10();
                    assert!(
                        (c.response(f, sr) - expected).abs() < 1e-5,
                        "{shape:?}, order {order}, {f} Hz"
                    );
                }
            }
        }
    }

    #[test]
    fn second_order_preserves_existing_resonance() {
        for shape in [Shape::LowCut, Shape::HighCut] {
            for q in [0.15, 0.707, 1.0, 18.0] {
                let b = Band {
                    shape,
                    q,
                    ..Band::default()
                };
                let old = Coeff::make(shape, b.freq, b.gain, q, 48000.0);
                let new = BandCoeffs::make(&b, 48000.0);
                for f in [50.0, 1000.0, 10000.0] {
                    assert!((old.response(f, 48000.0) - new.response(f, 48000.0)).abs() < 1e-8);
                }
            }
        }
    }

    #[test]
    fn runtime_matches_plotted_response() {
        let sr = 48000.0;
        for shape in Shape::ALL {
            for order in 1..=8 {
                let b = Band {
                    shape,
                    order,
                    gain: 6.0,
                    q: 0.707,
                    ..Band::default()
                };
                let coeff = BandCoeffs::make(&b, sr);
                let mut runtime = BandRuntime::new(b, sr);
                let frequencies = [500.0, 1000.0, 2000.0];
                let mut re = [0.0; 3];
                let mut im = [0.0; 3];
                for n in 0..8192 {
                    let out = runtime.tick([if n == 0 { 1.0 } else { 0.0 }; 2], sr);
                    assert_eq!(out[0], out[1]);
                    for (i, f) in frequencies.iter().enumerate() {
                        let w = 2.0 * PI * f * n as f64 / sr;
                        re[i] += out[0] * w.cos();
                        im[i] -= out[0] * w.sin();
                    }
                }
                for (i, f) in frequencies.iter().enumerate() {
                    let measured = re[i].hypot(im[i]);
                    assert!(
                        (measured - db_gain(coeff.response(*f, sr))).abs() < 1e-6,
                        "{shape:?}, {order}, {f}"
                    );
                }
            }
        }
    }

    #[test]
    fn cascades_are_stable_at_extremes() {
        for sr in [8000.0, 44100.0, 48000.0, 96000.0, 192000.0] {
            for shape in [Shape::LowCut, Shape::HighCut, Shape::BandPass] {
                for order in 1..=8 {
                    for freq in [20.0, 20000.0] {
                        for q in [0.15, 18.0] {
                            let b = Band {
                                shape,
                                order,
                                freq,
                                q,
                                ..Band::default()
                            };
                            let c = BandCoeffs::make(&b, sr);
                            let mut filter = Cascade::default();
                            for n in 0..4096 {
                                let out = filter.tick(if n == 0 { 1.0 } else { 0.0 }, c);
                                assert!(out.is_finite() && out.abs() < 100.0);
                                assert!(filter.stages.iter().all(TransposedDirectForm2::is_finite));
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn order_changes_do_not_inherit_incompatible_filter_state() {
        let b = Band {
            shape: Shape::LowCut,
            ..Band::default()
        };
        let mut old = BandRuntime::new(b.clone(), 48000.0);
        old.tick([1.0; 2], 48000.0);
        let mut new = BandRuntime::new(Band { order: 8, ..b }, 48000.0);
        new.inherit(&old);
        assert_eq!(new.processor.coefficient_stage_count(), 4);
        assert_eq!(new.tick([0.0; 2], 48000.0), [0.0; 2]);
    }

    #[test]
    fn bell_center_and_shelves() {
        assert!(
            (Coeff::make(Shape::Bell, 1000.0, 6.0, 1.0, 48000.0).response(1000.0, 48000.0) - 6.0)
                .abs()
                < 0.001
        );
        assert!(
            (Coeff::make(Shape::LowShelf, 200.0, 6.0, 0.707, 48000.0).response(1.0, 48000.0) - 6.0)
                .abs()
                < 0.01
        );
        assert!(
            Coeff::make(Shape::LowCut, 1000.0, 0.0, 0.707, 48000.0).response(50.0, 48000.0) < -50.0
        );
    }

    #[test]
    fn all_filters_finite_at_extremes() {
        for sr in [8000.0, 44100.0, 48000.0, 96000.0, 192000.0] {
            for shape in [
                Shape::Bell,
                Shape::LowShelf,
                Shape::HighShelf,
                Shape::LowCut,
                Shape::HighCut,
                Shape::Notch,
            ] {
                for f in [20.0, 20000.0] {
                    for q in [0.15, 18.0] {
                        let c = Coeff::make(shape, f, 24.0, q, sr);
                        let mut filter = Filter::default();
                        for i in 0..12000 {
                            let out = filter.tick(if i == 0 { 1.0 } else { 0.0 }, c);
                            assert!(out.is_finite() && out.abs() < 1000.0);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn dynamic_band_compresses_and_releases() {
        let mut b = BandRuntime::new(
            Band {
                dynamic: true,
                threshold: -30.0,
                attack: 1.0,
                release: 10.0,
                ..Band::default()
            },
            48000.0,
        );
        for i in 0..48000 {
            let v = (2.0 * PI * 1000.0 * i as f64 / 48000.0).sin() * 0.5;
            b.tick([v, v], 48000.0);
        }
        assert!(b.reduction > 5.0);
        for _ in 0..48000 {
            b.tick([0.0; 2], 48000.0);
        }
        assert!(b.reduction < 0.01);
    }

    #[test]
    fn dynamic_band_boosts_transient_gain_when_range_negative() {
        let mut b = BandRuntime::new(
            Band {
                dynamic: true,
                threshold: -30.0,
                attack: 1.0,
                release: 10.0,
                range: -8.0,
                ..Band::default()
            },
            48000.0,
        );
        for i in 0..48000 {
            let v = (2.0 * PI * 1000.0 * i as f64 / 48000.0).sin() * 0.5;
            b.tick([v, v], 48000.0);
        }
        assert!(b.reduction < -5.0);
        assert!(b.reduction >= -8.0);
        assert!(b.processor.current_gain_db() > 5.0);
        for _ in 0..48000 {
            b.tick([0.0; 2], 48000.0);
        }
        assert!(b.reduction.abs() < 0.01);
    }

    #[test]
    fn dynamic_band_uncapped_reduction_exceeds_range() {
        let mut cut = BandRuntime::new(
            Band {
                dynamic: true,
                threshold: -30.0,
                attack: 1.0,
                release: 10.0,
                range: 3.0,
                ratio: 10.0,
                ..Band::default()
            },
            48000.0,
        );
        let mut boost = BandRuntime::new(
            Band {
                dynamic: true,
                threshold: -30.0,
                attack: 1.0,
                release: 10.0,
                range: -3.0,
                ratio: 10.0,
                ..Band::default()
            },
            48000.0,
        );
        for i in 0..48000 {
            let v = (2.0 * PI * 1000.0 * i as f64 / 48000.0).sin() * 0.5;
            cut.tick([v, v], 48000.0);
            boost.tick([v, v], 48000.0);
        }
        assert!((cut.reduction - 3.0).abs() < 0.05);
        assert!(cut.reduction_uncapped > 8.0);
        assert!((boost.reduction + 3.0).abs() < 0.05);
        assert!(boost.reduction_uncapped < -8.0);
    }
}
