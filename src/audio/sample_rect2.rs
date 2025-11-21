use crate::audio::{self, buffer2::BufferE, sample, sample2::Sample};

///
/// A 2D 'camera' view expressed in terms of sample indices and sample values.
///
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct SampleRect<T: Sample> {
    /// X range in f64 (for zooming/moving with sub-sample resolution)
    pub ix_rng: sample::FracIxRange,
    /// Y range in SampleType
    pub val_rng: Option<sample::ValRange<T>>,
}

impl<T: Sample> SampleRect<T> {
    /// Rectangle contains the whole buffer
    pub fn from_buffer(buffer: &audio::buffer2::Buffer<T>) -> Self {
        Self {
            ix_rng: sample::FracIxRange {
                start: 0.0,
                end: buffer.nr_samples() as f64,
            },
            val_rng: Some(sample::ValRange::<T> { min: T::MIN, max: T::MAX }),
        }
    }
}

/// Dynamically typed sample rect
#[derive(Debug, Clone, PartialEq, Copy)]
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

    pub fn from_buffere(buffer: &BufferE) -> Self {
        match buffer {
            BufferE::F32(buffer) => SampleRectE::F32(SampleRect::<f32>::from_buffer(buffer)),
            BufferE::I32(buffer) => SampleRectE::I32(SampleRect::<i32>::from_buffer(buffer)),
            BufferE::I16(buffer) => SampleRectE::I16(SampleRect::<i16>::from_buffer(buffer)),
        }
    }
}

impl SampleRectE {
    pub fn ix_rng(&self) -> sample::FracIxRange {
        match self {
            SampleRectE::F32(rect) => rect.ix_rng,
            SampleRectE::I32(rect) => rect.ix_rng,
            SampleRectE::I16(rect) => rect.ix_rng,
        }
    }
}
