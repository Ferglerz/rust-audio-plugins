//! Trapezoidal companion model of the harp, bass rheostat and loaded volume wiper.
//! Component sources and the independent SPICE circuit are in research/.
pub const HARP_GROUP_FACTOR: f64 = 0.25 + 23.0 / 3.0;
pub const HARP_OHMS: f64 = 180.0 * HARP_GROUP_FACTOR;
pub const HARP_HENRIES: f64 = 0.075 * HARP_GROUP_FACTOR; // estimated inductance per pickup
pub const CABLE_FARADS: f64 = 300e-12;
pub const DEFAULT_CABLE_PF: f64 = CABLE_FARADS * 1e12;
pub const MIN_CABLE_PF: f64 = 100.;
pub const MAX_CABLE_PF: f64 = 1000.;
const LOAD_OHMS: f64 = 1e6;

pub struct Panel {
    series_r: f64,
    inductive_r: f64,
    bass_g: f64,
    cable_g: f64,
    bass_z: f64,
    wiper_z: f64,
    loop_g: f64,
    inductor_history: f64,
    bass_history: f64,
    cable_history: f64,
    rate: f64,
    setting: (f64, f64, f64),
}

impl Panel {
    pub fn new(rate: f64) -> Self {
        let mut panel = Self {
            series_r: HARP_OHMS,
            inductive_r: 2.0 * rate * HARP_HENRIES,
            bass_g: 2.0 * rate * 47e-9,
            cable_g: 2.0 * rate * (DEFAULT_CABLE_PF * 1e-12),
            bass_z: 0.0,
            wiper_z: 0.0,
            loop_g: 0.0,
            inductor_history: 0.0,
            bass_history: 0.0,
            cable_history: 0.0,
            rate,
            setting: (-1.0, -1.0, DEFAULT_CABLE_PF),
        };
        panel.set(1.0, 1.0);
        panel
    }

    pub fn set(&mut self, volume: f64, bass: f64) {
        self.set_loaded(volume, bass, DEFAULT_CABLE_PF);
    }

    pub fn set_loaded(&mut self, volume: f64, bass: f64, cable_pf: f64) {
        let cable_pf = cable_pf.clamp(MIN_CABLE_PF, MAX_CABLE_PF);
        if self.setting == (volume, bass, cable_pf) {
            return;
        }
        if cable_pf != self.setting.2 {
            // History is stored as a current. Retain its equivalent voltage
            // when the shunt capacitance changes under a held note.
            self.cable_history *= cable_pf / self.setting.2;
            self.cable_g = 2. * self.rate * (cable_pf * 1e-12);
        }
        self.setting = (volume, bass, cable_pf);
        // Unspecified tapers: cubic audio taper and its reversed counterpart.
        let bottom = 10_000.0 * volume.clamp(0.0, 1.0).powi(3);
        let bridge = 50_000.0 * (1.0 - bass.clamp(0.0, 1.0)).powi(3);
        self.bass_z = bridge / (1.0 + bridge * self.bass_g);
        self.wiper_z = bottom / (1.0 + bottom * (self.cable_g + 1.0 / LOAD_OHMS));
        self.loop_g =
            (self.series_r + self.inductive_r + self.bass_z + (10_000.0 - bottom) + self.wiper_z)
                .recip();
    }

    #[inline]
    pub fn tick(&mut self, emf: f64) -> f64 {
        let current = (emf + self.inductor_history
            - self.bass_z * self.bass_history
            - self.wiper_z * self.cable_history)
            * self.loop_g;
        let bass_drop = self.bass_z * (current + self.bass_history);
        let jack = self.wiper_z * (current + self.cable_history);
        self.inductor_history = 2.0 * self.inductive_r * current - self.inductor_history;
        self.bass_history = if self.bass_z == 0.0 {
            0.0
        } else {
            2.0 * self.bass_g * bass_drop - self.bass_history
        };
        self.cable_history = if self.wiper_z == 0.0 {
            0.0
        } else {
            2.0 * self.cable_g * jack - self.cable_history
        };
        jack
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unloaded_panel_endpoints() {
        let mut panel = Panel::new(192_000.0);
        panel.inductive_r = 0.0;
        panel.series_r = 0.0;
        // Exactly 10k load, full volume, no cable or receiver.
        for (bass, expected) in [(0.0, 1.0 / 6.0), (1.0, 1.0)] {
            panel.set(1.0, bass);
            panel.wiper_z = 10_000.0;
            panel.loop_g = (10_000.0 + panel.bass_z).recip();
            panel.cable_g = 0.0;
            let mut y = 0.0;
            for _ in 0..10_000 {
                y = panel.tick(1.0);
            }
            assert!((y - expected).abs() < 1e-8);
        }
    }
}
