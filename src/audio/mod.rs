pub mod buffer;
pub mod buffer_pool;
pub mod channel;
// pub mod channel_id;
// pub mod channel_mask;
pub mod buffer2;
pub mod cross_correlation;
pub mod db;
pub mod manager;
pub mod rms;
pub mod sample;
pub mod sample2;
pub mod sample_range;
pub mod sample_range2;
pub mod sample_rect;
pub mod sample_rect2;
pub mod thumbnail;
pub mod util;

// pub type SampleIx = i64;
pub type SampleIx = f64;

// TODO: Is this what we want to do to keep duplication down in the namespace?
use crate::audio;
pub use audio::buffer::{Buffer, BufferBuilder};
// pub use audio::channel::Channel;
pub use audio::buffer_pool::{BufferPool, SampleBuffer};
pub use audio::channel::{Channel, Id, Layout};
pub use audio::manager::BufferId;
pub use audio::sample::{Sample, SampleType};
pub use audio::sample_range::{SampleIxRange, SampleValueRange};
pub use audio::sample_rect::SampleRect;

pub use audio::sample_rect2::SampleRectE;

// pub use audio::sample_range2::SampleIx;
// pub use audio::sample_range2::SampleIxRange;
// pub use audio::sample_range2::SampleFractionalIx
// pub use audio::sample_range2::SampleFractionalIxRange;
