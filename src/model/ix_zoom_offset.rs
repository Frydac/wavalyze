use crate::audio;

/// Sample Index Zoom + Offset
/// Represents a range of audio samples that is visible in a tbd view
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IxZoomOffset {
    /// The zoom level for all tracks
    pub samples_per_pixel: f64,

    /// The sample index offset in order to draw the ruler at the correct position
    pub ix_start: audio::sample::FracIx,
}

impl IxZoomOffset {
    /// The range of samples that is visible in a given pixel width
    pub fn get_ix_range(&self, pixel_width: f64) -> audio::sample::FracIxRange {
        let start = self.ix_start;
        let end = start + pixel_width * self.samples_per_pixel;
        audio::sample::FracIxRange { start, end }
    }
}
