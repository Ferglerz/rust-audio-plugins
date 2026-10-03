use crate::{pse::Pse, PseSettings, WallSettings};
use pleasant_dsp::units::{db_to_linear, linear_to_db_with_floor};

#[derive(Clone, Copy, Debug)]
pub struct CompSettings {
    pub threshold: f64,
    pub ratio: f64,
    pub attack: f64,
    pub release: f64,
    pub knee: f64,
    pub depth: f64,
    pub auto_makeup: bool,
    pub stereo_link: bool,
    pub gate: f64,
    pub pse: PseSettings,
    pub wall: WallSettings,
    pub dry: f64,
    pub wet: f64,
    pub output_gain: f64,
}

impl Default for CompSettings {
    fn default() -> Self {
        Self {
            threshold: 0.0,
            ratio: 4.0,
            attack: 2.0,
            release: 120.0,
            knee: 6.0,
            depth: 30.0,
            auto_makeup: true,
            stereo_link: true,
            gate: -80.0,
            pse: PseSettings::default(),
            wall: WallSettings::default(),
            dry: 0.0,
            wet: 1.0,
            output_gain: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
struct CoefficientCache {
    sample_rate: f64,
    attack: f64,
    release: f64,
    envelope_attack: f64,
    envelope_release: f64,
    makeup: f64,
    link: f64,
    compression_attack: f64,
    compression_release: f64,
}

#[derive(Default)]
pub struct VocalComp {
    envelope: [f64; 3],
    reduction: [f64; 3],
    uncapped_reduction: [f64; 3],
    pse: [Pse; 3],
    makeup: f64,
    link_mix: f64,
    coefficients: Option<CoefficientCache>,
}

impl VocalComp {
    pub fn new() -> Self {
        Self {
            link_mix: 1.0,
            ..Self::default()
        }
    }

    fn target_reduction(level: f64, settings: CompSettings) -> f64 {
        let over = level - settings.threshold;
        let slope = 1.0 - 1.0 / settings.ratio.clamp(1.0, 20.0);
        if settings.threshold >= -0.0001 {
            return 0.0;
        }
        let knee = settings.knee.max(0.0);
        if knee <= 0.001 {
            over.max(0.0) * slope
        } else if over <= -knee / 2.0 {
            0.0
        } else if over < knee / 2.0 {
            slope * (over + knee / 2.0).powi(2) / (2.0 * knee)
        } else {
            over * slope
        }
    }

    fn stereo_peaks(input: [f64; 2]) -> [f64; 3] {
        let left = input[0].abs();
        let right = input[1].abs();
        [left, right, left.max(right)]
    }

    fn update_link_mix(&mut self, stereo_link: bool, coefficient: f64) {
        self.link_mix += coefficient * (f64::from(stereo_link) - self.link_mix);
    }

    fn linked(&self, channel: f64, stereo: f64) -> f64 {
        channel + self.link_mix * (stereo - channel)
    }

    fn coefficients(&mut self, sample_rate: f64, settings: CompSettings) -> CoefficientCache {
        let attack = (settings.attack * 0.001).max(0.00005);
        let release = (settings.release * 0.001).max(0.001);
        if let Some(cache) = self.coefficients {
            if cache.sample_rate == sample_rate && cache.attack == attack && cache.release == release
            {
                return cache;
            }
        }

        let rate_changed = self
            .coefficients
            .is_none_or(|cached| cached.sample_rate != sample_rate);
        let mut cache = self.coefficients.unwrap_or(CoefficientCache {
            sample_rate,
            attack,
            release,
            envelope_attack: 0.0,
            envelope_release: 0.0,
            makeup: 0.0,
            link: 0.0,
            compression_attack: 0.0,
            compression_release: 0.0,
        });
        if rate_changed {
            cache.sample_rate = sample_rate;
            cache.envelope_attack = (-1.0 / (sample_rate * 0.003)).exp();
            cache.envelope_release = (-1.0 / (sample_rate * 0.100)).exp();
            cache.makeup = 1.0 - (-1.0 / (sample_rate * 0.020)).exp();
            cache.link = 1.0 - (-1.0 / (sample_rate * 0.010)).exp();
        }
        if rate_changed || cache.attack != attack {
            cache.attack = attack;
            cache.compression_attack = (-1.0 / (sample_rate * attack)).exp();
        }
        if rate_changed || cache.release != release {
            cache.release = release;
            cache.compression_release = (-1.0 / (sample_rate * release)).exp();
        }
        self.coefficients = Some(cache);
        cache
    }

    fn pse_gains(
        &mut self,
        peaks: [f64; 3],
        settings: CompSettings,
        sample_rate: f64,
    ) -> ([f64; 3], f64) {
        let mut gains = [0.0; 3];
        for index in 0..3 {
            gains[index] =
                self.pse[index].process(peaks[index], settings.gate, sample_rate, settings.pse);
        }
        let mut reduction = 0.0_f64;
        for index in 0..2 {
            reduction = reduction.max(self.linked(
                self.pse[index].reduction_db.abs(),
                self.pse[2].reduction_db.abs(),
            ));
        }
        (gains, reduction)
    }

    pub fn tick_pse(
        &mut self,
        input: [f64; 2],
        settings: CompSettings,
        sample_rate: f64,
    ) -> ([f64; 2], f64) {
        let coefficients = self.coefficients(sample_rate, settings);
        self.update_link_mix(settings.stereo_link, coefficients.link);
        let (gains, reduction) = self.pse_gains(Self::stereo_peaks(input), settings, sample_rate);
        (
            [
                input[0] * self.linked(gains[0], gains[2]),
                input[1] * self.linked(gains[1], gains[2]),
            ],
            reduction,
        )
    }

    pub fn tick_comp(
        &mut self,
        input: [f64; 2],
        sidechain: [f64; 2],
        settings: CompSettings,
        sample_rate: f64,
    ) -> ([f64; 2], f64, f64) {
        let coefficients = self.coefficients(sample_rate, settings);
        self.update_link_mix(settings.stereo_link, coefficients.link);
        let peaks = Self::stereo_peaks(sidechain);
        let makeup_target = if settings.auto_makeup {
            -settings.threshold * 0.35 * (1.0 - 1.0 / settings.ratio.clamp(1.0, 20.0)) / 0.75
        } else {
            0.0
        };
        self.makeup += coefficients.makeup * (makeup_target - self.makeup);
        let mut gains = [0.0; 3];
        for index in 0..3 {
            let envelope_coefficient = if peaks[index] > self.envelope[index] {
                coefficients.envelope_attack
            } else {
                coefficients.envelope_release
            };
            self.envelope[index] = envelope_coefficient * self.envelope[index]
                + (1.0 - envelope_coefficient) * peaks[index];
            let level = linear_to_db_with_floor(self.envelope[index], 1.0e-12);
            let unlimited = Self::target_reduction(level, settings);
            let target = unlimited.min(settings.depth);
            let coefficient = if target > self.reduction[index] {
                coefficients.compression_attack
            } else {
                coefficients.compression_release
            };
            self.reduction[index] =
                coefficient * self.reduction[index] + (1.0 - coefficient) * target;
            let uncapped_coefficient = if unlimited > self.uncapped_reduction[index] {
                coefficients.compression_attack
            } else {
                coefficients.compression_release
            };
            self.uncapped_reduction[index] = uncapped_coefficient * self.uncapped_reduction[index]
                + (1.0 - uncapped_coefficient) * unlimited;
            gains[index] = db_to_linear(self.makeup - self.reduction[index]);
        }
        let mut output = [0.0; 2];
        let mut reduction = 0.0_f64;
        for index in 0..2 {
            let gain = self.linked(gains[index], gains[2]);
            output[index] = input[index] * (settings.dry + settings.wet * gain);
            reduction = reduction.max(self.linked(self.reduction[index], self.reduction[2]));
        }
        if settings.pse.listen {
            output = sidechain;
        }
        (
            output,
            reduction,
            linear_to_db_with_floor(self.envelope[2], 1.0e-12),
        )
    }

    pub fn tick(
        &mut self,
        input: [f64; 2],
        sidechain: [f64; 2],
        settings: CompSettings,
        sample_rate: f64,
    ) -> ([f64; 2], f64, f64, f64) {
        let coefficients = self.coefficients(sample_rate, settings);
        self.update_link_mix(settings.stereo_link, coefficients.link);
        let (pse_gains, pse_reduction) =
            self.pse_gains(Self::stereo_peaks(sidechain), settings, sample_rate);
        let gated = [
            input[0] * self.linked(pse_gains[0], pse_gains[2]),
            input[1] * self.linked(pse_gains[1], pse_gains[2]),
        ];
        let (output, reduction, sidechain_level) =
            self.tick_comp(gated, sidechain, settings, sample_rate);
        (output, reduction, sidechain_level, pse_reduction)
    }

    pub fn uncapped_gr(&self) -> f64 {
        let mut reduction = 0.0_f64;
        for index in 0..2 {
            reduction = reduction.max(
                self.uncapped_reduction[index]
                    + self.link_mix * (self.uncapped_reduction[2] - self.uncapped_reduction[index]),
            );
        }
        reduction
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct UncachedReference {
        envelope: [f64; 3],
        reduction: [f64; 3],
        uncapped_reduction: [f64; 3],
        makeup: f64,
        link_mix: f64,
    }

    impl UncachedReference {
        fn tick_comp(
            &mut self,
            input: [f64; 2],
            sidechain: [f64; 2],
            settings: CompSettings,
            sample_rate: f64,
        ) -> ([f64; 2], f64, f64) {
            self.link_mix += (1.0 - (-1.0 / (sample_rate * 0.010)).exp())
                * (f64::from(settings.stereo_link) - self.link_mix);
            let peaks = VocalComp::stereo_peaks(sidechain);
            let makeup_target = if settings.auto_makeup {
                -settings.threshold * 0.35
                    * (1.0 - 1.0 / settings.ratio.clamp(1.0, 20.0))
                    / 0.75
            } else {
                0.0
            };
            self.makeup +=
                (1.0 - (-1.0 / (sample_rate * 0.020)).exp()) * (makeup_target - self.makeup);

            let mut gains = [0.0; 3];
            for index in 0..3 {
                let envelope_coefficient = (-1.0
                    / (sample_rate
                        * if peaks[index] > self.envelope[index] {
                            0.003
                        } else {
                            0.100
                        }))
                .exp();
                self.envelope[index] = envelope_coefficient * self.envelope[index]
                    + (1.0 - envelope_coefficient) * peaks[index];
                let level = linear_to_db_with_floor(self.envelope[index], 1.0e-12);
                let unlimited = VocalComp::target_reduction(level, settings);
                let target = unlimited.min(settings.depth);
                let attack = (settings.attack * 0.001).max(0.00005);
                let release = (settings.release * 0.001).max(0.001);
                let coefficient = (-1.0
                    / (sample_rate
                        * if target > self.reduction[index] {
                            attack
                        } else {
                            release
                        }))
                .exp();
                self.reduction[index] = coefficient * self.reduction[index]
                    + (1.0 - coefficient) * target;
                let uncapped_coefficient = (-1.0
                    / (sample_rate
                        * if unlimited > self.uncapped_reduction[index] {
                            attack
                        } else {
                            release
                        }))
                .exp();
                self.uncapped_reduction[index] = uncapped_coefficient
                    * self.uncapped_reduction[index]
                    + (1.0 - uncapped_coefficient) * unlimited;
                gains[index] = db_to_linear(self.makeup - self.reduction[index]);
            }

            let mut output = [0.0; 2];
            let mut reduction = 0.0_f64;
            for index in 0..2 {
                let gain = gains[index] + self.link_mix * (gains[2] - gains[index]);
                output[index] = input[index] * (settings.dry + settings.wet * gain);
                reduction = reduction.max(
                    self.reduction[index]
                        + self.link_mix * (self.reduction[2] - self.reduction[index]),
                );
            }
            if settings.pse.listen {
                output = sidechain;
            }
            (
                output,
                reduction,
                linear_to_db_with_floor(self.envelope[2], 1.0e-12),
            )
        }
    }

    #[test]
    fn depth_limits_reduction() {
        let mut compressor = VocalComp::new();
        let settings = CompSettings {
            threshold: -30.0,
            ratio: 20.0,
            depth: 5.0,
            auto_makeup: false,
            ..CompSettings::default()
        };
        let mut reduction = 0.0;
        for index in 0..48_000 {
            let sample = (std::f64::consts::TAU * 1_000.0 * index as f64 / 48_000.0).sin() * 0.9;
            reduction = compressor
                .tick([sample; 2], [sample; 2], settings, 48_000.0)
                .1;
        }
        assert!((reduction - 5.0).abs() < 0.2);
        assert!(compressor.uncapped_gr() > reduction + 5.0);
    }

    #[test]
    fn memoized_coefficients_match_uncached_reference_across_parameter_changes() {
        let mut compressor = VocalComp::new();
        let mut reference = UncachedReference {
            link_mix: 1.0,
            ..UncachedReference::default()
        };
        let scenarios = [
            (48_000.0, 2.0, 120.0, -24.0, true),
            (48_000.0, 8.0, 35.0, -18.0, false),
            (96_000.0, 8.0, 35.0, -18.0, false),
            (44_100.0, 0.0, 0.0, -30.0, true),
        ];

        for (sample_rate, attack, release, threshold, auto_makeup) in scenarios {
            for frame in 0..256 {
                let settings = CompSettings {
                    threshold,
                    ratio: 8.0,
                    attack: attack + frame as f64 * 0.031,
                    release: release + (frame / 32) as f64 * 7.0,
                    depth: 12.0,
                    auto_makeup,
                    stereo_link: frame % 47 < 23,
                    dry: 0.1,
                    wet: 0.9,
                    ..CompSettings::default()
                };
                let left = (frame as f64 * 0.071).sin() * 0.8;
                let right = (frame as f64 * 0.043).cos() * 0.6;
                let input = [left, right];
                let sidechain = [left * 0.7, right * 0.9];
                let actual = compressor.tick_comp(input, sidechain, settings, sample_rate);
                let expected = reference.tick_comp(input, sidechain, settings, sample_rate);
                assert_eq!(actual, expected);
                assert_eq!(compressor.envelope, reference.envelope);
                assert_eq!(compressor.reduction, reference.reduction);
                assert_eq!(compressor.uncapped_reduction, reference.uncapped_reduction);
                assert_eq!(compressor.makeup, reference.makeup);
                assert_eq!(compressor.link_mix, reference.link_mix);
            }
        }
    }

    #[test]
    #[ignore = "manual release timing benchmark; run with --release --ignored --nocapture"]
    fn compressor_cache_release_benchmark() {
        use std::hint::black_box;
        use std::time::Instant;

        const SAMPLES: usize = 500_000;
        const SAMPLE_RATE: f64 = 48_000.0;
        let settings = CompSettings {
            threshold: -24.0,
            ratio: 8.0,
            attack: 2.0,
            release: 120.0,
            depth: 18.0,
            auto_makeup: true,
            ..CompSettings::default()
        };
        let mut cached = VocalComp::new();
        let mut uncached = UncachedReference {
            link_mix: 1.0,
            ..UncachedReference::default()
        };
        let mut cached_checksum = [0.0; 3];
        let mut uncached_checksum = [0.0; 3];

        let cached_start = Instant::now();
        for index in 0..SAMPLES {
            let level = if index % 16_384 < 8_192 { 0.65 } else { 0.08 };
            let output = cached.tick_comp(
                black_box([level, level * 0.7]),
                black_box([level, level * 0.7]),
                black_box(settings),
                black_box(SAMPLE_RATE),
            );
            cached_checksum[0] += black_box(output.0[0]);
            cached_checksum[1] += black_box(output.0[1]);
            cached_checksum[2] += black_box(output.1);
        }
        let cached_elapsed = cached_start.elapsed();

        let uncached_start = Instant::now();
        for index in 0..SAMPLES {
            let level = if index % 16_384 < 8_192 { 0.65 } else { 0.08 };
            let output = uncached.tick_comp(
                black_box([level, level * 0.7]),
                black_box([level, level * 0.7]),
                black_box(settings),
                black_box(SAMPLE_RATE),
            );
            uncached_checksum[0] += black_box(output.0[0]);
            uncached_checksum[1] += black_box(output.0[1]);
            uncached_checksum[2] += black_box(output.1);
        }
        let uncached_elapsed = uncached_start.elapsed();

        assert_eq!(cached_checksum, uncached_checksum);
        assert_eq!(cached.envelope, uncached.envelope);
        assert_eq!(cached.reduction, uncached.reduction);
        assert_eq!(cached.uncapped_reduction, uncached.uncapped_reduction);
        println!(
            "compressor samples={SAMPLES}: memoized={cached_elapsed:?}, uncached={uncached_elapsed:?}"
        );
    }
}
