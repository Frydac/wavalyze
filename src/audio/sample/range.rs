use std::fmt::Debug;
use std::ops::Range;

use crate::audio::sample2::Sample;

#[derive(Copy, Debug, Clone, PartialEq, Eq, Default)]
pub struct ValRange<T: Sample> {
    pub min: T,
    pub max: T,
}

pub type SampleIx = i64;
pub type IxRange = IxRangeG<SampleIx>;
pub type FracSampleIx = f64;
pub type FracIxRange = IxRangeG<FracSampleIx>;

pub trait Ix: Debug + Default + Copy + PartialOrd + PartialEq + Clone {}

impl Ix for SampleIx {}
impl Ix for FracSampleIx {}

#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct IxRangeG<T: Ix> {
    pub start: T,
    pub end: T,
}

impl<T: Ix + std::ops::Sub<Output = T>> IxRangeG<T> {
    pub fn len(&self) -> T {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool { self.len() == T::default() }
}

impl<T> From<Range<T>> for IxRangeG<T>
where
    T: Ix,
{
    fn from(r: Range<T>) -> Self {
        IxRangeG {
            start: r.start,
            end: r.end,
        }
    }
}

pub const PCM16_RANGE: ValRange<i16> = sample_range_i16(16);
pub const PCM24_RANGE: ValRange<i32> = sample_range_i32(24);
pub const PCM32_RANGE: ValRange<i32> = sample_range_i32(32);
pub const FLOAT_RANGE: ValRange<f32> = ValRange::<f32> { min: -1.0, max: 1.0 };

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
