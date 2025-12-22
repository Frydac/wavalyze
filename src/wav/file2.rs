use crate::{
    audio::{self, manager::BufferId},
    wav::read::ChIx,
};
use slotmap::new_key_type;
use std::{collections::HashMap, path::PathBuf};

// Accociate a channel id with a buffer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub ch_ix: ChIx,
    pub buffer_id: BufferId,
    pub channel_id: Option<audio::Id>,
}

pub type Channels = HashMap<ChIx, Channel>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub channels: Channels,
    pub sample_type: audio::SampleType,
    pub bit_depth: u16,
    pub sample_rate: u32,
    pub layout: Option<audio::Layout>,
    pub path: Option<PathBuf>,
    /// Number of samples per channel
    pub nr_samples: u64,
}

impl std::fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "File:")?;
        write!(f, " path: {:?}", self.path)?;
        write!(f, ", nr_channels: {}", self.channels.len())?;
        write!(f, ", sample_type: {:?}", self.sample_type)?;
        write!(f, ", bit_depth: {}", self.bit_depth)?;
        write!(f, ", sample_rate: {}", self.sample_rate)?;
        if let Some(layout) = &self.layout {
            write!(f, ", layout: {:?}", layout)?;
        }
        write!(f, ", nr_samples: {}", self.nr_samples)?;
        Ok(())
    }
}

new_key_type! { pub struct FileId; }
