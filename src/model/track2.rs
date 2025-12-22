use crate::{audio, wav};
use anyhow::Result;
use slotmap::new_key_type;

use crate::{
    audio::manager::{AudioManager, BufferId},
    model::{self, hover_info, track::single::Single},
    rect::Rect,
};

new_key_type! { pub struct TrackId; }

/// Represents a view on an audio buffer in the context of a track
/// A track could contain multiple TrackItems
/// TODO: we probaly want an AudioThumbnail soon
/// * also with id and pool
/// * some abstraction managing both buffers and thumbnails
///   * fn get_sample_view_buffer(BufferId, SampleFractionalIxRange, ViewRect) -> ViewBuffer
///     * ViewBuffer is an enum where depending on zoom level we have single samples
///       or min max per pixel. So at most ViewRect.width() nr of elements
///
// #[derive(Debug, PartialEq, Clone)]
// pub struct TrackItem {
//     buffer_id: BufferId,

//     /// Rectangular view over the buffer's samples
//     pub sample_rect: Option<SampleRectE>,
//     pub sample_view: Option<audio::sample::View>,

//     /// For positioning wrt the 'absolute' sample range of the track
//     pub sample_ix_offset: SampleFractionalIx,
// }

// impl TrackItem {
//     pub fn new(buffer_id: BufferId) -> Self {
//         Self {
//             buffer_id,
//             sample_rect: None,
//             sample_view: None,
//             sample_ix_offset: 0.0,
//         }
//     }

//     pub fn update_sample_view(&mut self, samples_per_pixel: f32, audio: &AudioManager, sample_rect: Option<SampleRectE>) -> Result<()> {
//         let buffer = audio
//             .buffers
//             .get(self.buffer_id)
//             .ok_or(anyhow!("Buffer {:?} not found", self.buffer_id))?;

//         // take the argument over self
//         // if no sample_rect is given, create one over the whole buffer.
//         self.sample_rect = sample_rect.or_else(|| self.sample_rect.take().or_else(|| Some(SampleRectE::from_buffere(buffer))));

//         let ix_rng = self.sample_rect.as_ref().unwrap().ix_rng();
//         let ix_rng = audio::sample::IxRange {
//             start: ix_rng.start.ceil() as i64,
//             end: ix_rng.end.ceil() as i64,
//         };
//         let sample_view = audio.get_sample_view_buffer(self.buffer_id, ix_rng, samples_per_pixel)?;
//         self.sample_view = Some(sample_view);

//         Ok(())
//     }
// }

// pub enum SampleViewBuffer<T: Sample> {
//     Single(Buffer<T>),
// }
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

    /// Zoom level in x direction
    ///
    /// * Doesn't change when updating the screen_rect to keep the zoom stable
    pub samples_per_pixel: Option<f32>,

    // x range is pixel width starting at 0.0
    // y range is sample_rect sample range coordinates I think
    // NOTE: not really needed, we can just use screen_rect and 'normalize' it, they should be very similar
    // pub view_rect: Rect,
    /// Contains all the samples as pixel positions relative to top_left (0,0), currently to be
    /// rendered by the track::View
    /// The final transformation to absolute screen coordinates is done in the view::Track
    view_buffer: Option<model::ViewBufferE>,

    /// One item for now
    // track_item: TrackItem,
    pub single: Single,

    pub hover_info: Option<hover_info::HoverInfo>,

    /// Dirty flag for the inputs of the view buffer
    update_view_buffer_: bool,

    track_md: TrackMetaData,
}

impl Track {
    // pub fn new2(track_id: TrackId, buffer_id: BufferId, audio: &mut AudioManager) -> Result<Self> {
    // TODO: probably not pass audio here, we want to initialize possibly with certain existing
    // samples_per_pixel.
    pub fn new2(buffer_id: BufferId) -> Result<Self> {
        let single = Single::new(buffer_id)?;

        Ok(Self {
            screen_rect: None,
            sample_rect: None,
            samples_per_pixel: None,
            view_buffer: None,
            single,
            hover_info: None,
            update_view_buffer_: false,
            track_md: TrackMetaData::None,
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

    pub fn update_view_buffer(&mut self, audio: &mut AudioManager) -> Result<()> {
        if !self.update_view_buffer_ {
            return Ok(());
        }
        self.update_view_buffer_ = false;

        if self.screen_rect.is_none() || self.sample_rect.is_none() {
            return Ok(());
        }

        // self.single

        todo!()
    }
}
