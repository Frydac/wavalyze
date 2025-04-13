pub mod buffer;
pub mod channel;
pub mod cross_correlation;
pub mod db;
pub mod rms;
pub mod sample;
pub mod sample_range;
pub mod sample_rect;
pub mod util;

pub type SampleIx = i64;

// TODO: Is this what we want to do to keep duplication down in the namespace?
use crate::audio;
pub use audio::buffer::{Buffer, BufferBuilder};
pub use audio::channel::Channel;
pub use audio::sample::{Sample, SampleType};
pub use audio::sample_range::{SampleIxRange, SampleValueRange};
pub use audio::sample_rect::SampleRect;
