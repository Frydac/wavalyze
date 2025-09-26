use crate::audio;

// Accociate a channel id with a buffer
pub struct Channel {
    pub ch_ix: usize,
    pub buffer: audio::buffer_pool::BufferId,
    pub channel_id: Option<audio::Id>,
}

pub struct File {
    pub channels: Vec<Channel>,
    pub sample_type: audio::SampleType,
    pub bit_depth: u16,
    pub sample_rate: u32,
    pub layout: Option<audio::Layout>,
}
