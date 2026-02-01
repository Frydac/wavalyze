#![allow(dead_code)]

use crate::{
    audio::{
        SampleRectE,
        buffer2::BufferE,
        sample::{self},
        thumbnail::ThumbnailE,
    },
    rect::Rect,
    wav::{
        file2::File,
        read::{ReadConfig, read_to_file},
    },
};
use anyhow::{Context, Result, anyhow};
use rayon::prelude::*;
use slotmap::{SecondaryMap, SlotMap, new_key_type};

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
        dbg!(&file);

        // Create thumbnails for each buffer in parallel
        let results: Result<Vec<(BufferId, ThumbnailE)>, anyhow::Error> = file
            .channels
            .par_iter()
            .map(|(_ch_ix, channel)| {
                let buffer = self
                    .buffers
                    .get(channel.buffer_id)
                    .ok_or_else(|| anyhow!("Buffer {:?} not found", channel.buffer_id))?;

                let thumbnail = ThumbnailE::from_buffer_e(buffer, None);
                Ok((channel.buffer_id, thumbnail))
            })
            .collect();

        // Propagate any error from the parallel computation
        for (buffer_id, thumbnail) in results? {
            self.thumbnails.insert(buffer_id, thumbnail);
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

    pub fn get_sample_view(&self, buffer_id: BufferId, sample_rect: SampleRectE, screen_rect: Rect) -> Result<sample::View> {
        let target_spp = sample_rect.width() / screen_rect.width();
        let thumbnail = self.thumbnails.get(buffer_id);
        let thumbnail_spp = thumbnail
            .and_then(|thumbnail| thumbnail.get_smallest_samples_per_pixel())
            .map(|spp| spp as f32);
        if thumbnail_spp.is_none() || thumbnail_spp.unwrap() > target_spp {
            let buffere = self.get_buffer(buffer_id)?;
            sample::View::from_buffere(buffere, sample_rect, screen_rect)
        } else {
            let thumbnail = thumbnail.unwrap();
            let level_data = thumbnail.get_level_data(target_spp).ok_or(anyhow!("level_data not found"))?;
            sample::View::from_level_data_e(&level_data, sample_rect, screen_rect)
        }
    }
}
