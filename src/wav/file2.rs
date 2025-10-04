use crate::{audio, wav::read::ChIx};
use slotmap::new_key_type;
use std::collections::HashMap;

// Accociate a channel id with a buffer
#[derive(Debug)]
pub struct Channel {
    pub ch_ix: usize,
    pub buffer_id: audio::buffer_pool::BufferId,
    pub channel_id: Option<audio::Id>,
}

pub type Channels = HashMap<ChIx, Channel>;

#[derive(Debug)]
pub struct File {
    pub channels: Channels,
    pub sample_type: audio::SampleType,
    pub bit_depth: u16,
    pub sample_rate: u32,
    pub layout: Option<audio::Layout>,
}


new_key_type! { pub struct FileId; }

