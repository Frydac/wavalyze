use crate::audio::sample2::Sample;

// TODO: move to sample:: namespace?

///
/// Inclusive interval of sample values `[min, max]`
///
#[derive(Debug, Clone, PartialEq)]
pub struct SampleValueRange<T: Sample>(pub std::ops::RangeInclusive<T>);

impl<T: Sample> SampleValueRange<T> {
    pub const fn new(min: T, max: T) -> Self {
        Self(min..=max)
    }
    
    pub fn min(&self) -> &T {
        self.0.start()
    }
    
    pub fn max(&self) -> &T {
        self.0.end()
    }
}

// Deref to get all RangeInclusive methods
impl<T: Sample> std::ops::Deref for SampleValueRange<T> {
    type Target = std::ops::RangeInclusive<T>;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

type SampleIx = i64;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SampleIxRange(pub std::ops::Range<SampleIx>);

impl std::ops::Deref for SampleIxRange {
    type Target = std::ops::Range<SampleIx>;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

type SampleFractionalIx = f64;

/// Fractional index range is useful for zooming/moving when less than one sample per pixel column
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SampleFractionalIxRange(pub std::ops::Range<SampleFractionalIx>);

impl std::ops::Deref for SampleFractionalIxRange {
    type Target = std::ops::Range<SampleFractionalIx>;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

