use crate::{
    audio::{
        self,
        manager::{AudioManager, BufferId},
        sample,
        sample_rect2::SampleRect,
    },
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

    /// The data to display
    pub sample_view: Option<audio::sample::View>,

    /// For positioning wrt the absolute zero pos for all tracks
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

    /// Track-local sample rect (with [`Self::sample_ix_offset`] applied).
    pub fn sample_rect(&self) -> Option<SampleRect> {
        self.sample_rect.map(|mut sample_rect| {
            sample_rect.shift_ix_rng(-self.sample_ix_offset);
            sample_rect
        })
    }

    /// The stored sample rect without the ix offset applied. For callers that
    /// only need the value range or width — i.e. don't care about the offset.
    pub fn sample_rect_raw(&self) -> Option<SampleRect> {
        self.sample_rect
    }

    /// Returns true when the stored rect actually changed.
    pub fn set_sample_rect(&mut self, sample_rect: SampleRect) -> bool {
        if self.sample_rect == Some(sample_rect) {
            return false;
        }
        self.sample_rect = Some(sample_rect);
        true
    }

    /// Create or update the sample rect to the given index range. Returns true
    /// when the stored rect actually changed.
    pub fn set_ix_range(
        &mut self,
        ix_range: sample::FracIxRange,
        audio: &AudioManager,
    ) -> Result<bool> {
        let mut new_sample_rect = match self.sample_rect {
            Some(rect) => rect,
            None => {
                let buffer = audio.get_buffer(self.buffer_id)?;
                audio::SampleRect::from_buffere(buffer)
            }
        };
        new_sample_rect.set_ix_rng(ix_range);
        Ok(self.set_sample_rect(new_sample_rect))
    }
}
