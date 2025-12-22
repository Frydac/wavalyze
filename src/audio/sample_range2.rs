// use crate::audio::sample2::Sample;

// TODO: move to sample:: namespace?

///
/// Inclusive interval of sample values `[min, max]`
///
// pub struct SampleValueRange<T: Sample>(pub std::ops::RangeInclusive<T>);
// #[derive(Debug, Clone, PartialEq, PartialOrd, Copy)]
// pub struct SampleValueRange<T: Sample> {
//     pub min: T,
//     pub max: T,
// }

pub type SampleIx = i64;

/// deprecated!! use sample::range::IxRange
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SampleIxRange(pub std::ops::Range<SampleIx>);

impl std::ops::Deref for SampleIxRange {
    type Target = std::ops::Range<SampleIx>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SampleIxRange {
    pub fn len(&self) -> usize {
        (self.end - self.start) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Fractional index of an audio sample is useful for zooming/moving at sub-sample resolution
pub type SampleFractionalIx = f64;

/// Fractional index range is useful for zooming/moving at sub-sample resolution
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SampleFractionalIxRange(pub std::ops::Range<SampleFractionalIx>);

impl std::ops::Deref for SampleFractionalIxRange {
    type Target = std::ops::Range<SampleFractionalIx>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
