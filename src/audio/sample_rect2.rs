use crate::audio::{self, sample, sample2::Sample, sample_range2::SampleFractionalIxRange};

///
/// Rectangle over a buffer with audio samples
/// Represenst a window in the buffer.
/// val_rng has no need to be fractional? Probably not, enough values, probaly not zooming in that
/// much?
/// TODO: rename to Fractional.. if we need a non-fractional I guess
///
#[derive(Debug, Clone, PartialEq)]
pub struct SampleRect<T: Sample> {
    /// X range in f64 (for zooming/moving with sub-sample resolution)
    pub ix_rng: SampleFractionalIxRange,
    /// Y range in SampleType
    pub val_rng: Option<sample::ValRange<T>>,
}

impl<T: Sample> SampleRect<T> {
    /// Rectangle contains the whole buffer
    pub fn from_buffer(buffer: &audio::buffer2::Buffer<T>) -> Self {
        let &min = buffer.min().unwrap_or(&T::ZERO);
        let &max = buffer.max().unwrap_or(&T::ZERO);
        Self {
            ix_rng: SampleFractionalIxRange(0.0..buffer.nr_samples() as f64),
            val_rng: Some(sample::ValRange::<T> { min, max }),
        }
    }
}

/// Dynamically typed sample rect
#[derive(Debug, Clone, PartialEq)]
pub enum SampleRectEnum {
    F32(SampleRect<f32>),
    I32(SampleRect<i32>),
    I16(SampleRect<i16>),
}

impl SampleRectEnum {
    pub fn from_buffer(buffer: &audio::SampleBuffer) -> Self {
        match buffer {
            audio::SampleBuffer::F32(buffer) => SampleRectEnum::F32(SampleRect::<f32>::from_buffer(buffer)),
            audio::SampleBuffer::I32(buffer) => SampleRectEnum::I32(SampleRect::<i32>::from_buffer(buffer)),
            audio::SampleBuffer::I16(buffer) => SampleRectEnum::I16(SampleRect::<i16>::from_buffer(buffer)),
        }
    }
}
