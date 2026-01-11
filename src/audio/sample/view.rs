use crate::{
    audio::{sample::ValRange, sample2::Sample},
    Pos,
};
use std::fmt;

pub type Samples<T> = Vec<T>;
pub type MinMaxSamples<T> = Vec<ValRange<T>>;

/// Depending on the zoom level, we want to render either a single sample, or a min/max range for
/// each pixel column.
#[derive(Debug, PartialEq, Clone)]
pub enum ViewDataOld<T: Sample> {
    Single(Samples<T>),
    MinMax(MinMaxSamples<T>),
}

impl<T: Sample> fmt::Display for ViewDataOld<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewDataOld::Single(samples) => {
                write!(f, "Single(count = {})", samples.len())
            }
            ViewDataOld::MinMax(ranges) => {
                write!(f, "MinMax(count = {})", ranges.len())
            }
        }
    }
}

/// Represents a pixel column defined by 2 positions with the same x coordinate.
#[derive(Debug, PartialEq, Clone)]
pub struct MinMaxPos {
    pub min: Pos,
    pub max: Pos,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ViewData {
    Single(Vec<Pos>),
    MinMax(Vec<MinMaxPos>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct View {
    pub data: ViewData,
    pub sample_rate: u32,
    pub bit_depth: u32,
    pub samples_per_pixel: f32,
}
