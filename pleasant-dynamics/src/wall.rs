#[derive(Clone, Copy, Debug)]
pub struct WallSettings {
    pub even: f64,
    pub odd: f64,
    pub threshold: f64,
    pub on: bool,
}

impl Default for WallSettings {
    fn default() -> Self {
        Self {
            even: 0.0,
            odd: 0.0,
            threshold: 0.0,
            on: true,
        }
    }
}

fn wall_sample(input: f64, even: f64, odd: f64, threshold: f64) -> f64 {
    let ceiling = 10.0_f64.powf(threshold / 20.0).max(1.0e-6);
    let saturated = (input / ceiling).tanh();
    let odd_path = saturated * ceiling;
    let even_path = saturated * saturated * ceiling;
    let colored = input + odd * (odd_path - input) + even * even_path;
    // The harmonic controls color the signal before the always-active soft clip.
    // At zero, this is exactly ceiling * tanh(input / ceiling).
    ceiling * (colored / ceiling).tanh()
}

pub fn tick_wall(input: [f64; 2], settings: WallSettings) -> [f64; 2] {
    if !settings.on {
        return input;
    }
    let even = (settings.even * 0.01).clamp(0.0, 1.0);
    let odd = (settings.odd * 0.01).clamp(0.0, 1.0);
    [
        wall_sample(input[0], even, odd, settings.threshold),
        wall_sample(input[1], even, odd, settings.threshold),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_harmonics_still_apply_threshold_scaled_tanh() {
        for threshold in [0.0, -12.0, -48.0] {
            let ceiling = 10.0_f64.powf(threshold / 20.0);
            let input = [2.0 * ceiling, -0.5 * ceiling];
            let output = tick_wall(
                input,
                WallSettings {
                    threshold,
                    ..WallSettings::default()
                },
            );
            for channel in 0..2 {
                let expected = ceiling * (input[channel] / ceiling).tanh();
                assert!(
                    (output[channel] - expected).abs() < 1.0e-12,
                    "zero-harmonic WALL must apply tanh at {threshold} dB"
                );
            }
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
    }

    #[test]
    fn harmonic_controls_preserve_the_ceiling_and_silence() {
        for threshold in [0.0, -12.0, -48.0] {
            let ceiling = 10.0_f64.powf(threshold / 20.0);
            for even in [0.0, 50.0, 100.0] {
                for odd in [0.0, 50.0, 100.0] {
                    let settings = WallSettings {
                        even,
                        odd,
                        threshold,
                        on: true,
                    };
                    assert_eq!(tick_wall([0.0; 2], settings), [0.0; 2]);
                    for step in -100..=100 {
                        let input = step as f64 * ceiling;
                        for output in tick_wall([input, -input], settings) {
                            assert!(output.is_finite() && output.abs() <= ceiling);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn controls_add_even_and_odd_harmonics_to_base_clipping() {
        fn harmonic(settings: WallSettings, number: usize) -> f64 {
            let mut real = 0.0;
            let mut imag = 0.0;
            for sample in 0..4096 {
                let phase = std::f64::consts::TAU * sample as f64 / 4096.0;
                let output = tick_wall([0.8 * phase.sin(); 2], settings)[0];
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
