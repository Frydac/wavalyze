use crate::audio::{self, buffer2::BufferE, sample, sample2::Sample};

///
/// A 2D 'camera' view expressed in terms of sample indices and sample values.
///
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct SampleRect<T: Sample> {
    /// X range in f64 (for zooming/moving with sub-sample resolution)
    pub ix_rng: sample::FracIxRange,
    /// Y range in SampleType
    /// TODO: maybe not an option?
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
            val_rng: Some(buffer.val_range()),
        }
    }

    pub fn width(&self) -> f32 {
        self.ix_rng.len() as f32
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
    pub fn from_buffere(buffer: &BufferE) -> Self {
        match buffer {
            BufferE::F32(buffer) => SampleRectE::F32(SampleRect::<f32>::from_buffer(buffer)),
            BufferE::I32(buffer) => SampleRectE::I32(SampleRect::<i32>::from_buffer(buffer)),
            BufferE::I16(buffer) => SampleRectE::I16(SampleRect::<i16>::from_buffer(buffer)),
        }
    }

    pub fn width(&self) -> f32 {
        match self {
            SampleRectE::F32(rect) => rect.width(),
            SampleRectE::I32(rect) => rect.width(),
            SampleRectE::I16(rect) => rect.width(),
        }
    }

    pub fn set_ix_rng(&mut self, ix_rng: sample::FracIxRange) {
        match self {
            SampleRectE::F32(rect) => rect.ix_rng = ix_rng,
            SampleRectE::I32(rect) => rect.ix_rng = ix_rng,
            SampleRectE::I16(rect) => rect.ix_rng = ix_rng,
        }
    }

    pub fn ix_rng(&self) -> sample::FracIxRange {
        match self {
            SampleRectE::F32(rect) => rect.ix_rng,
            SampleRectE::I32(rect) => rect.ix_rng,
            SampleRectE::I16(rect) => rect.ix_rng,
        }
    }
}
