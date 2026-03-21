use crate::{
    audio::{self, sample},
    model::{config::TrackConfig, ruler::ValueDisplayScale},
    wav,
};
use anyhow::{Result, anyhow};
use slotmap::new_key_type;

#[path = "track/single.rs"]
pub mod single;

use crate::{
    audio::manager::{AudioManager, BufferId},
    model::{self},
    rect::Rect,
};
use single::Single;

new_key_type! { pub struct TrackId; }
pub const HEADER_HEIGHT: f32 = 22.0;

pub fn min_total_height(track_config: &TrackConfig) -> f32 {
    track_config.min_height + HEADER_HEIGHT
}

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
    pub sample_rect: Option<audio::SampleRect>,

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

    /// Dirty flag for the inputs of the view buffer
    update_view_buffer_: bool,
    sample_view_scale: ValueDisplayScale,

    track_md: TrackMetaData,

    pub height: f32,
    pub visible: bool,
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
            update_view_buffer_: false,
            sample_view_scale: ValueDisplayScale::default(),
            track_md: TrackMetaData::None,
            height: min_total_height(track_config),
            visible: true,
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

    pub fn set_sample_rect(&mut self, sample_rect: audio::SampleRect) {
        if self.sample_rect != Some(sample_rect) {
            self.update_view_buffer_ = true;
            self.sample_rect = Some(sample_rect);
            self.single.item.set_sample_rect(sample_rect);
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
            let mut sample_rect = audio::SampleRect::from_buffere(buffer);
            sample_rect.set_ix_rng(ix_range);
            self.set_sample_rect(sample_rect);
        }
        Ok(())
    }

    pub fn update_sample_view(
        &mut self,
        audio: &mut AudioManager,
        display_scale: ValueDisplayScale,
    ) -> Result<()> {
        if self.sample_view_scale != display_scale {
            self.update_view_buffer_ = true;
        }
        if !self.update_view_buffer_ {
            return Ok(());
        }
        self.update_view_buffer_ = false;

        let screen_rect = self
            .screen_rect
            .ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
        let sample_rect = self
            .single
            .item
            .sample_rect()
            .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
        let buffer_id = self.single.item.buffer_id;

        self.single.item.sample_view =
            Some(audio.get_sample_view(buffer_id, sample_rect, screen_rect, display_scale)?);
        self.sample_view_scale = display_scale;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{
        buffer::{Buffer, BufferE},
        sample::view::ViewData,
    };
    use crate::model::config::TrackConfig;

    fn insert_buffer(audio: &mut AudioManager, nr_samples: usize) -> BufferId {
        let mut buffer = Buffer::new(48_000, 32);
        buffer.data = (0..nr_samples)
            .map(|i| i as f32 / nr_samples as f32)
            .collect();
        let buffere = BufferE::F32(buffer);
        let buffer_id = audio.buffers.insert(buffere.clone());
        audio.thumbnails.insert(
            buffer_id,
            audio::thumbnail::ThumbnailE::from_buffer_e(&buffere, None),
        );
        buffer_id
    }

    #[test]
    fn update_sample_view_replaces_stale_waveform_with_empty_view() {
        let mut audio = AudioManager::default();
        let buffer_id = insert_buffer(&mut audio, 32);
        let mut track = Track::new2(buffer_id, &TrackConfig::default()).unwrap();
        track.set_screen_rect(Rect::new(0.0, 0.0, 16.0, 40.0));

        track.set_ix_range((0.0..16.0).into(), &audio).unwrap();
        track
            .update_sample_view(&mut audio, ValueDisplayScale::default())
            .unwrap();

        let initial_view = track.get_sample_view().unwrap();
        assert!(
            matches!(initial_view.data, ViewData::Single(ref data) if !data.samples.is_empty())
        );

        track.set_ix_range((64.0..96.0).into(), &audio).unwrap();
        track
            .update_sample_view(&mut audio, ValueDisplayScale::default())
            .unwrap();

        let updated_view = track.get_sample_view().unwrap();
        assert_eq!(updated_view.data, ViewData::MinMax(vec![]));
    }
}
