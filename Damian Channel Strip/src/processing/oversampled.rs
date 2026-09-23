use super::path::cascade;
use crate::dsp::BandRuntime;
use pleasant_eq::OversampledEq;

pub struct Oversampled(OversampledEq);

impl Oversampled {
    pub(super) fn new() -> Self {
        Self(OversampledEq::new())
    }

    pub(super) fn reset(&mut self) {
        self.0.reset();
    }

    #[allow(dead_code)]
    pub(super) fn tick(&mut self, x: [f64; 2], bands: &mut [BandRuntime], sr: f64) -> [f64; 2] {
        self.tick_dual(x, bands, &mut [], sr, 1.0, 1.0)
    }

    pub(super) fn tick_dual(
        &mut self,
        x: [f64; 2],
        bands1: &mut [BandRuntime],
        bands2: &mut [BandRuntime],
        sr: f64,
        eq1_mix: f64,
        eq2_mix: f64,
    ) -> [f64; 2] {
        self.0.process(x, sr, |interpolated, oversampled_rate| {
            let x1 = cascade(interpolated, bands1, oversampled_rate);
            let mixed1 = [
                interpolated[0] + eq1_mix * (x1[0] - interpolated[0]),
                interpolated[1] + eq1_mix * (x1[1] - interpolated[1]),
            ];
            let x2 = cascade(mixed1, bands2, oversampled_rate);
            [
                mixed1[0] + eq2_mix * (x2[0] - mixed1[0]),
                mixed1[1] + eq2_mix * (x2[1] - mixed1[1]),
            ]
        })
    }
}
