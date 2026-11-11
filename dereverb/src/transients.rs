//! Adaptive input envelopes shared by all restoration engines.
use crate::dsp::DelayLine;

const HISTORY: usize = 96; // 192 ms
const RECENT: usize = 6; // 12 ms
const MAX_FRICATIVE: usize = 80; // 160 ms
const MAX_DRY_BLEND: f32 = 0.88;

pub struct Guard {
    // Broad and high-band power, in 2 ms hops. Decisions use earlier hops only.
    history: [[f64; 2]; HISTORY],
    cursor: usize,
    trailing: [f64; 2],
    recent: [f64; 2],
    power: [f64; 2],
    samples: usize,
    hop: usize,
    hp_input: [f64; 2],
    hp_output: [f64; 2],
    pole: f64,
    onset: f64,
    onset_hold: usize,
    fricative_left: usize,
    fricative_peak: f64,
    fricative_armed: bool,
    target: f32,
    aligned: DelayLine,
    applied: f32,
    attack: f32,
    release: f32,
}

impl Guard {
    pub fn new(rate: f64, latency: usize) -> Self {
        let hop = (0.002 * rate).round().max(1.0) as usize;
        Self {
            history: [[0.0; 2]; HISTORY],
            cursor: 0,
            trailing: [0.0; 2],
            recent: [0.0; 2],
            power: [0.0; 2],
            samples: 0,
            hop,
            hp_input: [0.0; 2],
            hp_output: [0.0; 2],
            pole: (-std::f64::consts::TAU * 1900.0 / rate).exp(),
            onset: 0.0,
            onset_hold: 0,
            fricative_left: 0,
            fricative_peak: 0.0,
            fricative_armed: true,
            target: 0.0,
            // Spend 4 ms of existing plugin delay on the 2 ms detector and
            // its attack, so the guard is open by the time the consonant plays.
            aligned: DelayLine::new(latency.saturating_sub(2 * hop)),
            applied: 0.0,
            attack: (1.0 - (-1.0 / (0.00065 * rate)).exp()) as f32,
            release: (1.0 - (-1.0 / (0.006 * rate)).exp()) as f32,
        }
    }

    pub fn reset(&mut self) {
        self.history.fill([0.0; 2]);
        self.cursor = 0;
        self.trailing = [0.0; 2];
        self.recent = [0.0; 2];
        self.power = [0.0; 2];
        self.samples = 0;
        self.hp_input = [0.0; 2];
        self.hp_output = [0.0; 2];
        self.onset = 0.0;
        self.onset_hold = 0;
        self.fricative_left = 0;
        self.fricative_peak = 0.0;
        self.fricative_armed = true;
        self.target = 0.0;
        self.aligned.reset();
        self.applied = 0.0;
    }

    pub fn amount(&self) -> f32 {
        self.applied
    }

    pub fn tick(&mut self, input: f32, amount: f32) -> f32 {
        let input = input as f64;
        let mut high = input;
        let gain = (1.0 + self.pole) * 0.5;
        for stage in 0..2 {
            let output = gain * (high - self.hp_input[stage]) + self.pole * self.hp_output[stage];
            self.hp_input[stage] = high;
            self.hp_output[stage] = output;
            high = output;
        }
        self.power[0] += input * input;
        self.power[1] += high * high;
        self.samples += 1;
        if self.samples == self.hop {
            self.update();
        }

        let target = self.aligned.tick(self.target) * amount;
        let coefficient = if target > self.applied {
            self.attack
        } else {
            self.release
        };
        self.applied += coefficient * (target - self.applied);
        if self.applied < 1e-6 {
            self.applied = 0.0;
        }
        self.applied
    }

    fn update(&mut self) {
        let [broad, high] = self.power;
        let [old_broad, old_high] = self.trailing.map(|sum| sum.max(0.0) / HISTORY as f64);
        let [recent_broad, recent_high] = self.recent.map(|sum| sum.max(0.0) / RECENT as f64);

        // Compare to both the recent envelope and the longer local context.
        // The longer reference prevents tiny fluctuations in a quiet tail
        // from looking like a new onset. All thresholds are level ratios.
        let broad_rise = ramp(ratio(broad, recent_broad + 0.035 * old_broad), 2.4, 8.0);
        let high_rise = ramp(
            ratio(high, recent_high + 0.05 * old_high + 0.002 * old_broad),
            2.0,
            6.0,
        );
        let brightness = ramp(ratio(high, broad + 0.015 * old_broad), 0.07, 0.27);
        let attack = broad_rise.max(high_rise * brightness);
        if attack > self.onset {
            self.onset = attack;
            self.onset_hold = 3;
        } else if self.onset_hold > 0 {
            self.onset_hold -= 1;
        } else {
            self.onset *= 0.72;
        }

        // An abrupt bright event can remain a consonant after its onset.
        // Re-arm only after its high-band envelope drops or loses brightness.
        // The time limit prevents a step into steady hiss from holding dry open.
        let fell = high <= 0.30 * self.fricative_peak || brightness < 0.05;
        if fell {
            self.fricative_armed = true;
            self.fricative_left = 0;
            self.fricative_peak = 0.0;
        }
        // A new jump above both references can also rise out of a hiss
        // that already exhausted its protection window.
        let new_burst = self.fricative_left == 0 && high_rise > 0.55 && high > 3.0 * old_high;
        if (self.fricative_armed || new_burst) && brightness > 0.08 && high_rise > 0.10 {
            self.fricative_armed = false;
            self.fricative_left = MAX_FRICATIVE;
            self.fricative_peak = high;
        }

        let mut fricative = 0.0;
        if self.fricative_left > 0 {
            self.fricative_peak = high.max(0.97 * self.fricative_peak);
            // Softly return to the engine over the last 20 ms.
            let remaining = (self.fricative_left as f64 / 10.0).min(1.0);
            let sustain = ramp(ratio(high, self.fricative_peak), 0.30, 0.65);
            fricative = brightness * sustain * remaining;
            self.fricative_left -= 1;
        }
        self.target = MAX_DRY_BLEND * self.onset.max(fricative) as f32;

        let short_index = (self.cursor + HISTORY - RECENT) % HISTORY;
        for band in 0..2 {
            self.trailing[band] += self.power[band] - self.history[self.cursor][band];
            self.recent[band] += self.power[band] - self.history[short_index][band];
        }
        self.history[self.cursor] = self.power;
        self.cursor = (self.cursor + 1) % HISTORY;
        self.power = [0.0; 2];
        self.samples = 0;
    }
}

fn ratio(value: f64, reference: f64) -> f64 {
    value / reference.max(f64::MIN_POSITIVE)
}

fn ramp(value: f64, start: f64, end: f64) -> f64 {
    ((value - start) / (end - start)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_guard_anticipates_attacks_then_releases_steady_hiss() {
        for rate in [8000.0, 44100.0, 48000.0, 192000.0, 384000.0] {
            let latency = (0.060 * rate) as usize;
            let onset = (0.220 * rate) as usize;
            let mut detector = Guard::new(rate, latency);
            let mut seed = 21_u32;
            let mut trace = Vec::new();
            for i in 0..(0.600 * rate) as usize {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = (seed as i32) as f32 / i32::MAX as f32;
                let input = if i >= onset {
                    0.15 * noise
                } else {
                    0.001 * noise
                };
                trace.push(detector.tick(input, 1.0));
            }
            let measure = |start: f64, end: f64| {
                let slice = &trace[onset + latency + (start * rate) as usize
                    ..onset + latency + (end * rate) as usize];
                slice.iter().sum::<f32>() / slice.len() as f32
            };
            let attack = measure(0.0, 0.002);
            assert!(attack > 0.55, "{rate} Hz: onset protection {attack}");
            let fricative = measure(0.04, 0.09);
            assert!(
                fricative > 0.55,
                "{rate} Hz: fricative protection {fricative}"
            );
            let steady = measure(0.24, 0.30);
            assert!(
                steady < 0.04,
                "{rate} Hz: stationary hiss protection {steady}"
            );
            detector.reset();
            assert!((0..latency).all(|_| detector.tick(0.0, 1.0) == 0.0));
        }
    }

    #[test]
    fn transient_guard_catches_soft_breath_after_a_louder_vowel() {
        let rate = 48000.0;
        let latency = 2880;
        let onset = 11_520;
        let mut guard = Guard::new(rate, latency);
        let mut seed = 13u32;
        let mut breath = 0.0;
        let mut max_attack = 0.0_f32;
        let mut protected = 0.0;
        let mut count = 0;
        for i in 0..onset + latency + 2880 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let noise = (seed as i32) as f32 / i32::MAX as f32;
            breath += 0.27 * (noise - breath);
            let input = if i < onset {
                0.18 * (std::f32::consts::TAU * 163.0 * i as f32 / rate as f32).sin()
            } else {
                0.08 * breath
            };
            let amount = guard.tick(input, 1.0);
            if (onset + latency..onset + latency + 240).contains(&i) {
                max_attack = max_attack.max(amount);
            }
            if (onset + latency + 480..onset + latency + 1920).contains(&i) {
                protected += amount;
                count += 1;
            }
        }
        assert!(
            max_attack > 0.4,
            "soft breath attack protection: {max_attack}"
        );
        assert!(
            protected / count as f32 > 0.35,
            "soft breath protection: {}",
            protected / count as f32
        );
    }
}
