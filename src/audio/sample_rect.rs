use crate::audio;

///
/// Rectangle over a buffer with audio samples
///
#[derive(Debug, Clone, Default, Copy, PartialEq)]
pub struct SampleRect {
    /// X range in i64
    pub ix_rng: audio::SampleIxRange,
    /// Y range if f32 (choosing f32 for simplicity, maybe make generic later)
    pub val_rng: audio::SampleValueRange,
}

impl SampleRect {
    /// Rectangle contains the whole buffer
    pub fn from_buffer(buffer: &audio::Buffer<f32>) -> Self {
        Self {
            ix_rng: audio::SampleIxRange::new(0, buffer.nr_samples() as i64),
            val_rng: audio::SampleValueRange::from_buffer(&buffer),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.ix_rng.is_empty()
    }

    pub fn shift_x(&mut self, dx: audio::SampleIx) -> &Self {
        self.ix_rng.shift(dx);
        self
    }

    pub fn zoom_x(&mut self, nr_samples: audio::SampleIx, center: audio::SampleIx) -> &Self {
        self.ix_rng.zoom(nr_samples, center);
        self
    }
}
