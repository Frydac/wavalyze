use crate::{
    audio::{self, sample},
    model::config::TrackConfig,
    wav,
};
use anyhow::{Result, anyhow};
use slotmap::new_key_type;

use crate::{
    audio::manager::{AudioManager, BufferId},
    model::{self, track::hover_info, track::single::Single},
    rect::Rect,
};

new_key_type! { pub struct TrackId; }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackMetaData {
    File(wav::file2::File, wav::ChIx),
    None,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Track {
    // pub id: Option<TrackId>,
    /// Id of this track in storage, mabye better Optional, lets try without option for now
    // pub track_id: TrackId,

    /// The pixel rectangle in absolute screen coordinates for the track
    /// Is updated by/for the view when displayed
    pub screen_rect: Option<Rect>,
    pub sample_rect: Option<audio::SampleRectE>,

    // x range is pixel width starting at 0.0
    // y range is sample_rect sample range coordinates I think
    // NOTE: not really needed, we can just use screen_rect and 'normalize' it, they should be very similar
    // pub view_rect: Rect,
    // Contains all the samples as pixel positions relative to top_left (0,0), currently to be
    // rendered by the track::View
    // The final transformation to absolute screen coordinates is done in the view::Track
    view_buffer: Option<model::ViewBufferE>,

    /// One item for now
    // track_item: TrackItem,
    pub single: Single,

    pub hover_info: Option<hover_info::HoverInfo>,

    /// Dirty flag for the inputs of the view buffer
    update_view_buffer_: bool,

    track_md: TrackMetaData,

    pub height: f32,
}

impl Track {
    // pub fn new2(track_id: TrackId, buffer_id: BufferId, audio: &mut AudioManager) -> Result<Self> {
    // TODO: probably not pass audio here, we want to initialize possibly with certain existing
    // samples_per_pixel.
    pub fn new2(buffer_id: BufferId, track_config: &TrackConfig) -> Result<Self> {
        let single = Single::new(buffer_id)?;

        Ok(Self {
            screen_rect: None,
            sample_rect: None,
            // samples_per_pixel: None,
            view_buffer: None,
            single,
            hover_info: None,
            update_view_buffer_: false,
            track_md: TrackMetaData::None,
            height: track_config.min_height,
        })

        // todo!()
    }
}

impl Track {
    pub fn set_screen_rect(&mut self, screen_rect: Rect) {
        if self.screen_rect != Some(screen_rect) {
            self.update_view_buffer_ = true;
            self.screen_rect = Some(screen_rect);
        }
    }

    pub fn set_sample_rect(&mut self, sample_rect: audio::SampleRectE) {
        if self.sample_rect != Some(sample_rect) {
            self.update_view_buffer_ = true;
            self.sample_rect = Some(sample_rect);
        }
    }

    /// Create or update the sample rect to the given range
    /// TODO: we could do this by only knowing the sample_type/bit_depth, iso depending on AudioManager?
    pub fn set_ix_range(
        &mut self,
        ix_range: audio::sample::FracIxRange,
        audio: &AudioManager,
    ) -> Result<()> {
        if let Some(sample_rect) = self.sample_rect {
            let mut new_sample_rect = sample_rect;
            new_sample_rect.set_ix_rng(ix_range);
            self.set_sample_rect(new_sample_rect);
        } else {
            let buffer = audio.get_buffer(self.single.item.buffer_id)?;
            let mut sample_rect = audio::SampleRectE::from_buffere(buffer);
            sample_rect.set_ix_rng(ix_range);
            self.set_sample_rect(sample_rect);
        }
        Ok(())
    }

    pub fn update_sample_view(&mut self, audio: &mut AudioManager) -> Result<()> {
        if !self.update_view_buffer_ {
            return Ok(());
        }
        self.update_view_buffer_ = false;

        let screen_rect = self
            .screen_rect
            .ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
        let sample_rect = self
            .sample_rect
            .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
        let buffer_id = self.single.item.buffer_id;

        self.single.item.sample_view =
            Some(audio.get_sample_view(buffer_id, sample_rect, screen_rect)?);

        // trace!("self.single.item.sample_view: {:?}", self.single.item.sample_view);

        Ok(())
    }

    pub fn get_sample_view(&self) -> Result<&sample::View> {
        self.single
            .item
            .sample_view
            .as_ref()
            .ok_or(anyhow!("sample_view is missing"))
    }

    // pub fn pos_y_sample_value<T: Sample>(&self, value: T) -> Option<f32> {
    //     todo!()
    // }
}
