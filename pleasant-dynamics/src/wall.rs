//! Hybrid limiter: stereo-linked gain reduction in front of a transparent
//! safety clipper. Optional Even/Odd saturation runs before the limiter so
//! colour can be driven without giving up the ceiling.

pub const WALL_LOOKAHEAD_SECONDS: f64 = 0.0015;
const RELEASE_SECONDS: f64 = 0.080;
const LIVE_ATTACK_SECONDS: f64 = 0.0001;
const SAFETY_KNEE: f64 = 0.9;
const MODE_CROSSFADE_SECONDS: f64 = 0.005;

#[derive(Clone, Copy, Debug)]
pub struct WallSettings {
    pub even: f64,
    pub odd: f64,
    pub threshold: f64,
    pub on: bool,
    pub lookahead: bool,
}

impl Default for WallSettings {
    fn default() -> Self {
        Self {
            even: 0.0,
            odd: 0.0,
            threshold: 0.0,
            on: true,
            lookahead: false,
        }
    }
}

impl WallSettings {
    fn ceiling(self) -> f64 {
        10.0_f64.powf(self.threshold / 20.0).max(1.0e-6)
    }
    fn even_amount(self) -> f64 {
        (self.even * 0.01).clamp(0.0, 1.0)
    }
    fn odd_amount(self) -> f64 {
        (self.odd * 0.01).clamp(0.0, 1.0)
    }
}

pub fn wall_lookahead_samples(sample_rate: f64) -> usize {
    if sample_rate.is_finite() && sample_rate > 0.0 {
        (WALL_LOOKAHEAD_SECONDS * sample_rate).round().max(1.0) as usize
    } else {
        0
    }
}

fn sanitize(sample: f64) -> f64 {
    if sample.is_finite() {
        sample
    } else {
        0.0
    }
}

fn colour_sample(input: f64, even: f64, odd: f64, ceiling: f64) -> f64 {
    if even == 0.0 && odd == 0.0 {
        return input;
    }
    let saturated = (input / ceiling).tanh();
    let odd_path = saturated * ceiling;
    let even_path = saturated * saturated * ceiling;
    input + odd * (odd_path - input) + even * even_path
}

fn colour(input: [f64; 2], settings: WallSettings) -> [f64; 2] {
    let ceiling = settings.ceiling();
    let even = settings.even_amount();
    let odd = settings.odd_amount();
    [
        colour_sample(sanitize(input[0]), even, odd, ceiling),
        colour_sample(sanitize(input[1]), even, odd, ceiling),
    ]
}

/// Exact identity at and below the knee. Above the knee a tanh tail approaches
/// the ceiling asymptotically, so the result never exceeds the ceiling.
fn safety_sample(input: f64, ceiling: f64) -> f64 {
    let magnitude = input.abs() / ceiling;
    if magnitude <= SAFETY_KNEE {
        input
    } else {
        let tail = ((magnitude - SAFETY_KNEE) / (1.0 - SAFETY_KNEE)).tanh();
        input.signum() * ceiling * (SAFETY_KNEE + (1.0 - SAFETY_KNEE) * tail)
    }
}

fn safety(input: [f64; 2], ceiling: f64) -> [f64; 2] {
    [
        safety_sample(sanitize(input[0]), ceiling),
        safety_sample(sanitize(input[1]), ceiling),
    ]
}

fn one_pole(current: f64, target: f64, seconds: f64, sample_rate: f64) -> f64 {
    let coefficient = (-1.0 / (sample_rate * seconds.max(1.0e-6))).exp();
    coefficient * current + (1.0 - coefficient) * target
}

/// Stateless colour plus safety clip. Stateful limiting lives in [`Wall`];
/// this remains for one-shot use where no gain history exists.
pub fn tick_wall(input: [f64; 2], settings: WallSettings) -> [f64; 2] {
    if !settings.on {
        return input;
    }
    safety(colour(input, settings), settings.ceiling())
}

#[derive(Clone, Debug)]
pub struct Wall {
    delay: Vec<[f64; 2]>,
    position: usize,
    gain: f64,
    mode_mix: f64,
    initialized: bool,
}

impl Default for Wall {
    fn default() -> Self {
        Self::new(48_000.0)
    }
}

impl Wall {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            delay: vec![[0.0; 2]; wall_lookahead_samples(sample_rate).max(1)],
            position: 0,
            gain: 1.0,
            mode_mix: 0.0,
            initialized: false,
        }
    }

    pub fn reset(&mut self) {
        self.delay.fill([0.0; 2]);
        self.position = 0;
        self.gain = 1.0;
        self.initialized = false;
    }

    pub fn latency_samples(&self, settings: WallSettings) -> usize {
        if settings.lookahead {
            self.delay.len()
        } else {
            0
        }
    }

    pub fn gain(&self) -> f64 {
        self.gain
    }

    pub fn reduction_db(&self) -> f64 {
        if self.gain > 0.0 && self.gain < 1.0 {
            -20.0 * self.gain.log10()
        } else {
            0.0
        }
    }

    fn ensure_delay(&mut self, sample_rate: f64) {
        let wanted = wall_lookahead_samples(sample_rate).max(1);
        if self.delay.len() != wanted {
            self.delay = vec![[0.0; 2]; wanted];
            self.position = 0;
            self.gain = 1.0;
            self.initialized = false;
        }
    }

    pub fn process(
        &mut self,
        input: [f64; 2],
        settings: WallSettings,
        sample_rate: f64,
    ) -> [f64; 2] {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 0.0 {
            sample_rate
        } else {
            48_000.0
        };
        self.ensure_delay(sample_rate);
        let input = [sanitize(input[0]), sanitize(input[1])];

        let mode_target = if settings.lookahead { 1.0 } else { 0.0 };
        if !self.initialized {
            self.mode_mix = mode_target;
            self.initialized = true;
        } else {
            self.mode_mix = one_pole(
                self.mode_mix,
                mode_target,
                MODE_CROSSFADE_SECONDS,
                sample_rate,
            );
            if (self.mode_mix - mode_target).abs() < 1.0e-4 {
                self.mode_mix = mode_target;
            }
        }

        // The ring always runs. The detector reads the live (future) sample;
        // the delayed candidate reads the sample leaving the ring. Toggling
        // lookahead crossfades between the two candidates.
        let future = if settings.on {
            colour(input, settings)
        } else {
            input
        };
        let delayed = self.delay[self.position];
        self.delay[self.position] = future;
        self.position = (self.position + 1) % self.delay.len();

        if !settings.on {
            self.gain = one_pole(self.gain, 1.0, RELEASE_SECONDS, sample_rate);
            let live = input;
            return [
                live[0] + self.mode_mix * (delayed[0] - live[0]),
                live[1] + self.mode_mix * (delayed[1] - live[1]),
            ];
        }

        let ceiling = settings.ceiling();
        // Lookahead gain must target the loudest sample still in the ring,
        // not the newest sample alone. Otherwise gain modulation follows a
        // future phase and lands on the wrong delayed sample.
        let peak = if settings.lookahead {
            self.delay.iter().fold(0.0_f64, |peak, sample| {
                peak.max(sample[0].abs()).max(sample[1].abs())
            })
        } else {
            future[0].abs().max(future[1].abs())
        };
        let target = if peak > ceiling {
            (ceiling / peak).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let attack = if settings.lookahead {
            WALL_LOOKAHEAD_SECONDS / 3.0
        } else {
            LIVE_ATTACK_SECONDS
        };
        let seconds = if target < self.gain {
            attack
        } else {
            RELEASE_SECONDS
        };
        self.gain = one_pole(self.gain, target, seconds, sample_rate).clamp(0.0, 1.0);

        let gained_live = [future[0] * self.gain, future[1] * self.gain];
        let gained_delayed = [delayed[0] * self.gain, delayed[1] * self.gain];
        let live = safety(gained_live, ceiling);
        let delayed_out = safety(gained_delayed, ceiling);
        [
            sanitize(live[0] + self.mode_mix * (delayed_out[0] - live[0])),
            sanitize(live[1] + self.mode_mix * (delayed_out[1] - live[1])),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f64 = 48_000.0;

    fn settings(threshold: f64) -> WallSettings {
        WallSettings {
            threshold,
            ..WallSettings::default()
        }
    }

    #[test]
    fn stateless_path_is_identity_below_the_safety_knee() {
        let ceiling = 10.0_f64.powf(-12.0 / 20.0);
        for input in [[0.5 * ceiling, -0.25 * ceiling], [0.0, 0.0]] {
            assert_eq!(tick_wall(input, settings(-12.0)), input);
        }
    }

    #[test]
    fn bypass_is_identity() {
        let input = [0.8, -0.4];
        assert_eq!(
            tick_wall(
                input,
                WallSettings {
                    even: 100.0,
                    odd: 100.0,
                    on: false,
                    ..WallSettings::default()
                }
            ),
            input
        );
        let mut wall = Wall::new(SR);
        assert_eq!(
            wall.process(
                input,
                WallSettings {
                    on: false,
                    lookahead: true,
                    ..WallSettings::default()
                },
                SR
            ),
            [0.0, 0.0],
            "lookahead bypass still travels the delay so enabling WALL stays time-aligned"
        );
    }

    #[test]
    fn ceiling_holds_for_colour_and_large_inputs_in_both_modes() {
        for lookahead in [false, true] {
            for threshold in [0.0, -12.0, -48.0] {
                let ceiling = 10.0_f64.powf(threshold / 20.0);
                let mut wall = Wall::new(SR);
                let active = WallSettings {
                    even: 100.0,
                    odd: 100.0,
                    threshold,
                    lookahead,
                    ..WallSettings::default()
                };
                assert_eq!(wall.process([0.0; 2], active, SR), [0.0; 2]);
                for step in -100..=100 {
                    let input = step as f64 * ceiling;
                    for output in wall.process([input, -input], active, SR) {
                        assert!(output.is_finite() && output.abs() <= ceiling + 1.0e-12);
                    }
                    for output in tick_wall([input, -input], active) {
                        assert!(output.is_finite() && output.abs() <= ceiling + 1.0e-12);
                    }
                }
            }
        }
    }

    #[test]
    fn limiter_is_transparent_below_ceiling_and_recovers() {
        let mut wall = Wall::new(SR);
        let active = settings(-12.0);
        let ceiling = 10.0_f64.powf(-12.0 / 20.0);
        let quiet = [0.4 * ceiling, -0.2 * ceiling];
        for _ in 0..1_000 {
            assert_eq!(wall.process(quiet, active, SR), quiet);
        }
        assert_eq!(wall.reduction_db(), 0.0);
        for _ in 0..48_000 {
            wall.process([4.0 * ceiling, 4.0 * ceiling], active, SR);
        }
        assert!(wall.reduction_db() > 11.0);
        let mut output = [0.0; 2];
        for _ in 0..48_000 {
            output = wall.process(quiet, active, SR);
        }
        for channel in 0..2 {
            assert!((output[channel] - quiet[channel]).abs() < quiet[channel].abs() * 0.01);
        }
    }

    #[test]
    fn stereo_link_reduces_the_quiet_channel_with_the_loud_one() {
        let mut wall = Wall::new(SR);
        let mut output = [0.0; 2];
        for _ in 0..48_000 {
            output = wall.process([2.0, 0.5], settings(0.0), SR);
        }
        assert!(
            (output[0] - 1.0).abs() < 0.03,
            "left sits at the safety knee"
        );
        assert!(
            (output[1] - 0.25).abs() < 0.01,
            "right channel shares left gain"
        );
    }

    #[test]
    fn lookahead_delays_by_the_reported_latency_and_prereduces_gain() {
        let active = WallSettings {
            lookahead: true,
            ..settings(0.0)
        };
        let mut wall = Wall::new(SR);
        let latency = wall.latency_samples(active);
        assert_eq!(latency, wall_lookahead_samples(SR));
        for _ in 0..latency {
            assert_eq!(wall.process([2.0, 2.0], active, SR), [0.0, 0.0]);
        }
        assert!(
            wall.reduction_db() > 5.0,
            "gain must already fall before the delayed peak emerges, got {} dB",
            wall.reduction_db()
        );
        let emerged = wall.process([2.0, 2.0], active, SR);
        assert!(emerged[0] > 0.9 && emerged[0] <= 1.0);
    }

    #[test]
    fn lookahead_distorts_a_sudden_burst_less_than_live_attack() {
        fn burst_error(lookahead: bool) -> f64 {
            let active = WallSettings {
                lookahead,
                ..settings(0.0)
            };
            let mut wall = Wall::new(SR);
            let latency = wall.latency_samples(active);
            let mut error = 0.0;
            let mut energy = 0.0;
            for index in 0..48_000 {
                let input = if index < 1_000 {
                    0.0
                } else {
                    2.0 * (std::f64::consts::TAU * 1_000.0 * (index - 1_000) as f64 / SR).sin()
                };
                let output = wall.process([input, input], active, SR)[0];
                // Output at this index carries the source from `latency`
                // samples earlier. The ideal brickwall gain for a 2x sine
                // is 0.5; deviation from that gained sine is distortion.
                if index >= 1_000 + latency && index < 3_000 + latency {
                    let source = 2.0
                        * (std::f64::consts::TAU * 1_000.0 * (index - latency - 1_000) as f64 / SR)
                            .sin();
                    let reference = source * 0.5;
                    error += (output - reference).powi(2);
                    energy += reference.powi(2);
                }
            }
            error / energy.max(1.0e-12)
        }
        let live = burst_error(false);
        let ahead = burst_error(true);
        assert!(
            ahead < live * 0.5,
            "lookahead error {ahead} must be well below live error {live}"
        );
    }

    #[test]
    fn controls_add_even_and_odd_harmonics_to_the_stateless_path() {
        fn harmonic(active: WallSettings, number: usize) -> f64 {
            let mut real = 0.0;
            let mut imag = 0.0;
            for sample in 0..4096 {
                let phase = std::f64::consts::TAU * sample as f64 / 4096.0;
                let output = tick_wall([0.8 * phase.sin(); 2], active)[0];
                real += output * (number as f64 * phase).cos();
                imag += output * (number as f64 * phase).sin();
            }
            real.hypot(imag) / 2048.0
        }
        let base = WallSettings::default();
        let even = WallSettings {
            even: 100.0,
            ..base
        };
        let odd = WallSettings { odd: 100.0, ..base };
        assert!(harmonic(base, 2) < 1.0e-12);
        assert!(harmonic(even, 2) > 0.01);
        assert!(harmonic(odd, 2) < 1.0e-12);
        assert!(harmonic(odd, 3) / harmonic(odd, 1) > harmonic(base, 3) / harmonic(base, 1));
    }
}
