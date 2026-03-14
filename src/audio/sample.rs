pub mod convert;
pub mod ix_range;
pub mod val_range;
pub mod view;

// use crate::audio::sample::ValRange;
pub use ix_range::IxRange;
pub use ix_range::{FracIx, FracIxRange, Ix, OptIxRange};
use num_traits::ToPrimitive;
use std::fmt::Debug;
pub use val_range::{ValRange, ValRangeE};
pub use view::View;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SampleType {
    Float,
    Int,
}

impl From<hound::SampleFormat> for SampleType {
    fn from(sample_type: hound::SampleFormat) -> Self {
        match sample_type {
            hound::SampleFormat::Float => SampleType::Float,
            hound::SampleFormat::Int => SampleType::Int,
        }
    }
}

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
    fn to_norm(self, bit_depth: u16) -> f64;

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
        if self < other { self } else { other }
    }

    fn max(self, other: Self) -> Self {
        if self.is_nan() {
            return other;
        }
        if other.is_nan() {
            return self;
        }
        if self > other { self } else { other }
    }

    fn val_range(_bit_depth: u16) -> ValRange<Self> {
        ValRange {
            min: -1.0,
            max: 1.0,
        }
    }

    fn distance(self, other: Self) -> f64 {
        (self as f64 - other as f64).abs()
    }

    fn to_norm(self, _bit_depth: u16) -> f64 {
        self as f64
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

    fn to_norm(self, bit_depth: u16) -> f64 {
        let bit_depth = bit_depth.clamp(1, 32) as u32;
        self as f64 * crate::audio::sample::convert::pcm2float_factor(bit_depth)
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

    fn to_norm(self, _bit_depth: u16) -> f64 {
        crate::audio::sample::convert::pcm162flt(self)
    }

    const MAX: Self = i16::MAX;
    const MIN: Self = i16::MIN;
    const ZERO: Self = 0;
}

impl Sample for f64 {
    fn is_nan(&self) -> bool {
        f64::is_nan(*self)
    }

    fn min(self, other: Self) -> Self {
        if self.is_nan() {
            return other;
        }
        if other.is_nan() {
            return self;
        }
        if self < other { self } else { other }
    }

    fn max(self, other: Self) -> Self {
        if self.is_nan() {
            return other;
        }
        if other.is_nan() {
            return self;
        }
        if self > other { self } else { other }
    }

    fn val_range(_bit_depth: u16) -> ValRange<Self> {
        ValRange {
            min: -1.0,
            max: 1.0,
        }
    }

    fn distance(self, other: Self) -> f64 {
        (self - other).abs()
    }

    fn to_norm(self, _bit_depth: u16) -> f64 {
        self
    }

    const MAX: Self = f64::INFINITY;
    const MIN: Self = f64::NEG_INFINITY;
    const ZERO: Self = 0.0;
}
