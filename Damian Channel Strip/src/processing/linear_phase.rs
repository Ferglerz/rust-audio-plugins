use super::config::Resolution;
use crate::band::Band;
use pleasant_eq::LinearPhaseEq;

pub struct LinearPhase(LinearPhaseEq);

impl LinearPhase {
    pub(super) fn new(bands: &[Band], sr: f64, resolution: Resolution) -> Self {
        let settings: Vec<_> = bands.iter().map(Into::into).collect();
        Self(LinearPhaseEq::prepare(&settings, sr, resolution.order(sr)))
    }

    pub(super) fn reset(&mut self) {
        self.0.reset();
    }

    pub(super) fn tick(&mut self, x: [f64; 2]) -> [f64; 2] {
        self.0.process_sample(x)
    }
}
