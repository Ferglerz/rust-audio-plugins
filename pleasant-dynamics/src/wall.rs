//! Hybrid limiter: stereo-linked gain reduction in front of a transparent
//! safety clipper. Optional Even/Odd saturation runs before the limiter so
//! colour can be driven without giving up the ceiling. Both waveshapers
//! run through first-order antiderivative anti-aliasing in [`Wall`], so
//! generated harmonics stay in band instead of folding below Nyquist.

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

fn log_cosh(value: f64) -> f64 {
    let magnitude = value.abs();
    magnitude + (-2.0 * magnitude).exp().ln_1p() - std::f64::consts::LN_2
}

/// Antiderivative of `colour_sample`. First-order antiderivative
/// anti-aliasing (ADAA) divides the difference of this function between
/// consecutive samples by the sample difference, which replaces the
/// instantaneous waveshape with its exact average over the interval and
/// strongly suppresses harmonics that would fold back below Nyquist.
fn colour_antiderivative(input: f64, even: f64, odd: f64, ceiling: f64) -> f64 {
    let normalized = input / ceiling;
    (1.0 - odd) * input * input * 0.5
        + odd * ceiling * ceiling * log_cosh(normalized)
        + even * ceiling * ceiling * (normalized - normalized.tanh())
}

fn colour_sample_adaa(input: f64, previous: f64, even: f64, odd: f64, ceiling: f64) -> f64 {
    // No colour is the identity function. ADAA of the identity would be a
    // two-sample average, so the transparent path must stay exact.
    if even == 0.0 && odd == 0.0 {
        return input;
    }
    let difference = input - previous;
    if (difference / ceiling).abs() < 1.0e-4 {
        colour_sample((input + previous) * 0.5, even, odd, ceiling)
    } else {
        (colour_antiderivative(input, even, odd, ceiling)
            - colour_antiderivative(previous, even, odd, ceiling))
            / difference
    }
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

/// Antiderivative of `safety_sample` (an even function, like the safety
/// curve's integral from silence), for ADAA on the gained signal.
fn safety_antiderivative(input: f64, ceiling: f64) -> f64 {
    let magnitude = input.abs();
    let knee = SAFETY_KNEE * ceiling;
    if magnitude <= knee {
        input * input * 0.5
    } else {
        SAFETY_KNEE * ceiling * magnitude - 0.5 * knee * knee
            + (1.0 - SAFETY_KNEE)
                * (1.0 - SAFETY_KNEE)
                * ceiling
                * ceiling
                * log_cosh((magnitude - knee) / ((1.0 - SAFETY_KNEE) * ceiling))
    }
}

fn safety_sample_adaa(input: f64, previous: f64, ceiling: f64) -> f64 {
    let knee = SAFETY_KNEE * ceiling;
    // Below the knee the safety curve is the identity, and ADAA of the
    // identity would average two samples. Stay exact there.
    if input.abs() <= knee && previous.abs() <= knee {
        return input;
    }
    let difference = input - previous;
    if (difference / ceiling).abs() < 1.0e-4 {
        safety_sample((input + previous) * 0.5, ceiling)
    } else {
        (safety_antiderivative(input, ceiling) - safety_antiderivative(previous, ceiling))
            / difference
    }
}

fn one_pole_coeff(seconds: f64, sample_rate: f64) -> f64 {
    (-1.0 / (sample_rate * seconds.max(1.0e-6))).exp()
}

fn one_pole(current: f64, target: f64, coefficient: f64) -> f64 {
    coefficient * current + (1.0 - coefficient) * target
}

#[derive(Clone, Copy, Debug)]
struct Coefficients {
    sample_rate: f64,
    mode: f64,
    release: f64,
    live_attack: f64,
    lookahead_attack: f64,
}

impl Coefficients {
    fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            mode: one_pole_coeff(MODE_CROSSFADE_SECONDS, sample_rate),
            release: one_pole_coeff(RELEASE_SECONDS, sample_rate),
            live_attack: one_pole_coeff(LIVE_ATTACK_SECONDS, sample_rate),
            lookahead_attack: one_pole_coeff(WALL_LOOKAHEAD_SECONDS / 3.0, sample_rate),
        }
    }
}

#[derive(Clone, Debug)]
struct PeakWindow {
    ring: Vec<(u64, f64)>,
    head: usize,
    len: usize,
    sample_index: u64,
    window: usize,
}

impl PeakWindow {
    fn new(window: usize) -> Self {
        let window = window.max(1);
        Self {
            ring: vec![(0, 0.0); window],
            head: 0,
            len: 0,
            sample_index: 0,
            window,
        }
    }

    fn reset(&mut self) {
        self.head = 0;
        self.len = 0;
        self.sample_index = 0;
    }

    fn push(&mut self, value: f64) -> f64 {
        let value = if value.is_finite() {
            value.max(0.0)
        } else {
            0.0
        };
        let index = self.sample_index;
        self.sample_index += 1;
        let window = self.window as u64;
        while self.len > 0 {
            let (front_idx, _) = self.ring[self.head];
            if index.saturating_sub(front_idx) >= window {
                self.head = (self.head + 1) % self.window;
                self.len -= 1;
            } else {
                break;
            }
        }
        while self.len > 0 {
            let back_pos = (self.head + self.len - 1) % self.window;
            if self.ring[back_pos].1 <= value {
                self.len -= 1;
            } else {
                break;
            }
        }
        let back_pos = (self.head + self.len) % self.window;
        self.ring[back_pos] = (index, value);
        self.len += 1;
        self.ring[self.head].1
    }
}

/// Stateless colour plus safety clip, without anti-aliasing. Stateful
/// limiting lives in [`Wall`], which runs both waveshapers through ADAA;
/// this remains for one-shot use where no sample history exists.
pub fn tick_wall(input: [f64; 2], settings: WallSettings) -> [f64; 2] {
    if !settings.on {
        return input;
    }
    safety(colour(input, settings), settings.ceiling())
}

#[derive(Clone, Debug)]
pub struct Wall {
    delay: Vec<[f64; 2]>,
    peak_window: PeakWindow,
    position: usize,
    gain: f64,
    mode_mix: f64,
    initialized: bool,
    previous_colour_input: [f64; 2],
    previous_live_input: [f64; 2],
    previous_delayed_input: [f64; 2],
    coeffs: Coefficients,
    last_threshold: f64,
    last_ceiling: f64,
}

impl Default for Wall {
    fn default() -> Self {
        Self::new(48_000.0)
    }
}

impl Wall {
    pub fn new(sample_rate: f64) -> Self {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 0.0 {
            sample_rate
        } else {
            48_000.0
        };
        let wanted = wall_lookahead_samples(sample_rate).max(1);
        Self {
            delay: vec![[0.0; 2]; wanted],
            peak_window: PeakWindow::new(wanted),
            position: 0,
            gain: 1.0,
            mode_mix: 0.0,
            initialized: false,
            previous_colour_input: [0.0; 2],
            previous_live_input: [0.0; 2],
            previous_delayed_input: [0.0; 2],
            coeffs: Coefficients::new(sample_rate),
            last_threshold: 0.0,
            last_ceiling: 1.0,
        }
    }

    pub fn prepare(&mut self, sample_rate: f64) {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 0.0 {
            sample_rate
        } else {
            48_000.0
        };
        self.ensure_delay(sample_rate);
    }

    pub fn reset(&mut self) {
        self.delay.fill([0.0; 2]);
        self.peak_window.reset();
        self.position = 0;
        self.gain = 1.0;
        self.initialized = false;
        self.previous_colour_input = [0.0; 2];
        self.previous_live_input = [0.0; 2];
        self.previous_delayed_input = [0.0; 2];
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

    fn ceiling_for(&mut self, threshold: f64) -> f64 {
        if threshold.to_bits() != self.last_threshold.to_bits() {
            self.last_threshold = threshold;
            self.last_ceiling = 10.0_f64.powf(threshold / 20.0).max(1.0e-6);
        }
        self.last_ceiling
    }

    fn ensure_delay(&mut self, sample_rate: f64) {
        let wanted = wall_lookahead_samples(sample_rate).max(1);
        if self.delay.len() != wanted {
            self.delay = vec![[0.0; 2]; wanted];
            self.peak_window = PeakWindow::new(wanted);
            self.position = 0;
            self.gain = 1.0;
            self.initialized = false;
            self.previous_colour_input = [0.0; 2];
            self.previous_live_input = [0.0; 2];
            self.previous_delayed_input = [0.0; 2];
        }
        if sample_rate.to_bits() != self.coeffs.sample_rate.to_bits() {
            self.coeffs = Coefficients::new(sample_rate);
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
            self.mode_mix = one_pole(self.mode_mix, mode_target, self.coeffs.mode);
            if (self.mode_mix - mode_target).abs() < 1.0e-4 {
                self.mode_mix = mode_target;
            }
        }

        // The ring always runs. The detector reads the live (future) sample;
        // the delayed candidate reads the sample leaving the ring. Toggling
        // lookahead crossfades between the two candidates.
        let ceiling = self.ceiling_for(settings.threshold);
        let future = if settings.on {
            let even = settings.even_amount();
            let odd = settings.odd_amount();
            [
                colour_sample_adaa(input[0], self.previous_colour_input[0], even, odd, ceiling),
                colour_sample_adaa(input[1], self.previous_colour_input[1], even, odd, ceiling),
            ]
        } else {
            input
        };
        self.previous_colour_input = input;
        let delayed = self.delay[self.position];
        self.delay[self.position] = future;
        self.position = (self.position + 1) % self.delay.len();
        let future_peak = future[0].abs().max(future[1].abs());
        let window_peak = self.peak_window.push(future_peak);

        if !settings.on {
            self.gain = one_pole(self.gain, 1.0, self.coeffs.release);
            self.previous_live_input = input;
            self.previous_delayed_input = delayed;
            let live = input;
            return [
                live[0] + self.mode_mix * (delayed[0] - live[0]),
                live[1] + self.mode_mix * (delayed[1] - live[1]),
            ];
        }
        // Lookahead gain must target the loudest sample still in the ring,
        // not the newest sample alone. Otherwise gain modulation follows a
        // future phase and lands on the wrong delayed sample.
        let peak = if settings.lookahead {
            window_peak
        } else {
            future_peak
        };
        let target = if peak > ceiling {
            (ceiling / peak).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let coeff = if target < self.gain {
            if settings.lookahead {
                self.coeffs.lookahead_attack
            } else {
                self.coeffs.live_attack
            }
        } else {
            self.coeffs.release
        };
        self.gain = one_pole(self.gain, target, coeff).clamp(0.0, 1.0);

        let gained_live = [future[0] * self.gain, future[1] * self.gain];
        let gained_delayed = [delayed[0] * self.gain, delayed[1] * self.gain];
        let live = [
            safety_sample_adaa(gained_live[0], self.previous_live_input[0], ceiling),
            safety_sample_adaa(gained_live[1], self.previous_live_input[1], ceiling),
        ];
        let delayed_out = [
            safety_sample_adaa(gained_delayed[0], self.previous_delayed_input[0], ceiling),
            safety_sample_adaa(gained_delayed[1], self.previous_delayed_input[1], ceiling),
        ];
        self.previous_live_input = gained_live;
        self.previous_delayed_input = gained_delayed;
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
        // ADAA averages the safety curve across the first emerging step,
        // so that one sample sits mid-ramp and the next is fully limited.
        let emerged = wall.process([2.0, 2.0], active, SR);
        assert!(emerged[0] > 0.4 && emerged[0] <= 1.0);
        let settled = wall.process([2.0, 2.0], active, SR);
        assert!(settled[0] > 0.9 && settled[0] <= 1.0);
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

    #[test]
    fn antiderivatives_match_the_waveshaper_derivatives() {
        let ceiling = 0.5;
        for (even, odd) in [(0.0, 1.0), (1.0, 0.0), (0.7, 0.4)] {
            for input in [-1.2, -0.3, 0.05, 0.8] {
                let step = 1.0e-6;
                let numeric = (colour_antiderivative(input + step, even, odd, ceiling)
                    - colour_antiderivative(input - step, even, odd, ceiling))
                    / (2.0 * step);
                let expected = colour_sample(input, even, odd, ceiling);
                assert!(
                    (numeric - expected).abs() < 1.0e-5,
                    "colour antiderivative at {input}: {numeric} != {expected}"
                );
            }
        }
        for input in [-1.5, -0.4, 0.2, 1.1] {
            let ceiling = 1.0;
            let step = 1.0e-6;
            let numeric = (safety_antiderivative(input + step, ceiling)
                - safety_antiderivative(input - step, ceiling))
                / (2.0 * step);
            let expected = safety_sample(input, ceiling);
            assert!(
                (numeric - expected).abs() < 1.0e-5,
                "safety antiderivative at {input}: {numeric} != {expected}"
            );
        }
    }

    #[test]
    fn adaa_colour_aliases_less_than_the_naive_waveshaper() {
        fn component(samples: &[f64], frequency: f64) -> f64 {
            let mut real = 0.0;
            let mut imag = 0.0;
            for (index, sample) in samples.iter().enumerate() {
                let phase = std::f64::consts::TAU * frequency * index as f64 / SR;
                real += sample * phase.cos();
                imag += sample * phase.sin();
            }
            real.hypot(imag) / samples.len() as f64
        }
        // 7 kHz at 48 kHz: even harmonics at 28/42/56 kHz fold to
        // 20/6/8 kHz, none of which are harmonics of the source. Folded
        // energy there is pure aliasing. Amplitude and ceiling keep the gained signal
        // below the safety knee, so only the colour stage is measured.
        let active = WallSettings {
            even: 100.0,
            ..settings(0.0)
        };
        let mut wall = Wall::new(SR);
        let mut naive = Vec::with_capacity(48_000);
        let mut antialiased = Vec::with_capacity(48_000);
        for index in 0..48_000 {
            let input = 0.3 * (std::f64::consts::TAU * 7_000.0 * index as f64 / SR).sin();
            naive.push(tick_wall([input, input], active)[0]);
            antialiased.push(wall.process([input, input], active, SR)[0]);
        }
        // Measure the steady-state second half only.
        let naive = &naive[24_000..];
        let antialiased = &antialiased[24_000..];
        for alias_frequency in [20_000.0, 8_000.0, 6_000.0] {
            let naive_alias = component(naive, alias_frequency);
            let adaa_alias = component(antialiased, alias_frequency);
            assert!(
                adaa_alias < naive_alias * 0.5,
                "alias at {alias_frequency} Hz: ADAA {adaa_alias} must be well below naive {naive_alias}"
            );
        }
        // The wanted second harmonic must survive anti-aliasing.
        assert!(component(antialiased, 14_000.0) > 0.005);
    }

    #[test]
    fn sliding_peak_matches_delay_buffer_across_toggles() {
        let window_len = wall_lookahead_samples(SR);
        let mut wall = Wall::new(SR);
        let mut reference_ring = vec![[0.0_f64; 2]; window_len];
        let mut ref_pos = 0usize;
        let mut seed = 0x1234_5678_u32;
        for step in 0..2_000 {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let a = (seed as i32) as f64 / i32::MAX as f64 * 2.5;
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let b = (seed as i32) as f64 / i32::MAX as f64 * 2.5;
            let active = WallSettings {
                even: 35.0,
                odd: 20.0,
                threshold: if step % 300 < 150 { -6.0 } else { -12.0 },
                on: step % 170 >= 30,
                lookahead: step % 120 >= 40,
            };
            let prev = wall.previous_colour_input;
            let ceiling = active.ceiling();
            let future = if active.on {
                [
                    colour_sample_adaa(a, prev[0], active.even_amount(), active.odd_amount(), ceiling),
                    colour_sample_adaa(b, prev[1], active.even_amount(), active.odd_amount(), ceiling),
                ]
            } else {
                [a, b]
            };
            reference_ring[ref_pos] = future;
            ref_pos = (ref_pos + 1) % window_len;
            let expected_peak = reference_ring
                .iter()
                .fold(0.0_f64, |p, s| p.max(s[0].abs()).max(s[1].abs()));
            wall.process([a, b], active, SR);
            let got_peak = wall.peak_window.ring[wall.peak_window.head].1;
            assert!(
                (got_peak - expected_peak).abs() < 1.0e-12,
                "step {step}: deque peak {got_peak} != brute-force {expected_peak}"
            );
            assert!(wall.peak_window.len <= wall.peak_window.window);
        }
    }
}
