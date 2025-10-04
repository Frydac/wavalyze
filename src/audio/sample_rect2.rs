use crate::audio::{self, sample_range2::SampleValueRange, sample2::Sample};

///
/// Rectangle over a buffer with audio samples
///
#[derive(Debug, PartialEq)]
pub struct SampleRect<T: Sample> {
    /// X range in f64
    pub ix_rng: audio::SampleIxRange,
    /// Y range  in SampleType
    pub val_rng: Option<SampleValueRange<T>>,
}

impl<T: Sample> SampleRect<T> {
    /// Rectangle contains the whole buffer
    pub fn from_buffer(buffer: &audio::buffer2::Buffer<T>) -> Option<Self> {
        let min = buffer.min()?;
        let max = buffer.max()?;
        Self {
            ix_rng: audio::SampleIxRange::new(0.0, buffer.nr_samples() as audio::SampleIx),
            val_rng: Some(SampleValueRange::<T>::new(*min, *max)),
        }
        .into()
    }
}

