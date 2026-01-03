#![allow(dead_code)]

use crate::{
    audio::{
        buffer2::BufferE,
        sample::{self, range::IxRange},
        thumbnail::ThumbnailE,
    },
    wav::{
        file2::File,
        read::{read_to_file, ReadConfig},
    },
};
use anyhow::{anyhow, Context, Result};
use slotmap::{new_key_type, SecondaryMap, SlotMap};

new_key_type! { pub struct BufferId; }

pub type Buffers = SlotMap<BufferId, BufferE>;
pub type Thumbnails = SecondaryMap<BufferId, ThumbnailE>;

/// Manages audio buffers and their associated thumbnails
#[derive(Debug, Clone, Default)]
pub struct AudioManager {
    pub buffers: Buffers,
    pub thumbnails: Thumbnails,
}

impl AudioManager {
    pub fn load_file(&mut self, read_config: &ReadConfig) -> Result<File> {
        // Read requested buffers and create a File instance to keep references
        let file = read_to_file(read_config, &mut self.buffers)?;

        // Create thumbnails for each buffer
        for (_ch_ix, channel) in file.channels.iter() {
            let buffer = self
                .buffers
                .get(channel.buffer_id)
                .ok_or(anyhow!("Buffer {:?} not found", channel.buffer_id))?;
            let thumbnail = ThumbnailE::from_buffer_e(buffer, None);
            self.thumbnails.insert(channel.buffer_id, thumbnail);
        }

        Ok(file)
    }

    pub fn remove_buffer(&mut self, buffer_id: BufferId) {
        self.buffers.remove(buffer_id);
        self.thumbnails.remove(buffer_id);
    }

    pub fn remove_buffers_from_file(&mut self, file: &File) {
        for (ch_ix, channel) in file.channels.iter() {
            self.remove_buffer(channel.buffer_id);
        }
    }

    pub fn get_buffer(&self, buffer_id: BufferId) -> Result<&BufferE> {
        self.buffers
            .get(buffer_id)
            .with_context(|| format!("Buffer {:?} not found", buffer_id))
    }

    pub fn get_sample_view_buffer(&self, buffer_id: BufferId, sample_ix_range: IxRange, samples_per_pixel: f32) -> Result<sample::View> {
        let buffer = self
            .buffers
            .get(buffer_id)
            .with_context(|| format!("Buffer {:?} not found", buffer_id))?;

        // if samples_per_pixel < 2.0 {

        // } else {

        // }
        todo!()
    }
}
