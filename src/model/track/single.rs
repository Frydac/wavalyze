use crate::{
    audio::{
        self,
        manager::{AudioManager, BufferId},
        sample,
        sample_rect2::SampleRect,
    },
    model::ruler::ValueDisplayScale,
    rect::Rect,
};
use anyhow::Result;

/// Represents a time domain view on an audio buffer
#[derive(Debug, PartialEq, Clone)]
pub struct Single {
    screen_rect: Option<Rect>,

    pub buffer_id: BufferId,

    /// Rectangular view over the buffer's samples
    sample_rect: Option<SampleRect>,

    /// The data to display
    pub sample_view: Option<sample::View>,

    /// Display scale that was used to compute `sample_view`.
    sample_view_scale: ValueDisplayScale,

    /// Set by any input setter when its value actually changes. Cleared by
    /// [`Self::update`] after recomputing the sample view.
    dirty: bool,

    /// Signed sample offset used to map absolute sample indices to this buffer's local indices.
    /// A sample at absolute index `n` reads local index `n + sample_ix_offset`.
    pub sample_ix_offset: f64,
}

impl Single {
    pub fn new(buffer_id: BufferId) -> Result<Self> {
        Ok(Self {
            screen_rect: None,
            buffer_id,
            sample_rect: None,
            sample_view: None,
            sample_view_scale: ValueDisplayScale::default(),
            dirty: false,
            sample_ix_offset: 0.0,
        })
    }

    /// Track-local sample rect (with [`Self::sample_ix_offset`] applied).
    pub fn sample_rect(&self) -> Option<SampleRect> {
        self.sample_rect.map(|mut sample_rect| {
            // Convert the absolute timeline window into this buffer's local sample indices.
            sample_rect.shift_ix_rng(self.sample_ix_offset);
            sample_rect
        })
    }

    /// The stored sample rect without the ix offset applied. For callers that
    /// only need the value range or width — i.e. don't care about the offset.
    pub fn sample_rect_raw(&self) -> Option<SampleRect> {
        self.sample_rect
    }

    pub fn screen_rect(&self) -> Option<Rect> {
        self.screen_rect
    }

    pub fn set_screen_rect(&mut self, screen_rect: Rect) {
        if self.screen_rect != Some(screen_rect) {
            self.screen_rect = Some(screen_rect);
            self.dirty = true;
        }
    }

    pub fn set_sample_rect(&mut self, sample_rect: SampleRect) {
        if self.sample_rect != Some(sample_rect) {
            self.sample_rect = Some(sample_rect);
            self.dirty = true;
        }
    }

    /// Create or update the sample rect to the given index range.
    pub fn set_ix_range(
        &mut self,
        ix_range: sample::FracIxRange,
        audio: &AudioManager,
    ) -> Result<()> {
        let mut new_sample_rect = match self.sample_rect {
            Some(rect) => rect,
            None => {
                let buffer = audio.get_buffer(self.buffer_id)?;
                audio::SampleRect::from_buffere(buffer)
            }
        };
        new_sample_rect.set_ix_rng(ix_range);
        self.set_sample_rect(new_sample_rect);
        Ok(())
    }

    pub fn set_display_scale(&mut self, scale: ValueDisplayScale) {
        if self.sample_view_scale != scale {
            self.sample_view_scale = scale;
            self.dirty = true;
        }
    }

    /// Flag the sample view as needing a recompute on the next [`Self::update`].
    /// Used by callers that mutate a `pub` field directly (e.g. `sample_ix_offset`).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Recompute [`Self::sample_view`] if any input has changed since the last
    /// call. Cheap no-op when nothing changed.
    pub fn update(&mut self, audio: &mut AudioManager) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }
        let screen_rect = self
            .screen_rect
            .ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
        let sample_rect = self
            .sample_rect()
            .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
        self.sample_view = Some(audio.get_sample_view(
            self.buffer_id,
            sample_rect,
            screen_rect,
            self.sample_view_scale,
        )?);
        self.dirty = false;
        Ok(())
    }

    pub fn get_sample_view(&self) -> Result<&sample::View> {
        self.sample_view
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("sample_view is missing"))
    }
}
