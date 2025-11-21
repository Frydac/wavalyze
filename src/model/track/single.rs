use crate::{
    audio::{
        self, manager::{AudioManager, BufferId}, sample_range2::SampleFractionalIx, sample_rect2::SampleRectE
    },
    model, rect::Rect,
};
use anyhow::{anyhow, Result};

/// Rerpesents a time domain view on an audio buffer
#[derive(Debug, PartialEq, Clone)]
pub struct Single {
    pub view_buffer: model::ViewBufferE,

    pub screen_rect: Option<Rect>,

    // NOTE: maybe more than one Item at some point (in sequence on the 'single' track)
    pub item: Item,
}

impl Single {
    pub fn new(buffer_id: BufferId) -> Result<Self> {
        Ok(Self {
            view_buffer: model::ViewBufferE::SingleSamples(vec![]),
            screen_rect: None,
            item: Item::new(buffer_id)
        })
    }

    pub fn update_sample_view(&mut self, samples_per_pixel: f32, audio: &mut AudioManager, sample_rect: Option<SampleRectE>) -> Result<()> {
        self.item.update_sample_view(samples_per_pixel, audio, sample_rect)
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
            sample_rect : None,
            sample_view: None,
            sample_ix_offset: 0.0,
        }
    }

    pub fn update_sample_view(&mut self, samples_per_pixel: f32, audio: &AudioManager, sample_rect: Option<SampleRectE>) -> Result<()> {
        let buffer = audio
            .buffers
            .get(self.buffer_id)
            .ok_or(anyhow!("Buffer {:?} not found", self.buffer_id))?;

        // take the argument over self
        // if no sample_rect is given, create one over the whole buffer.
        self.sample_rect = sample_rect.or_else(|| self.sample_rect.take().or_else(|| Some(SampleRectE::from_buffere(buffer))));

        let ix_rng = self.sample_rect.as_ref().unwrap().ix_rng();
        let ix_rng = audio::sample::IxRange {
            start: ix_rng.start.ceil() as i64,
            end: ix_rng.end.ceil() as i64,
        };
        let sample_view = audio.get_sample_view_buffer(self.buffer_id, ix_rng, samples_per_pixel)?;
        self.sample_view = Some(sample_view);

        Ok(())
    }
}
