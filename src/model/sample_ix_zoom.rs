use crate::audio;

/// Sample Index Zoom + Offset
/// Represents a range of audio samples that is visible in a tbd view
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SampleIxZoom {
    /// The zoom level for all tracks
    samples_per_pixel: f64,

    /// The sample index offset in order to draw the ruler at the correct position
    pub ix_start: audio::sample::FracIx,
}

const MIN_SAMPLES_PER_PIXEL: f64 = 0.01;

impl SampleIxZoom {
    /// The range of samples that is visible in a given pixel width
    pub fn get_ix_range(&self, pixel_width: f64) -> audio::sample::FracIxRange {
        assert!(pixel_width >= 0.0, "pixel_width must be >= 0");
        let start = self.ix_start;
        let end = start + pixel_width * self.samples_per_pixel;
        audio::sample::FracIxRange { start, end }
    }

    pub fn set_samples_per_pixel(&mut self, samples_per_pixel: f64) {
        self.samples_per_pixel = samples_per_pixel.max(MIN_SAMPLES_PER_PIXEL);
    }

    pub fn samples_per_pixel(&self) -> f64 {
        self.samples_per_pixel
    }
}
