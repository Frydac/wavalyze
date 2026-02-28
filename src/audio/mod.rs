pub mod buffer_pool;
pub mod channel;
// pub mod channel_id;
// pub mod channel_mask;
pub mod buffer;
// pub mod cross_correlation;
pub mod db;
pub mod manager;
pub mod rms;
pub mod sample;
pub mod sample_rect2;
pub mod thumbnail;
pub mod util;

// pub type SampleIx = i64;
pub type SampleIx = f64;

// TODO: Is this what we want to do to keep duplication down in the namespace?
use crate::audio;
pub use audio::buffer_pool::{BufferPool, SampleBuffer};
pub use audio::channel::{Id, Layout};
pub use audio::manager::BufferId;
pub use audio::sample::SampleType;

pub use audio::sample_rect2::SampleRectE;
