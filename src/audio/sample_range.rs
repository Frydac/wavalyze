use super::SampleIx;
use crate::audio;

///
/// Half open interval of sample indices `[start, end)`
///
#[derive(Debug, Clone, Default, Copy, PartialEq)]
pub struct SampleIxRange {
    start: SampleIx,
    end: SampleIx,
}

impl SampleIxRange {
    pub fn new(start: SampleIx, end: SampleIx) -> Self {
        assert!(start <= end);
        Self { start, end }
    }

    // Adjusts the end to match the new width, the start is not changed
    pub fn resize(&mut self, width: SampleIx) -> &Self {
        self.end = self.start + width;
        self
    }

    pub fn len(&self) -> SampleIx {
        (self.end - self.start).abs()
    }

    // Lenght of the positive part of the range, i.e. when start and end are negative, this len is
    // 0.
    pub fn positive_len(&self) -> usize {
        let end = self.end.max(0.0);
        let start = self.start.max(0.0);
        (end - start) as usize
    }

    pub fn first(&self) -> SampleIx {
        self.start
    }

    pub fn last(&self) -> SampleIx {
        self.end - 1.0
    }

    pub fn start(&self) -> SampleIx {
        self.start
    }

    pub fn end(&self) -> SampleIx {
        self.end
    }

    pub fn width(&self) -> SampleIx {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0.0
    }

    pub fn contains(&self, ix: SampleIx) -> bool {
        ix >= self.start && ix < self.end
    }

    pub fn shift(&mut self, offset: SampleIx) {
        self.start += offset;
        self.end += offset;
    }

    pub fn include(&mut self, ix: SampleIx) {
        if ix < self.start {
            self.start = ix;
        }
        if ix >= self.end {
            self.end = ix + 1.0;
        }
    }

    pub fn include_range(&mut self, range: Self) {
        self.include(range.start);
        self.include(range.end);
    }

    pub fn with_shift(&self, offset: SampleIx) -> Self {
        Self::new(self.start + offset, self.end + offset)
    }

    /// Zoom nr of samples around center_ix
    pub fn zoom(&mut self, nr_of_samples: SampleIx, center_ix: SampleIx) {
        if self.is_empty() {
            return;
        }
        let center_ix = center_ix.clamp(self.start, self.end);
        let nr_samples_to_center = center_ix - self.start;
        let ratio_start_to_center = nr_samples_to_center as f32 / self.len() as f32;
        // let shift_start = (ratio_start_to_center * nr_of_samples as f32).round() as SampleIx;
        let shift_start = (ratio_start_to_center * nr_of_samples as f32) as SampleIx;
        let shift_end = nr_of_samples - shift_start;
        self.start -= shift_start;
        self.end += shift_end;
        // Make sure we keep the invariant, it can happen I think due to float rounding
        if self.start > self.end {
            self.start = self.end;
        }
    }
}

///
/// Inclusive interval of sample values `[min, max]`
///
#[derive(Debug, Clone, Default, Copy, PartialEq)]
pub struct SampleValueRange {
    pub min: f32,
    pub max: f32,
}

impl SampleValueRange {
    pub fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    pub fn from_sample_type(sample_type: audio::SampleType, bit_depth: u32) -> Self {
        match sample_type {
            audio::SampleType::Float => Self { min: -1.0, max: 1.0 },
            audio::SampleType::Int => {
                assert!(bit_depth <= 24);
                let max = 1i32 << (bit_depth - 1);
                Self {
                    min: -max as f32,
                    max: (max - 1) as f32,
                }
            }
        }
    }

    pub fn from_buffer(buffer: &audio::Buffer<f32>) -> Self {
        Self::from_sample_type(buffer.sample_type, buffer.bit_depth)
    }
}
