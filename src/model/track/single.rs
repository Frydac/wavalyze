use crate::{
    audio::{self, manager::BufferId, sample_rect2::SampleRect},
    rect::Rect,
};
use anyhow::Result;

/// Rerpesents a time domain view on an audio buffer
#[derive(Debug, PartialEq, Clone)]
pub struct Single {
    pub screen_rect: Option<Rect>,

    pub buffer_id: BufferId,

    /// Rectangular view over the buffer's samples
    sample_rect: Option<SampleRect>,
    /// The data to display but still in 'sample' coordinates
    pub sample_view: Option<audio::sample::View>,

    /// For positioning wrt the 'absolute' sample range of the track
    pub sample_ix_offset: f64,
}

impl Single {
    pub fn new(buffer_id: BufferId) -> Result<Self> {
        Ok(Self {
            screen_rect: None,
            buffer_id,
            sample_rect: None,
            sample_view: None,
            sample_ix_offset: 0.0,
        })
    }

    pub fn sample_rect(&self) -> Option<SampleRect> {
        self.sample_rect.map(|mut sample_rect| {
            sample_rect.shift_ix_rng(-self.sample_ix_offset);
            sample_rect
        })
    }

    pub fn set_sample_rect(&mut self, sample_rect: SampleRect) {
        self.sample_rect = Some(sample_rect);
    }
}
