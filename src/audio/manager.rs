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
use anyhow::{Context, Result};
use slotmap::{new_key_type, SlotMap};

new_key_type! { pub struct BufferId; }

pub type Buffers = SlotMap<BufferId, BufferE>;
pub type Thumbnails = SlotMap<BufferId, ThumbnailE>;

/// Manages audio buffers and their associated thumbnails
#[derive(Debug, Clone, Default)]
pub struct AudioManager {
    pub buffers: Buffers,
    pub thumbnails: Thumbnails,
}

impl AudioManager {
    pub fn load_file(&mut self, read_config: ReadConfig) -> Result<File> {
        read_to_file(read_config, &mut self.buffers)
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
