use crate::audio::sample::Sample;

#[derive(Copy, Debug, Clone, PartialEq, Eq, Default)]
pub struct ValRange<T: Sample> {
    pub min: T,
    pub max: T,
}

// impl<T: Sample + std::ops::Sub<Output = T>> ValRange<T> {
//     pub fn len(&self) -> T {
//         self.max - self.min
//     }
//     pub fn is_empty(&self) -> bool {
//         self.len() == T::ZERO
//     }
// }

impl<T: Sample> ValRange<T> {
    pub fn len(&self) -> f64 {
        self.max.distance(self.min)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0.0
    }
}
impl<T: Sample> ValRange<T> {
    //// Extend the range to include the given value
    pub fn include(&mut self, val: T) {
        if val < self.min {
            self.min = val;
        }
        if val > self.max {
            self.max = val;
        }
    }

    /// Check if the given value is within the range
    pub fn includes(&self, val: T) -> bool {
        val >= self.min && val <= self.max
    }

    /// Extend the range to include the given range
    pub fn extend(&mut self, other: Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    /// Check if the given range is within the range
    pub fn contains(&self, other: Self) -> bool {
        self.min <= other.min && self.max >= other.max
    }
}

#[derive(Copy, Debug, Clone, PartialEq)]
pub enum ValRangeE {
    PCM16(ValRange<i16>),
    PCM24(ValRange<i32>),
    PCM32(ValRange<i32>),
    F32(ValRange<f32>),
}

impl ValRangeE {
    /// Returns the Full Scale values for given bit depth/sample type
    pub fn min_max(&self) -> Self {
        match self {
            ValRangeE::PCM16(_) => ValRangeE::PCM16(PCM16_RANGE),
            ValRangeE::PCM24(_) => ValRangeE::PCM24(PCM24_RANGE),
            ValRangeE::PCM32(_) => ValRangeE::PCM32(PCM32_RANGE),
            ValRangeE::F32(_) => ValRangeE::F32(FLOAT_RANGE),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            ValRangeE::PCM16(range) => range.is_empty(),
            ValRangeE::PCM24(range) => range.is_empty(),
            ValRangeE::PCM32(range) => range.is_empty(),
            ValRangeE::F32(range) => range.is_empty(),
        }
    }

    pub fn len(&self) -> f64 {
        match self {
            ValRangeE::PCM16(range) => range.len(),
            ValRangeE::PCM24(range) => range.len(),
            ValRangeE::PCM32(range) => range.len(),
            ValRangeE::F32(range) => range.len(),
        }
    }
}

pub const PCM16_RANGE: ValRange<i16> = sample_range_i16(16);
pub const PCM24_RANGE: ValRange<i32> = sample_range_i32(24);
pub const PCM32_RANGE: ValRange<i32> = sample_range_i32(32);
pub const FLOAT_RANGE: ValRange<f32> = ValRange::<f32> {
    min: -1.0,
    max: 1.0,
};

const fn sample_range_i16(bit_depth: u32) -> ValRange<i16> {
    assert!(bit_depth <= 16);
    let max = ((1_u16 << (bit_depth - 1)) - 1) as i16;
    let min = -max - 1;
    ValRange::<i16> { min, max }
}

const fn sample_range_i32(bit_depth: u32) -> ValRange<i32> {
    assert!(bit_depth <= 32);
    let max = ((1_u32 << (bit_depth - 1)) - 1) as i32;
    let min = -max - 1;
    ValRange::<i32> { min, max }
}
