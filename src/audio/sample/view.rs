use crate::audio::{sample::ValRange, sample2::Sample};
use std::fmt;

pub type Samples<T> = Vec<T>;
pub type MinMaxSamples<T> = Vec<ValRange<T>>;

/// Depending on the zoom level, we want to render either a single sample, or a min/max range for
/// each pixel column.
#[derive(Debug, PartialEq, Clone)]
pub enum ViewData<T: Sample> {
    Single(Samples<T>),
    MinMax(MinMaxSamples<T>),
}

impl<T: Sample> fmt::Display for ViewData<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ViewData::Single(samples) => {
                write!(f, "Single(count = {})", samples.len())
            }
            ViewData::MinMax(ranges) => {
                write!(f, "MinMax(count = {})", ranges.len())
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct View2<T: Sample> {
    pub data: ViewData<T>,
    /// ix 0 in data corresponds to sample_ix_offset
    pub sample_ix_offset: i64,
    pub samples_per_pixel: f32,
    pub sample_rate: u32,
    pub bit_depth: u32,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ViewE {
    I16(View2<i16>),
    I32(View2<i32>),
    F32(View2<f32>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum ViewDataE {
    I16(ViewData<i16>),
    I32(ViewData<i32>),
    F32(ViewData<f32>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct View {
    pub data: ViewDataE,
    pub sample_rate: u32,
    pub bit_depth: u32,
    pub samples_per_pixel: f32,
}

/// When in a generic context, this can be used to create a ViewDataE
pub trait IntoViewDataE: Sample + Sized {
    fn single(vec: Samples<Self>) -> ViewDataE;
    fn minmax(vec: MinMaxSamples<Self>) -> ViewDataE;
}

impl IntoViewDataE for i16 {
    fn single(vec: Samples<Self>) -> ViewDataE {
        ViewDataE::I16(ViewData::Single(vec))
    }
    fn minmax(vec: MinMaxSamples<Self>) -> ViewDataE {
        ViewDataE::I16(ViewData::MinMax(vec))
    }
}

impl IntoViewDataE for i32 {
    fn single(vec: Samples<Self>) -> ViewDataE {
        ViewDataE::I32(ViewData::Single(vec))
    }
    fn minmax(vec: MinMaxSamples<Self>) -> ViewDataE {
        ViewDataE::I32(ViewData::MinMax(vec))
    }
}

impl IntoViewDataE for f32 {
    fn single(vec: Samples<Self>) -> ViewDataE {
        ViewDataE::F32(ViewData::Single(vec))
    }
    fn minmax(vec: MinMaxSamples<Self>) -> ViewDataE {
        ViewDataE::F32(ViewData::MinMax(vec))
    }
}
