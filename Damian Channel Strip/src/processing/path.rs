use super::{
    config::{Config, ProcessingMode},
    LinearPhase, Oversampled,
};
use crate::{band::Band, dsp::BandRuntime};

pub enum EqPath {
    Direct,
    Natural(Box<Oversampled>),
    Linear(Box<LinearPhase>),
}

impl EqPath {
    pub fn new(bands: &[Band], sr: f64, config: Config) -> Self {
        match config.mode {
            ProcessingMode::ZeroLatency => Self::Direct,
            ProcessingMode::NaturalPhase => Self::Natural(Box::new(Oversampled::new())),
            ProcessingMode::LinearPhase => {
                Self::Linear(Box::new(LinearPhase::new(bands, sr, config.resolution)))
            }
        }
    }

    #[allow(dead_code)]
    pub fn tick(&mut self, input: [f64; 2], bands: &mut [BandRuntime], sr: f64) -> [f64; 2] {
        match self {
            Self::Direct => cascade(input, bands, sr),
            Self::Natural(p) => p.tick(input, bands, sr),
            Self::Linear(p) => p.tick(input),
        }
    }

    pub fn tick_dual(
        &mut self,
        input: [f64; 2],
        bands1: &mut [BandRuntime],
        bands2: &mut [BandRuntime],
        sr: f64,
        eq1_mix: f64,
        eq2_mix: f64,
    ) -> [f64; 2] {
        match self {
            Self::Direct => {
                let x1 = cascade(input, bands1, sr);
                let eff_x1 = [
                    input[0] + eq1_mix * (x1[0] - input[0]),
                    input[1] + eq1_mix * (x1[1] - input[1]),
                ];
                let x2 = cascade(eff_x1, bands2, sr);
                [
                    eff_x1[0] + eq2_mix * (x2[0] - eff_x1[0]),
                    eff_x1[1] + eq2_mix * (x2[1] - eff_x1[1]),
                ]
            }
            Self::Natural(p) => p.tick_dual(input, bands1, bands2, sr, eq1_mix, eq2_mix),
            Self::Linear(p) => {
                let out = p.tick(input);
                let total_eq_mix = eq1_mix.max(eq2_mix);
                [
                    input[0] + total_eq_mix * (out[0] - input[0]),
                    input[1] + total_eq_mix * (out[1] - input[1]),
                ]
            }
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::Direct => {}
            Self::Natural(p) => p.reset(),
            Self::Linear(p) => p.reset(),
        }
    }
}

pub(super) fn cascade(mut x: [f64; 2], bands: &mut [BandRuntime], sr: f64) -> [f64; 2] {
    for b in bands {
        x = b.tick(x, sr);
    }
    x
}
