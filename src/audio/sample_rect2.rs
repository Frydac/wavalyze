use crate::audio::{self, buffer2::BufferE, sample, sample2::Sample};
use anyhow::{Result, anyhow};

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

    pub fn shift_ix_rng(&mut self, shift: f64) {
        match self {
            SampleRectE::F32(rect) => rect.ix_rng.start += shift,
            SampleRectE::I32(rect) => rect.ix_rng.start += shift,
            SampleRectE::I16(rect) => rect.ix_rng.start += shift,
        }
    }

    pub fn ix_rng(&self) -> sample::FracIxRange {
        match self {
            SampleRectE::F32(rect) => rect.ix_rng,
            SampleRectE::I32(rect) => rect.ix_rng,
            SampleRectE::I16(rect) => rect.ix_rng,
        }
    }

    pub fn val_rng(&self) -> Option<sample::ValRangeE> {
        match self {
            SampleRectE::F32(rect) => rect.val_rng.map(sample::ValRangeE::F32),
            SampleRectE::I32(rect) => rect.val_rng.map(sample::ValRangeE::PCM24),
            SampleRectE::I16(rect) => rect.val_rng.map(sample::ValRangeE::PCM16),
        }
    }

    // pub fn val_rng<T: Sample>(&self) -> Option<sample::ValRange<T>> {
    // match self {
    //     SampleRectE::F32(sample_rect) => sample_rect.get_f32().ok().map(|rect| rect.val_rng),
    //     SampleRectE::I32(sample_rect) => sample_rect.val_rng,
    //     SampleRectE::I16(sample_rect) => sample_rect.val_rng,
    // }
    // }

    pub fn get_f32(&self) -> Result<&SampleRect<f32>> {
        match self {
            SampleRectE::F32(rect) => Ok(rect),
            _ => Err(anyhow!("Not a f32 rect")),
        }
    }

    pub fn get_i32(&self) -> Result<&SampleRect<i32>> {
        match self {
            SampleRectE::I32(rect) => Ok(rect),
            _ => Err(anyhow!("Not a i32 rect")),
        }
    }

    pub fn get_i16(&self) -> Result<&SampleRect<i16>> {
        match self {
            SampleRectE::I16(rect) => Ok(rect),
            _ => Err(anyhow!("Not a i16 rect")),
        }
    }
}
