use crate::{
    audio::{self, manager::BufferId},
    wav::read::ChIx,
};
use slotmap::new_key_type;
use std::{collections::HashMap, path::PathBuf};

// Accociate a channel id with a buffer
#[derive(Debug, Clone)]
pub struct Channel {
    pub ch_ix: usize,
    pub buffer_id: BufferId,
    pub channel_id: Option<audio::Id>,
}

pub type Channels = HashMap<ChIx, Channel>;

#[derive(Debug, Clone)]
pub struct File {
    pub channels: Channels,
    pub sample_type: audio::SampleType,
    pub bit_depth: u16,
    pub sample_rate: u32,
    pub layout: Option<audio::Layout>,
    // pub path: Option<PathBuf>,
}

new_key_type! { pub struct FileId; }
