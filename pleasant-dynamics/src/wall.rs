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
    if even <= 0.0 && odd <= 0.0 {
        return input;
    }
    let ceiling = 10.0_f64.powf(threshold / 20.0).max(1.0e-6);
    let saturated = (input / ceiling).tanh();
    let odd_path = saturated * ceiling;
    let even_path = saturated * saturated * ceiling;
    input + odd * (odd_path - input) + even * even_path
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
    fn neutral_and_bypass_are_identity() {
        let input = [0.8, -0.4];
        assert_eq!(tick_wall(input, WallSettings::default()), input);
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
}
