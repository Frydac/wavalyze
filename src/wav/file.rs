//! Metadata and channel-to-buffer mappings for audio already registered in the model.
//!
//! A file keeps its identity while reload replaces its buffers. Its optional source recipe
//! distinguishes reloadable disk files from browser uploads and generated audio, whose names
//! may look like paths but do not grant access to a filesystem source.

use crate::{
    audio::{self, manager::BufferId, sample},
    wav::read::ChIx,
};
use slotmap::new_key_type;
use std::{collections::BTreeMap, path::PathBuf};

/// Connect a WAV channel index and optional speaker identity to its currently loaded audio buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    pub ch_ix: ChIx,
    pub buffer_id: BufferId,
    pub channel_id: Option<audio::Id>,
}

pub type Channels = BTreeMap<ChIx, Channel>;

/// Registered file metadata and its loaded channel buffers, independent of displayed tracks.
/// Reload updates this record in place so file identity and user-controlled offsets survive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// Retained reload recipe, absent for browser bytes and generated audio.
    pub source: Option<Box<crate::wav::ReadConfig>>,
    /// Loaded channel buffers, which can be a subset of the WAV's channels.
    pub channels: Channels,
    /// Total channel count from the WAV header, including channels that were not loaded.
    pub total_nr_channels: usize,
    pub sample_type: audio::SampleType,
    pub bit_depth: u16,
    pub sample_rate: u32,
    pub layout: Option<audio::Layout>,
    pub path: Option<PathBuf>,
    /// Number of samples per channel
    pub nr_samples: u64,
    pub sample_ix_offset: sample::Ix,
}

impl std::fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "File:")?;
        write!(f, " path: {:?}", self.path)?;
        write!(
            f,
            ", nr_channels: {}/{}",
            self.channels.len(),
            self.total_nr_channels
        )?;
        write!(f, ", sample_type: {:?}", self.sample_type)?;
        write!(f, ", bit_depth: {}", self.bit_depth)?;
        write!(f, ", sample_rate: {}", self.sample_rate)?;
        if let Some(layout) = &self.layout {
            write!(f, ", layout: {:?}", layout)?;
        }
        write!(f, ", nr_samples: {}", self.nr_samples)?;
        write!(f, ", sample_ix_offset: {}", self.sample_ix_offset)?;
        Ok(())
    }
}

new_key_type! { pub struct FileId; }

impl File {
    pub fn get_channel(&self, buffer_id: BufferId) -> Option<&Channel> {
        self.channels
            .iter()
            .find(|(_, channel)| channel.buffer_id == buffer_id)
            .map(|(_, channel)| channel)
    }
}
