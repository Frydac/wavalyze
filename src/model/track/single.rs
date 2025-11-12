use crate::{
    audio::{
        manager::{AudioManager, BufferId},
        sample_range2::SampleFractionalIx,
        sample_rect2::SampleRectE,
    },
    model,
};
use anyhow::Result;

/// Rerpesents a time domain view on an audio buffer
#[derive(Debug, PartialEq, Clone)]
pub struct Single {
    view_buffer: model::ViewBuffer,

    // NOTE: maybe more than one Item at some point (in sequence on the 'single' track)
    item: Item,
}

impl Single {
    pub fn new(buffer_id: BufferId, audio: &mut AudioManager) -> Result<Self> {
        let buffer = audio
            .buffers
            .get(buffer_id)
            .ok_or_else(|| anyhow::anyhow!("Buffer {:?} not found", buffer_id))?;
        Ok(Self {
            view_buffer: model::ViewBuffer::SingleSamples(vec![]),
            item: Item {
                buffer_id,
                sample_rect: SampleRectE::from_buffer(buffer),
                sample_ix_offset: 0.0,
            },
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Item {
    pub buffer_id: BufferId,

    /// Rectangular view over the buffer's samples
    pub sample_rect: SampleRectE,

    /// For positioning wrt the 'absolute' sample range of the track
    pub sample_ix_offset: SampleFractionalIx,
}

impl Item {
    pub fn new(buffer_id: BufferId, sample_rect: SampleRectE) -> Self {
        Self {
            buffer_id,
            sample_rect,
            sample_ix_offset: 0.0,
        }
    }
}
