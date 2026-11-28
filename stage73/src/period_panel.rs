//! Small-signal period circuits. Node numbers follow research/{early,jordan}.cir.
use crate::circuit::{DEFAULT_CABLE_PF, HARP_GROUP_FACTOR, MAX_CABLE_PF, MIN_CABLE_PF};
use crate::circuit_network::{Filter, Network};
use crate::jordan_lfo::CYCLES;
pub const MIN_TREM_HZ: f64 = CYCLES[0].0;
pub const MAX_TREM_HZ: f64 = CYCLES[2].0;

pub const ORIGINAL_GROUP_FACTOR: f64 = 1.0 / 7.0 + 11.0 / 6.0;

pub struct EarlyPanel {
    circuit: Filter<3>,
    rate: f64,
    setting: (f64, f64, f64),
}
impl EarlyPanel {
    pub fn new(rate: f64) -> Self {
        let mut p = Self {
            circuit: Filter::empty(),
            rate,
            setting: (-1., -1., DEFAULT_CABLE_PF),
        };
        p.set(1., 1.);
        p
    }
    pub fn clear(&mut self) {
        self.circuit.clear();
    }
    pub fn set(&mut self, volume: f64, tone: f64) {
        self.set_loaded(volume, tone, DEFAULT_CABLE_PF);
    }
    pub fn set_loaded(&mut self, volume: f64, tone: f64, cable_pf: f64) {
        let cable_pf = cable_pf.clamp(MIN_CABLE_PF, MAX_CABLE_PF);
        if self.setting == (volume, tone, cable_pf) {
            return;
        }
        self.setting = (volume, tone, cable_pf);
        let mut net = Network::<4, 3>::new(self.rate, 1, HARP_GROUP_FACTOR, 0.);
        let volume_bottom = 10_000. * volume.powi(3);
        let tone_top = 10_000. * (1.0 - (1.0 - tone).powi(3));
        net.resistor(1, 2, tone_top);
        net.resistor(2, 0, 10_000. - tone_top);
        net.capacitor(2, 0, 1e-6);
        net.resistor(1, 3, 10_000. - volume_bottom);
        net.resistor(3, 0, volume_bottom);
        net.resistor(3, 0, 1e6);
        net.capacitor(3, 0, cable_pf * 1e-12);
        net.compile(3, &mut self.circuit);
    }
    pub fn tick(&mut self, x: f64) -> f64 {
        let y = self.circuit.tick(x);
        if self.setting.0 == 0. {
            0.
        } else {
            y
        }
    }
}

pub struct Jordan {
    circuit: Filter<11>,
    rate: f64,
    setting: (f64, f64, f64),
    phase: f64,
    step: f64,
    depth: f64,
    cycle: usize,
    blend: f64,
}
impl Jordan {
    pub fn new(rate: f64) -> Self {
        let mut p = Self {
            circuit: Filter::empty(),
            rate,
            setting: (-1., -1., -1.),
            phase: 0.,
            step: 0.,
            depth: 0.,
            cycle: 0,
            blend: 0.,
        };
        p.set(1., 0.5, 0.5);
        p.tremolo(5., 0.);
        p
    }
    pub fn clear(&mut self) {
        self.circuit.clear();
        self.phase = 0.;
    }
    pub fn tremolo(&mut self, hz: f64, depth: f64) {
        let hz = hz.clamp(MIN_TREM_HZ, MAX_TREM_HZ);
        self.cycle = usize::from(hz > CYCLES[1].0);
        let lo = CYCLES[self.cycle].0;
        self.blend = (hz - lo) / (CYCLES[self.cycle + 1].0 - lo);
        self.step = hz / self.rate;
        self.depth = depth;
    }
    pub fn set(&mut self, volume: f64, bass: f64, treble: f64) {
        if self.setting == (volume, bass, treble) {
            return;
        }
        self.setting = (volume, bass, treble);
        // 33k before the shunt FET and 10k after it. Q7 is open in the dry
        // small-signal model. Its periodic attenuation is applied at the input.
        let mut n = Network::<19, 11>::new(self.rate, 1, ORIGINAL_GROUP_FACTOR, 43_000.);
        n.capacitor(1, 2, 0.15e-6);
        for (base, collector, gm, rin) in [(2, 3, 0.011390, 3.547e6), (9, 10, 0.011318, 3.570e6)] {
            n.resistor(base, 0, 220e3);
            n.resistor(collector, 0, 12e3);
            n.resistor(base, collector, 1e6);
            // Q2/Q3 and Q4/Q5 are collector-feedback Darlington pairs.
            n.transconductance(collector, 0, base, gm);
            n.resistor(base, 0, rin);
        }
        n.resistor(3, 4, 6.8e3);
        n.resistor(5, 10, 6.8e3);
        n.resistor(4, 6, 100e3 * (1. - bass));
        n.resistor(6, 5, 100e3 * bass);
        n.capacitor(4, 6, 0.1e-6);
        n.capacitor(6, 5, 0.1e-6);
        n.resistor(6, 7, 22e3);
        // 100k treble wiper coupled through 5nF to the summing junction.
        n.resistor(3, 8, 6.8e3 + 100e3 * (1. - treble));
        n.resistor(8, 10, 6.8e3 + 100e3 * treble);
        n.capacitor(8, 7, 5e-9);
        n.capacitor(7, 9, 47e-9);
        n.resistor(10, 11, 6.8e3);
        n.resistor(11, 12, 68e3);
        n.capacitor(11, 13, 5e-9);
        n.capacitor(12, 0, 470e-12);
        n.transconductance(0, 13, 12, 0.030657);
        n.resistor(12, 13, 6523.75);
        n.resistor(13, 0, 6.8e3);
        let top = 100e3 * (1. - volume.powi(3));
        n.resistor(13, 14, top);
        n.resistor(14, 15, 100e3 - top);
        // Loudness bypass across the pot, and the AC return at its foot.
        n.resistor(13, 16, 100e3);
        n.capacitor(16, 15, 0.01e-6);
        n.resistor(15, 17, 100.);
        n.capacitor(17, 0, 1e-6);
        // Cabinet input coupling and the summing resistor feeding Q8.
        n.capacitor(14, 18, 1e-6);
        n.resistor(18, 0, 10e3);
        n.compile(14, &mut self.circuit);
    }
    pub fn tick(&mut self, x: f64) -> f64 {
        let modulated = if self.depth == 0. {
            x
        } else {
            let phase = self.phase * 64.;
            let i = phase as usize;
            let value = |table: &[f64; 64]| {
                table[i] + (phase - i as f64) * (table[(i + 1) % 64] - table[i])
            };
            let a = value(&CYCLES[self.cycle].1);
            let lfo = a + self.blend * (value(&CYCLES[self.cycle + 1].1) - a);
            x * (1. - self.depth * lfo)
        };
        self.phase += self.step;
        if self.phase >= 1. {
            self.phase -= 1.;
        }
        self.circuit.tick(modulated)
    }
}
