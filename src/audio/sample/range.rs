use std::ops::Range;

use crate::audio::sample2::Sample;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValRange<T: Sample> {
    pub min: T,
    pub max: T,
}

pub type SampleIx = i64;

/// Half open range of sample indices `[start, end)`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct IxRange {
    pub start: SampleIx,
    pub end: SampleIx,
}

impl IxRange {
    pub fn len(&self) -> SampleIx {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn positive_len(&self) -> SampleIx {
        self.end.max(self.start) - self.start
    }
}

/// Enable (start..end).into()
impl From<Range<SampleIx>> for IxRange {
    fn from(r: Range<SampleIx>) -> Self {
        IxRange {
            start: r.start,
            end: r.end,
        }
    }
}

/// Fractional index of an audio sample is useful for zooming/moving at sub-sample resolution
pub type FracSampleIx = f64;

/// Half open range of sample indices `[start, end)`
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FracIxRange {
    pub start: FracSampleIx,
    pub end: FracSampleIx,
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
