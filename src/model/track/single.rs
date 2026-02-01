use crate::{
    audio::{
        self,
        manager::{AudioManager, BufferId},
        sample_range2::SampleFractionalIx,
        sample_rect2::SampleRectE,
    },
    rect::Rect,
};
use anyhow::{Result, anyhow};

/// Rerpesents a time domain view on an audio buffer
#[derive(Debug, PartialEq, Clone)]
pub struct Single {
    pub screen_rect: Option<Rect>,

    // NOTE: maybe more than one Item at some point (in sequence on the 'single' track)
    pub item: Item,
}

impl Single {
    pub fn new(buffer_id: BufferId) -> Result<Self> {
        Ok(Self {
            screen_rect: None,
            item: Item::new(buffer_id),
        })
    }

    pub fn update_sample_view(&mut self, samples_per_pixel: f32, audio: &mut AudioManager, sample_rect: Option<SampleRectE>) -> Result<()> {
        if let (Some(sample_rect), Some(screen_rect)) = (sample_rect, self.screen_rect) {
            self.item.update_sample_view(samples_per_pixel, audio, &sample_rect, &screen_rect)?;
        }
        Ok(())
    }

    // probably better to have like Track set_screen_rect and set_sample_rect. both updating the
    // view buffer if needed, or maybe have a separate commit function for that
    // we generally first handle interactions which adjusts the sample_rect (zoom, pan)
}

#[derive(Debug, PartialEq, Clone)]
pub struct Item {
    pub buffer_id: BufferId,

    /// Rectangular view over the buffer's samples
    pub sample_rect: Option<SampleRectE>,
    /// The data to display but still in 'sample' coordinates
    pub sample_view: Option<audio::sample::View>,

    /// For positioning wrt the 'absolute' sample range of the track
    pub sample_ix_offset: SampleFractionalIx,
}

impl Item {
    pub fn new(buffer_id: BufferId) -> Self {
        Self {
            buffer_id,
            sample_rect: None,
            sample_view: None,
            sample_ix_offset: 0.0,
        }
    }

    pub fn update_sample_view(
        &mut self,
        samples_per_pixel: f32,
        audio: &AudioManager,
        sample_rect: &SampleRectE,
        screen_rect: &Rect,
    ) -> Result<()> {
        let buffer = audio
            .buffers
            .get(self.buffer_id)
            .ok_or(anyhow!("Buffer {:?} not found", self.buffer_id))?;

        let sample_view = audio.get_sample_view(self.buffer_id, *sample_rect, *screen_rect)?;
        self.sample_view = Some(sample_view);

        self.sample_rect = Some(*sample_rect);

        Ok(())
    }
}
