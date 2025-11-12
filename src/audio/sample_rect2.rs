use crate::audio::{self, buffer2::BufferE, sample, sample2::Sample, sample_range2::SampleFractionalIxRange};

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
pub enum SampleRectE {
    F32(SampleRect<f32>),
    I32(SampleRect<i32>),
    I16(SampleRect<i16>),
}

impl SampleRectE {
    pub fn from_sample_buffer(buffer: &audio::SampleBuffer) -> Self {
        match buffer {
            audio::SampleBuffer::F32(buffer) => SampleRectE::F32(SampleRect::<f32>::from_buffer(buffer)),
            audio::SampleBuffer::I32(buffer) => SampleRectE::I32(SampleRect::<i32>::from_buffer(buffer)),
            audio::SampleBuffer::I16(buffer) => SampleRectE::I16(SampleRect::<i16>::from_buffer(buffer)),
        }
    }

    pub fn from_buffer(buffer: &BufferE) -> Self {
        match buffer {
            BufferE::F32(buffer) => SampleRectE::F32(SampleRect::<f32>::from_buffer(buffer)),
            BufferE::I32(buffer) => SampleRectE::I32(SampleRect::<i32>::from_buffer(buffer)),
            BufferE::I16(buffer) => SampleRectE::I16(SampleRect::<i16>::from_buffer(buffer)),
        }
    }
}
