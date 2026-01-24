pub mod convert;
pub mod range;
pub mod view;

pub use range::{FracIx, FracIxRange, Ix, IxRange, OptIxRange, ValRange, ValRangeE};
pub use view::View;

// The intention is to allow and audio Sample to be of type f32, i32 or i16
// TODO: Not sure if this accomplishes this?
pub trait Sample: Default + Clone {}
impl Sample for f32 {}
impl Sample for i32 {}
impl Sample for i16 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum SampleType {
    Float,
    Int,
}

impl From<hound::SampleFormat> for SampleType {
    fn from(sample_type: hound::SampleFormat) -> Self {
        match sample_type {
            hound::SampleFormat::Float => SampleType::Float,
            hound::SampleFormat::Int => SampleType::Int,
        }
    }
}
