//! Core math utilities ported from `MathUtils/00_core_math.jsfx-inc`.

use super::constants::EPS;

const LOG10_20: f64 = 20.0 / std::f64::consts::LN_10;
const LOG_10_20: f64 = std::f64::consts::LN_10 / 20.0;

#[inline]
pub fn lerp_f64(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[macro_export]
macro_rules! impl_lerp_f64 {
    ($struct:ident { $($field:ident),+ $(,)? }) => {
        impl $struct {
            pub fn lerp(self, target: Self, one_minus: f64) -> Self {
                Self {
                    $($field: $crate::dsp::core_math::lerp_f64(self.$field, target.$field, one_minus),)+
                }
            }
        }
    };
}

#[inline]
pub fn db_per_sec_to_ms(db_per_sec: f64) -> f64 {
    10000.0 / db_per_sec
}

#[inline]
pub fn ms_to_coeff(time_ms: f64, srate: f64) -> f64 {
    (-1000.0 / (time_ms * srate)).exp()
}

#[derive(Debug, Clone, Copy)]
pub struct OnePole {
    pub y: f64,
}

impl OnePole {
    pub fn new() -> Self {
        Self { y: 0.0 }
    }

    #[inline]
    pub fn process(&mut self, x: f64, coeff: f64) -> f64 {
        self.y = coeff * x + (1.0 - coeff) * self.y;
        self.y
    }
}

/// Fast tanh (Padé approximant) with JSFX hard clamp beyond ±10.
///
/// `x * (27 + x²) / (27 + 9x²)` — no `exp`; output clamped to ±1 inside the knee.
#[inline]
pub fn tanh_jsfx(x: f64) -> f64 {
    if x > 10.0 {
        return 1.0;
    }
    if x < -10.0 {
        return -1.0;
    }
    let x2 = x * x;
    (x * (27.0 + x2) / (27.0 + 9.0 * x2)).clamp(-1.0, 1.0)
}

#[inline]
pub fn db_to_linear(db: f64) -> f64 {
    (db * LOG_10_20).exp()
}

#[inline]
pub fn linear_to_db(linear: f64) -> f64 {
    if linear > 0.0 {
        linear.ln() * LOG10_20
    } else {
        -150.0
    }
}

#[inline]
pub fn sign_jsfx(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else {
        0.0
    }
}

#[allow(dead_code)]
pub fn is_near_zero(x: f64) -> bool {
    x.abs() < EPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_roundtrip() {
        for db in [-60.0, -20.0, 0.0, 6.0, 12.0] {
            let lin = db_to_linear(db);
            let back = linear_to_db(lin);
            assert!((back - db).abs() < 1e-9);
        }
    }

    #[test]
    fn test_tanh_jsfx_extremes() {
        assert_eq!(tanh_jsfx(20.0), 1.0);
        assert_eq!(tanh_jsfx(-20.0), -1.0);
    }

    #[test]
    fn tanh_jsfx_stays_in_unit_interval() {
        for i in -100..=100 {
            let x = i as f64 * 0.2;
            let y = tanh_jsfx(x);
            assert!(y >= -1.0 && y <= 1.0, "x={x} y={y}");
        }
    }

    #[test]
    fn tanh_jsfx_is_monotonic_in_limiter_range() {
        let mut prev = tanh_jsfx(-2.0);
        for i in -20..=20 {
            let x = i as f64 * 0.1;
            let y = tanh_jsfx(x);
            assert!(y >= prev - 1e-12, "x={x} prev={prev} y={y}");
            prev = y;
        }
    }

    #[test]
    fn tanh_jsfx_near_origin() {
        assert!((tanh_jsfx(0.0)).abs() < 1e-15);
        assert!((tanh_jsfx(0.95) - 0.95f64.tanh()).abs() < 0.02);
    }
}
