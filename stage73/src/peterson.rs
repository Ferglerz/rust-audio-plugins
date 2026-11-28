//! Reduced, centre-EQ small-signal model of Rhodes factory figure 11-8.
use crate::circuit::HARP_GROUP_FACTOR;
use crate::circuit_network::{Filter, Network};

// Nominal 5 uF astable, 22k base feeds and 50k speed rheostat.
// Endpoint periods from research/peterson_lfo.cir.
pub const MIN_HZ: f64 = 1.92;
pub const MAX_HZ: f64 = 7.23;

pub(crate) struct Peterson {
    input: Filter<4>,
    phase: f64,
    rate: f64,
    lamp: [f64; 2],
    cell: [f64; 2],
    thermal: [f64; 2],
    photo: [f64; 2],
}

impl Peterson {
    pub(crate) fn new(rate: f64, oversample: usize) -> Self {
        // Input pair at its linear operating point. The bypassed supply is AC
        // ground. The two halves of the active bass/treble network balance at
        // centre; represent its buffered output at nominal midband gain.
        let mut n = Network::<7, 4>::new(rate * oversample as f64, 1, HARP_GROUP_FACTOR, 0.);
        n.capacitor(1, 2, 0.22e-6);
        n.resistor(2, 0, 33e3);
        n.resistor(2, 0, 390e3);
        n.resistor(2, 3, 4.7e3); // estimated small-signal base resistance
        n.resistor(3, 0, 1.5e3);
        n.resistor(4, 0, 12e3);
        n.transconductance(4, 3, 2, 0.05);
        n.capacitor(2, 4, 330e-12);
        // 0.22 uF coupling into the 100k/100k follower bias divider.
        // The buffer isolates this load from the tone network.
        n.transconductance(0, 5, 4, 1.);
        n.capacitor(5, 6, 0.22e-6);
        n.resistor(6, 0, 50e3);
        let mut input = Filter::empty();
        n.compile(6, &mut input);
        let alpha = |ms: f64| -(-1000. / (rate * ms)).exp_m1();
        Self {
            input,
            phase: 0.,
            rate,
            lamp: [0.5; 2],
            cell: [0.25; 2],
            thermal: [alpha(14.), alpha(24.)],
            photo: [alpha(6.), alpha(16.)],
        }
    }

    pub(crate) fn clear_audio(&mut self) {
        self.input.clear();
    }

    pub(crate) fn clear(&mut self) {
        self.input.clear();
        self.phase = 0.;
        self.lamp = [0.5; 2];
        self.cell = [0.25; 2];
    }

    pub(crate) fn audio(&mut self, harp: f64) -> f64 {
        // The two inverting amplifier stages restore polarity. Fixed digital
        // alignment allows the full-intensity bright channel to retain headroom.
        self.input.tick(harp) * -0.105
    }

    pub(crate) fn gains(&mut self, hz: f64, intensity: f64) -> [f64; 2] {
        self.phase = (self.phase + hz / self.rate).fract();
        let mut branches = [0.; 2];
        for (ch, branch) in branches.iter_mut().enumerate() {
            let drive = f64::from((self.phase < 0.5) == (ch == 0));
            let lamp = &mut self.lamp[ch];
            *lamp += self.thermal[usize::from(drive < *lamp)] * (drive - *lamp);
            // Light output rises steeply with filament temperature. The incident
            // illuminance and cell time constants are provisional.
            let light = lamp.powi(3);
            let cell = &mut self.cell[ch];
            *cell += self.photo[usize::from(light < *cell)] * (light - *cell);
            *branch = 1. / 2e6 + *cell * (1. / 2e3 - 1. / 2e6);
        }
        // Two series ORP61s feed 100k bias loads. Each output also receives
        // dry signal via a 47k arm from the common 10k / 50k intensity node.
        let arm = 1. / 47e3;
        let feed = 1. / 10e3;
        let load = 1. / 100e3;
        let z = branches.map(|g| 1. / (g + arm + load));
        let pot = 50e3 * (1. - intensity).powi(2);
        let common = pot * (feed + arm * (branches[0] * z[0] + branches[1] * z[1]))
            / (1. + pot * (feed + 2. * arm - arm * arm * (z[0] + z[1])));
        std::array::from_fn(|ch| {
            let optical = (branches[ch] + common * arm) * z[ch];
            1. + intensity * (optical - 1.)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::TAU;

    #[test]
    fn steady_tone_measures_opposed_envelopes_and_mono_overlap() {
        let rate = 48_000.;
        for hz in [MIN_HZ, 5., MAX_HZ] {
            let mut preamp = Peterson::new(rate, 1);
            let mut envelope = Vec::new();
            let mut power = [0.; 3];
            for i in 0..144_000 {
                let tone = preamp.audio((TAU * 1000. * i as f64 / rate).sin());
                let [l, r] = preamp.gains(hz, 1.).map(|g| g * tone);
                for (p, x) in power.iter_mut().zip([l, r, (l + r) * 0.5]) {
                    *p += x * x;
                }
                if (i + 1) % 48 == 0 {
                    if i >= 48_000 {
                        envelope.push(power.map(|p| (p / 48.).sqrt()));
                    }
                    power = [0.; 3];
                }
            }
            let extrema = |ch: usize| {
                envelope
                    .iter()
                    .map(|v| v[ch])
                    .fold((f64::INFINITY, 0_f64), |(lo, hi), x| (lo.min(x), hi.max(x)))
            };
            for ch in 0..2 {
                let (lo, hi) = extrema(ch);
                let db = 20. * (hi / lo).log10();
                let threshold = |fraction| lo + (hi - lo) * fraction;
                let crossings = |fraction: f64, rising| {
                    envelope
                        .windows(2)
                        .enumerate()
                        .filter_map(|(i, w)| {
                            let y = threshold(fraction);
                            let cross = if rising {
                                w[0][ch] < y && w[1][ch] >= y
                            } else {
                                w[0][ch] >= y && w[1][ch] < y
                            };
                            cross.then_some(i)
                        })
                        .collect::<Vec<_>>()
                };
                let duration = |rising| {
                    let (a, b) = if rising { (0.1, 0.9) } else { (0.9, 0.1) };
                    let start = crossings(a, rising)[0];
                    crossings(b, rising)
                        .into_iter()
                        .find(|&i| i > start)
                        .unwrap()
                        - start
                };
                let peaks = crossings(0.5, true);
                let period = (peaks.last().unwrap() - peaks[0]) as f64 / (peaks.len() - 1) as f64;
                let rise = duration(true);
                let fall = duration(false);
                assert!((1000. / period - hz).abs() < 0.04);
                assert!(
                    db > 4. && db < 40. && (2..120).contains(&rise) && (2..120).contains(&fall)
                );
                eprintln!("{hz} Hz channel {ch}: rise {rise} ms, fall {fall} ms, depth {db:.2} dB");
            }
            let (lo, hi) = extrema(2);
            let ripple = 20. * (hi / lo).log10();
            assert!(lo > 0.1 && ripple < 6.);
            // Envelopes are measured every millisecond. Compare at the
            // fractional half-period, including speeds off the millisecond grid.
            let delay = 500. / hz;
            let samples = delay.floor() as usize;
            for (left, opposite) in envelope.iter().zip(envelope[samples..].windows(2)) {
                let right = opposite[0][1] + delay.fract() * (opposite[1][1] - opposite[0][1]);
                assert!(
                    (left[0] - right).abs() < 0.006,
                    "{hz} Hz half-cycle error {}",
                    left[0] - right
                );
            }
            eprintln!("{hz} Hz mono ripple {ripple:.2} dB");
            assert_eq!(preamp.gains(hz, 0.), [1.; 2]);
        }
    }
}
