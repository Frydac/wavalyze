use crate::audio::{buffer::BufferE, sample};

///
/// A 2D 'camera' view expressed in terms of sample indices and sample values.
///
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct SampleRect {
    /// X range in f64 (for zooming/moving with sub-sample resolution)
    pub ix_rng: sample::FracIxRange,
    /// Y range is always normalized sample value range.
    pub val_rng: Option<sample::ValRange<f64>>,
}

impl SampleRect {
    /// Rectangle contains the whole buffer
    pub fn from_buffere(buffer: &BufferE) -> Self {
        let nr_samples = match buffer {
            BufferE::F32(buffer) => buffer.nr_samples(),
            BufferE::I32(buffer) => buffer.nr_samples(),
            BufferE::I16(buffer) => buffer.nr_samples(),
        };
        Self {
            ix_rng: sample::FracIxRange {
                start: 0.0,
                end: nr_samples as f64,
            },
            val_rng: Some(sample::ValRange {
                min: -1.0,
                max: 1.0,
            }),
        }
    }

    pub fn width(&self) -> f32 {
        self.ix_rng.len() as f32
    }

    pub fn set_ix_rng(&mut self, ix_rng: sample::FracIxRange) {
        self.ix_rng = ix_rng;
    }

    pub fn shift_ix_rng(&mut self, shift: f64) {
        self.ix_rng.start += shift;
    }

    pub fn ix_rng(&self) -> sample::FracIxRange {
        self.ix_rng
    }

    pub fn val_rng(&self) -> Option<sample::ValRange<f64>> {
        self.val_rng
    }

    pub fn set_val_rng(&mut self, val_range: sample::ValRange<f64>) {
        self.val_rng = Some(val_range);
    }
}
