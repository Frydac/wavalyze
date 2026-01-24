use num_traits::ToPrimitive;

use crate::audio::sample::ValRange;
use std::fmt::Debug;

// Represents a single audio sample value
pub trait Sample: Debug + Default + Copy + PartialOrd + PartialEq + Clone + ToPrimitive {
    // Needed for dealing with partial ordering of floats
    fn is_nan(&self) -> bool;

    fn min(self, other: Self) -> Self;
    fn max(self, other: Self) -> Self;

    fn val_range(bit_depth: u16) -> ValRange<Self>
    where
        Self: Sized;

    fn distance(self, other: Self) -> f64;

    const MAX: Self;
    const MIN: Self;
    const ZERO: Self;
}
impl Sample for f32 {
    fn is_nan(&self) -> bool {
        f32::is_nan(*self)
    }

    fn min(self, other: Self) -> Self {
        if self.is_nan() {
            return other;
        }
        if other.is_nan() {
            return self;
        }
        if self < other {
            self
        } else {
            other
        }
    }

    fn max(self, other: Self) -> Self {
        if self.is_nan() {
            return other;
        }
        if other.is_nan() {
            return self;
        }
        if self > other {
            self
        } else {
            other
        }
    }

    fn val_range(_bit_depth: u16) -> ValRange<Self> {
        ValRange { min: -1.0, max: 1.0 }
    }

    fn distance(self, other: Self) -> f64 {
        (self as f64 - other as f64).abs()
    }

    const MAX: Self = f32::INFINITY;
    const MIN: Self = f32::NEG_INFINITY;
    const ZERO: Self = 0.0;
}
impl Sample for i32 {
    fn is_nan(&self) -> bool {
        false
    }

    fn min(self, other: Self) -> Self {
        std::cmp::min(self, other)
    }

    fn max(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }

    fn val_range(bit_depth: u16) -> ValRange<Self> {
        if bit_depth > 32 || bit_depth == 0 {
            return ValRange {
                min: i32::MIN,
                max: i32::MAX,
            };
        }
        let max = ((1_u32 << (bit_depth - 1)) - 1) as i32;
        let min = -max - 1;
        ValRange { min, max }
    }

    fn distance(self, other: Self) -> f64 {
        (self as f64 - other as f64).abs()
    }

    const MAX: Self = i32::MAX;
    const MIN: Self = i32::MIN;
    const ZERO: Self = 0;
}
impl Sample for i16 {
    fn is_nan(&self) -> bool {
        false
    }

    fn min(self, other: Self) -> Self {
        std::cmp::min(self, other)
    }

    fn max(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }

    fn val_range(bit_depth: u16) -> ValRange<Self> {
        if bit_depth > 16 || bit_depth == 0 {
            return ValRange {
                min: i16::MIN,
                max: i16::MAX,
            };
        }
        let max = ((1_u16 << (bit_depth - 1)) - 1) as i16;
        let min = -max - 1;
        ValRange { min, max }
    }

    fn distance(self, other: Self) -> f64 {
        (self as f64 - other as f64).abs()
    }

    const MAX: Self = i16::MAX;
    const MIN: Self = i16::MIN;
    const ZERO: Self = 0;
}
