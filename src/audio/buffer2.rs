use std::ops::{Deref, DerefMut};

use crate::audio::sample;
use crate::audio::sample2::Sample;

/// One channel of audio samples.  
/// Could be used for storing interleaved samples, but not yet used like that? Probably want some
/// kind of enum for that?
#[derive(Debug, PartialEq, Clone)]
pub struct Buffer<T: Sample> {
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub data: Vec<T>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum BufferE {
    F32(Buffer<f32>),
    I32(Buffer<i32>),
    I16(Buffer<i16>),
}

/// Constructors
impl<T: Sample> Buffer<T> {
    pub fn new(sample_rate: u32, bit_depth: u16) -> Self {
        Self {
            sample_rate,
            bit_depth,
            data: vec![],
        }
    }

    pub fn with_capacity(sample_rate: u32, bit_depth: u16, capacity: usize) -> Self {
        Self {
            sample_rate,
            bit_depth,
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn with_size(sample_rate: u32, bit_depth: u16, size: usize) -> Self {
        let mut result = Self {
            sample_rate,
            bit_depth,
            data: Vec::with_capacity(size),
        };
        result.data.resize(size, T::default());
        result
    }
}

impl<T: Sample> Buffer<T> {
    pub fn nr_samples(&self) -> usize {
        self.data.len()
    }

    pub fn duration_s(&self) -> f64 {
        self.data.len() as f64 / self.sample_rate as f64
    }

    /// Returns the minimum value in the buffer
    /// NOTE: we filter out NaN values, they have no effect (we expect no NaN values in general)
    pub fn min(&self) -> Option<&T> {
        self.data
            .iter()
            .filter(|&&x| !x.is_nan()) // Remove NaN values
            .min_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Returns the maximum value in the buffer
    /// NOTE: we ignore NaN values, similar to C or C++ afaik
    pub fn max(&self) -> Option<&T> {
        self.data.iter().filter(|&&x| !x.is_nan()).max_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Returns the minimum value in the buffer
    /// NOTE: NaN values are propagated I think, similar to C or C++ afaik
    /// * it propagates NaN in unpredictable ways, depending of it comes first or second,
    ///   comparison is always false
    /// * doesn't do any checks, so probably fastest
    pub fn min_fold(&self) -> T {
        self.data.iter().fold(T::MAX, |a, &b| if b < a { b } else { a })
    }

    pub fn max_fold(&self) -> T {
        self.data.iter().fold(T::MIN, |a, &b| if b > a { b } else { a })
    }

    pub fn val_range(&self) -> sample::ValRange<T> {
        T::val_range(self.bit_depth)
    }
}

/// Use deref to access the underlying buffer
/// Impies Indexing and iterator support (not IntoIterator! Deref takes a reference)
impl<T: Sample> Deref for Buffer<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl<T: Sample> DerefMut for Buffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

// TODO: Enum -> SampleValueRange<T>
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleType {
    Float,
    Int(u32),
}

// pub const PCM16_FORMAT: SampleType = SampleType::Int(16);
// pub const PCM24_FORMAT: SampleType = SampleType::Int(24);
// pub const PCM32_FORMAT: SampleType = SampleType::Int(32);
// pub const FLOAT_FORMAT: SampleType = SampleType::Float;

// pub const PCM16_RANGE: SampleValueRange<i16> = sample_range_i16(16);
// pub const PCM24_RANGE: SampleValueRange<i32> = sample_range_i32(24);
// pub const PCM32_RANGE: SampleValueRange<i32> = sample_range_i32(32);
// pub const FLOAT_RANGE: SampleValueRange<f32> = SampleValueRange::<f32>{min: -1.0, max: 1.0};

// This can't work at compile time (yet) unfortunately
// pub fn sample_range_generic<T: Sample + From<i64>>(bit_depth: u32) -> SampleValueRange<T> {
//     let min_i64 = -(1_i64 << (bit_depth - 1));
//     let max_i64 = (1_i64 << (bit_depth - 1)) - 1;
//     SampleValueRange::new(T::from(.min_i64), T::from(max_i64))
// }
