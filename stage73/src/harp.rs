use crate::{calibration::KEYS, engine::LOW, era::Era, field::Field};
use pleasant_dsp::sympathetic::{ReceiverSpec, Sympathetic};

pub(crate) struct Harp {
    pub modes: Sympathetic<73>,
    voicing: Option<(Era, f64, f64)>,
}

impl Harp {
    pub(crate) fn new(rate: f64) -> Self {
        // Provisional frame frequency, bandwidth and transfer. Each receiving
        // tine uses the calibrated decay of its own fundamental.
        let modes = Sympathetic::new(rate, LOW, 0, 220., 140., 2., |i| ReceiverSpec {
            hz: 440. * 2_f64.powf((f64::from(LOW) + i as f64 - 69.) / 12.),
            decay: KEYS[i][1],
            damped_decay: 76. + 1.4 * i as f64,
            pickup: [0.; 2],
        });
        let mut harp = Self {
            modes,
            voicing: None,
        };
        harp.voice(Era::LateI, 0., 0.);
        harp
    }
    pub(crate) fn voice(&mut self, era: Era, height: f64, gap: f64) {
        if self.voicing == Some((era, height, gap)) {
            return;
        }
        self.voicing = Some((era, height, gap));
        let field = Field::shared();
        for i in 0..73 {
            let (h, g) = era.geometry(i);
            let gain = field.emf(h + height, 0., 1., 0., g + gap);
            self.modes.set_pickup(
                i,
                [
                    gain / if i < 4 { 4. } else { 3. },
                    gain / if i < 7 { 7. } else { 6. },
                ],
            );
        }
    }
}
