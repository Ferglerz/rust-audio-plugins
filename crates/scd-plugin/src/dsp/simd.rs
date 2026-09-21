//! Stereo f32x2 helpers for the audio thread.
//! Uses NEON / SSE when available; scalar otherwise. No extra crates.

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::{
    float32x2_t, vadd_f32, vdup_n_f32, vfma_f32, vget_lane_f32, vmul_f32, vset_lane_f32, vsub_f32,
};

#[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
use std::arch::x86_64::{
    __m128, _mm_add_ps, _mm_cvtss_f32, _mm_mul_ps, _mm_set_ps, _mm_shuffle_ps, _mm_sub_ps,
};

#[inline(always)]
fn load2(v: [f32; 2]) -> Stereo {
    Stereo::new(v[0], v[1])
}

#[inline(always)]
fn splat(x: f32) -> Stereo {
    Stereo::splat(x)
}

#[derive(Clone, Copy)]
struct Stereo {
    #[cfg(target_arch = "aarch64")]
    v: float32x2_t,
    #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
    v: __m128,
    #[cfg(not(any(
        target_arch = "aarch64",
        all(target_arch = "x86_64", target_feature = "sse")
    )))]
    v: [f32; 2],
}

impl Stereo {
    #[inline(always)]
    fn new(l: f32, r: f32) -> Self {
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                v: unsafe { vset_lane_f32(r, vdup_n_f32(l), 1) },
            }
        }
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            Self {
                v: unsafe { _mm_set_ps(0.0, 0.0, r, l) },
            }
        }
        #[cfg(not(any(
            target_arch = "aarch64",
            all(target_arch = "x86_64", target_feature = "sse")
        )))]
        {
            Self { v: [l, r] }
        }
    }

    #[inline(always)]
    fn splat(x: f32) -> Self {
        Self::new(x, x)
    }

    #[inline(always)]
    fn add(self, other: Self) -> Self {
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                v: unsafe { vadd_f32(self.v, other.v) },
            }
        }
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            Self {
                v: unsafe { _mm_add_ps(self.v, other.v) },
            }
        }
        #[cfg(not(any(
            target_arch = "aarch64",
            all(target_arch = "x86_64", target_feature = "sse")
        )))]
        {
            Self {
                v: [self.v[0] + other.v[0], self.v[1] + other.v[1]],
            }
        }
    }

    #[inline(always)]
    fn sub(self, other: Self) -> Self {
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                v: unsafe { vsub_f32(self.v, other.v) },
            }
        }
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            Self {
                v: unsafe { _mm_sub_ps(self.v, other.v) },
            }
        }
        #[cfg(not(any(
            target_arch = "aarch64",
            all(target_arch = "x86_64", target_feature = "sse")
        )))]
        {
            Self {
                v: [self.v[0] - other.v[0], self.v[1] - other.v[1]],
            }
        }
    }

    #[inline(always)]
    fn mul(self, other: Self) -> Self {
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                v: unsafe { vmul_f32(self.v, other.v) },
            }
        }
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            Self {
                v: unsafe { _mm_mul_ps(self.v, other.v) },
            }
        }
        #[cfg(not(any(
            target_arch = "aarch64",
            all(target_arch = "x86_64", target_feature = "sse")
        )))]
        {
            Self {
                v: [self.v[0] * other.v[0], self.v[1] * other.v[1]],
            }
        }
    }

    /// `self + other * scale`
    #[inline(always)]
    fn mul_add(self, other: Self, scale: Self) -> Self {
        #[cfg(target_arch = "aarch64")]
        {
            Self {
                v: unsafe { vfma_f32(self.v, other.v, scale.v) },
            }
        }
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        {
            Self {
                v: unsafe { _mm_add_ps(self.v, _mm_mul_ps(other.v, scale.v)) },
            }
        }
        #[cfg(not(any(
            target_arch = "aarch64",
            all(target_arch = "x86_64", target_feature = "sse")
        )))]
        {
            Self {
                v: [
                    self.v[0] + other.v[0] * scale.v[0],
                    self.v[1] + other.v[1] * scale.v[1],
                ],
            }
        }
    }

    #[inline(always)]
    fn to_array(self) -> [f32; 2] {
        #[cfg(target_arch = "aarch64")]
        unsafe {
            [vget_lane_f32(self.v, 0), vget_lane_f32(self.v, 1)]
        }
        #[cfg(all(target_arch = "x86_64", target_feature = "sse"))]
        unsafe {
            let l = _mm_cvtss_f32(self.v);
            let r = _mm_cvtss_f32(_mm_shuffle_ps(self.v, self.v, 0x55));
            [l, r]
        }
        #[cfg(not(any(
            target_arch = "aarch64",
            all(target_arch = "x86_64", target_feature = "sse")
        )))]
        {
            self.v
        }
    }
}

/// `out = (a + frac * (b - a)) * scale` for a stereo pair.
#[inline(always)]
pub fn stereo_lerp_scale(a: [f32; 2], b: [f32; 2], frac: f32, scale: [f32; 2]) -> [f32; 2] {
    let a = load2(a);
    let b = load2(b);
    let frac = splat(frac);
    let scale = load2(scale);
    a.mul_add(b.sub(a), frac).mul(scale).to_array()
}

#[inline(always)]
pub fn add_stereo(acc: &mut [f32; 2], sample: [f32; 2]) {
    *acc = load2(*acc).add(load2(sample)).to_array();
}

#[inline(always)]
pub fn sum_stereo_pairs<const N: usize>(pairs: &[[f32; 2]; N]) -> [f32; 2] {
    let mut acc = Stereo::splat(0.0);
    for pair in pairs {
        acc = acc.add(load2(*pair));
    }
    acc.to_array()
}

#[cfg(test)]
mod tests {
    use super::{stereo_lerp_scale, sum_stereo_pairs};

    #[test]
    fn stereo_lerp_matches_scalar() {
        let a = [0.25, -0.5];
        let b = [1.0, 0.5];
        let frac = 0.3;
        let scale = [0.8, 1.2];
        let got = stereo_lerp_scale(a, b, frac, scale);
        let expect = [
            (a[0] + frac * (b[0] - a[0])) * scale[0],
            (a[1] + frac * (b[1] - a[1])) * scale[1],
        ];
        assert!((got[0] - expect[0]).abs() < 1e-6);
        assert!((got[1] - expect[1]).abs() < 1e-6);
    }

    #[test]
    fn stereo_sum_matches_scalar() {
        let pairs = [[0.1, 0.2], [-0.3, 0.4], [0.5, -0.6]];
        let got = sum_stereo_pairs(&pairs);
        assert!((got[0] - 0.3).abs() < 1e-6);
        assert!((got[1] - 0.0).abs() < 1e-6);
    }
}
