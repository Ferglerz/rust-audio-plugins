//! The offline and live engines evaluate the same equations at different precision.
use std::ops::{Add, AddAssign, Mul, Sub};
use wide::{f32x8, f64x4};

pub trait Lanes:
    Copy + Default + Add<Output = Self> + AddAssign + Sub<Output = Self> + Mul<Output = Self>
{
    const WIDTH: usize;
    const ZERO: Self;
    const ONE: Self;
    fn uniform(value: f64) -> Self;
    fn set_lane(&mut self, lane: usize, value: f64);
    fn magnitude(self) -> Self;
    fn sum_lanes(self) -> f64;
}

macro_rules! lanes {
    ($vector:ty, $scalar:ty, $width:expr) => {
        impl Lanes for $vector {
            const WIDTH: usize = $width;
            const ZERO: Self = <$vector>::ZERO;
            const ONE: Self = <$vector>::ONE;
            #[inline]
            fn uniform(value: f64) -> Self {
                Self::splat(value as $scalar)
            }
            #[inline]
            fn set_lane(&mut self, lane: usize, value: f64) {
                self.as_array_mut()[lane] = value as $scalar;
            }
            #[inline]
            fn magnitude(self) -> Self {
                self.abs()
            }
            #[inline]
            fn sum_lanes(self) -> f64 {
                self.reduce_add() as f64
            }
        }
    };
}
lanes!(f64x4, f64, 4);
lanes!(f32x8, f32, 8);
