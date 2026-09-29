use super::{
    config::{Config, ProcessingMode},
    Delay, LinearPhase, Oversampled,
};
use crate::{band::Band, dsp::BandRuntime};

pub enum EqPath {
    Direct,
    Natural(Box<Oversampled>),
    #[cfg(test)]
    Linear(Box<LinearPhase>),
    LinearDual(Box<LinearPhaseBanks>),
}

pub struct LinearPhaseBanks {
    eq1: LinearPhase,
    eq2: LinearPhase,
    combined: LinearPhase,
    dry: Delay,
}

impl EqPath {
    #[cfg(test)]
    pub fn new(bands: &[Band], sr: f64, config: Config) -> Self {
        match config.mode {
            ProcessingMode::ZeroLatency => Self::Direct,
            ProcessingMode::NaturalPhase => Self::Natural(Box::new(Oversampled::new())),
            ProcessingMode::LinearPhase => {
                Self::Linear(Box::new(LinearPhase::new(bands, sr, config.resolution)))
            }
        }
    }

    pub fn new_dual(bands1: &[Band], bands2: &[Band], sr: f64, config: Config) -> Self {
        match config.mode {
            ProcessingMode::ZeroLatency => Self::Direct,
            ProcessingMode::NaturalPhase => Self::Natural(Box::new(Oversampled::new())),
            ProcessingMode::LinearPhase => {
                let combined: Vec<_> = bands1.iter().chain(bands2).cloned().collect();
                Self::LinearDual(Box::new(LinearPhaseBanks {
                    eq1: LinearPhase::new(bands1, sr, config.resolution),
                    eq2: LinearPhase::new(bands2, sr, config.resolution),
                    combined: LinearPhase::new(&combined, sr, config.resolution),
                    dry: Delay::new(config.latency(sr)),
                }))
            }
        }
    }

    #[cfg(test)]
    pub fn tick(&mut self, input: [f64; 2], bands: &mut [BandRuntime], sr: f64) -> [f64; 2] {
        match self {
            Self::Direct => cascade(input, bands, sr),
            Self::Natural(p) => p.tick(input, bands, sr),
            Self::Linear(p) => p.tick(input),
            Self::LinearDual(p) => p.tick(input, 1.0, 1.0),
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
            #[cfg(test)]
            Self::Linear(p) => p.tick(input),
            Self::LinearDual(p) => p.tick(input, eq1_mix, eq2_mix),
        }
    }

    pub fn reset(&mut self) {
        match self {
            Self::Direct => {}
            Self::Natural(p) => p.reset(),
            #[cfg(test)]
            Self::Linear(p) => p.reset(),
            Self::LinearDual(p) => {
                p.eq1.reset();
                p.eq2.reset();
                p.combined.reset();
                p.dry.reset();
            }
        }
    }
}

impl LinearPhaseBanks {
    fn tick(&mut self, input: [f64; 2], eq1_mix: f64, eq2_mix: f64) -> [f64; 2] {
        let eq1 = self.eq1.tick(input);
        let eq2 = self.eq2.tick(input);
        let combined = self.combined.tick(input);
        let dry = self.dry.tick(input);
        std::array::from_fn(|ch| {
            (1.0 - eq1_mix) * (1.0 - eq2_mix) * dry[ch]
                + eq1_mix * (1.0 - eq2_mix) * eq1[ch]
                + (1.0 - eq1_mix) * eq2_mix * eq2[ch]
                + eq1_mix * eq2_mix * combined[ch]
        })
    }
}

pub(super) fn cascade(mut x: [f64; 2], bands: &mut [BandRuntime], sr: f64) -> [f64; 2] {
    for b in bands {
        x = b.tick(x, sr);
    }
    x
}
