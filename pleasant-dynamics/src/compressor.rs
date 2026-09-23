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

#[derive(Default)]
pub struct VocalComp {
    envelope: [f64; 3],
    reduction: [f64; 3],
    uncapped_reduction: [f64; 3],
    pse: [Pse; 3],
    makeup: f64,
    link_mix: f64,
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

    fn update_link_mix(&mut self, stereo_link: bool, sample_rate: f64) {
        self.link_mix +=
            (1.0 - (-1.0 / (sample_rate * 0.010)).exp()) * (f64::from(stereo_link) - self.link_mix);
    }

    fn linked(&self, channel: f64, stereo: f64) -> f64 {
        channel + self.link_mix * (stereo - channel)
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
        self.update_link_mix(settings.stereo_link, sample_rate);
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
        self.update_link_mix(settings.stereo_link, sample_rate);
        let peaks = Self::stereo_peaks(sidechain);
        let makeup_target = if settings.auto_makeup {
            -settings.threshold * 0.35 * (1.0 - 1.0 / settings.ratio.clamp(1.0, 20.0)) / 0.75
        } else {
            0.0
        };
        self.makeup += (1.0 - (-1.0 / (sample_rate * 0.020)).exp()) * (makeup_target - self.makeup);
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
            let unlimited = Self::target_reduction(level, settings);
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
            self.reduction[index] =
                coefficient * self.reduction[index] + (1.0 - coefficient) * target;
            let uncapped_coefficient = (-1.0
                / (sample_rate
                    * if unlimited > self.uncapped_reduction[index] {
                        attack
                    } else {
                        release
                    }))
            .exp();
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
        self.update_link_mix(settings.stereo_link, sample_rate);
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
}
