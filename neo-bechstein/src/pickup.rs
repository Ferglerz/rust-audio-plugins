//! Eighteen horseshoe heads in series, equalized by six shunt capacitors.
use crate::action::{strings_at, HIGH, LOW};
use crate::amp::{INPUT_OHMS, INPUT_SHUNT_F};
use pleasant_dsp::circuit::{Filter, SizedCircuit};

pub const HEADS: usize = 18;
// Sixteen groups of five keys, followed by two groups of four. Extended
// strings feed the end head instead of adding turns to the Vienna network.
pub fn head(note: u8) -> usize {
    let offset = usize::from(note.clamp(LOW, HIGH) - LOW);
    if offset < 80 {
        offset / 5
    } else {
        16 + (offset - 80) / 4
    }
}

// The six shunt capacitors are documented; their values and grouping here
// are estimates. Coils within one unshunted series section share its current.
const COUNTS: [usize; 6] = [1, 2, 3, 4, 4, 4];
const CAPACITORS: [f64; 6] = [47e-9, 22e-9, 10e-9, 4.7e-9, 2.2e-9, 1.0e-9];
// Effective per-head flux linkage in Wb-turns. The original coil data are
// missing. A velocity-100 C chord gives 100 mV RMS at E1 from 20 to 120 ms,
// the amplifier's documented nominal sensitivity. See examples/regulation.rs.
const LINKAGE: f64 = 0.01240;

pub struct Pickups {
    circuit: Filter<14, 6, 1>,
    prev: [f64; HEADS],
    wires: [f64; HEADS],
    linkage: f64,
    rate: f64,
    offset_mm: f64,
}

impl Pickups {
    pub fn new(rate: f64) -> Self {
        Self::with_linkage(rate, LINKAGE)
    }

    /// Set the pickup sensitivity in Wb-turns for controlled mechanical measurements.
    pub fn with_linkage(rate: f64, linkage: f64) -> Self {
        assert!(linkage.is_finite() && linkage > 0.);
        let mut c = SizedCircuit::<14, 14, 20>::new(13, rate);
        for (i, count) in COUNTS.into_iter().enumerate() {
            let prev = 2 * i;
            let coil = prev + 1;
            let next = prev + 2;
            let copper = 170. * count as f64;
            c.r(prev, coil, copper);
            c.input_port(i, coil, 1. / copper);
            c.input_port(i, prev, -1. / copper);
            c.l(coil, next, 0.048 * count as f64);
            c.c(prev, next, CAPACITORS[i]);
        }
        c.c(12, 13, 22e-9);
        c.r(13, 0, INPUT_OHMS);
        c.c(12, 0, INPUT_SHUNT_F);
        let mut circuit = Filter::default();
        c.compile_ports([12], &mut circuit);
        let mut wires = [0.; HEADS];
        for note in LOW..=HIGH {
            wires[head(note)] += strings_at(note) as f64;
        }
        Self {
            circuit,
            prev: [0.; HEADS],
            wires,
            linkage,
            rate,
            offset_mm: 0.,
        }
    }

    pub fn reset(&mut self) {
        self.circuit.reset();
        self.prev.fill(0.);
    }

    pub fn set_distance_mm(&mut self, offset_mm: f64) -> bool {
        let offset = offset_mm.clamp(-1., 1.);
        if !offset_mm.is_finite() || offset == self.offset_mm {
            return false;
        }
        self.offset_mm = offset;
        true
    }

    pub fn rebase(&mut self, motion: [f64; HEADS]) {
        for (i, m) in motion.into_iter().enumerate() {
            self.prev[i] = self.flux(i, m);
        }
    }

    fn flux(&self, group: usize, motion: f64) -> f64 {
        let delta = motion / self.wires[group];
        // Difference of linked flux from rest for 20% shared reluctance.
        self.linkage * 0.8 * delta / (1. + 0.2 * delta)
    }

    /// Contribution of a wire to normalized head permeance. The complete
    /// pickup shares a finite return reluctance across all vibrating wires.
    pub fn motion(&self, summed_displacement: f64, strings: usize) -> f64 {
        let gap = (2.4 + self.offset_mm) * 0.001;
        let per_string = summed_displacement / strings as f64;
        strings as f64 * per_string / (gap - per_string).max(0.25 * gap)
    }

    pub fn tick(&mut self, motion: [f64; HEADS]) -> ([f64; HEADS], f64) {
        let mut emfs = [0.; HEADS];
        let mut sections = [0.; 6];
        let mut group = 0;
        for (section, count) in COUNTS.into_iter().enumerate() {
            for _ in 0..count {
                let flux = self.flux(group, motion[group]);
                emfs[group] = -(flux - self.prev[group]) * self.rate;
                self.prev[group] = flux;
                sections[section] += emfs[group];
                group += 1;
            }
        }
        (emfs, self.circuit.tick_ports(sections)[0])
    }
}
