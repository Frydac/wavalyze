use crate::audio;
use anyhow::Result;
use slotmap::new_key_type;

use crate::{
    audio::{
        manager::{AudioManager, BufferId},
        sample_range2::SampleFractionalIx,
        sample_rect2::SampleRectE,
        BufferPool,
    },
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
#[derive(Debug, PartialEq, Clone)]
pub struct TrackItem {
    buffer_id: BufferId,

    /// Rectangular view over the buffer's samples
    pub sample_rect: SampleRectE,
    pub sample_view: audio::sample::View,

    /// For positioning wrt the 'absolute' sample range of the track
    pub sample_ix_offset: SampleFractionalIx,
}

impl TrackItem {
    // create sample_rect/view that encompasses the whole buffer
    pub fn new(buffer_id: BufferId) -> Self {
        // Self {
        //     buffer_id,
        //     sample_rect,
        //     sample_ix_offset: 0.0,
        // }
        todo!()
    }
}

// pub enum SampleViewBuffer<T: Sample> {
//     Single(Buffer<T>),
// }

#[derive(Debug, PartialEq, Clone)]
pub struct Track {
    // pub id: Option<TrackId>,
    /// Id of this track in storage, mabye better Optional, lets try without option for now
    pub track_id: TrackId,

    /// The pixel rectangle in absolute screen coordinates for the track
    /// Is updated by/for the view when displayed
    pub screen_rect: Option<Rect>,

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
    view_buffer: Option<model::ViewBuffer>,

    /// One item for now
    track_item: TrackItem,

    pub single: Single,

    pub hover_info: Option<hover_info::HoverInfo>,
}

impl Track {
    pub fn new2(track_id: TrackId, buffer_id: BufferId, audio: &mut AudioManager) -> Result<Self> {
        // let buffer = audio

        todo!()
    }
    pub fn new(track_id: TrackId, buffer_id: BufferId, buffer_pool: &mut BufferPool) -> Result<Self> {
        // let buffer = buffer_pool
        //     .get_buffer(buffer_id)
        //     .with_context(|| format!("Buffer {:?} not found in pool", buffer_id))?;

        // Ok(Self {
        //     track_id,
        //     screen_rect: Rect::default(),
        //     // sample_rect: SampleRectEnum::from_buffer(buffer),
        //     // view_rect: Rect::default(),
        //     samples_per_pixel: None,
        //     view_buffer: model::ViewBuffer::SingleSamples(vec![]),
        //     track_item: TrackItem::new(buffer_id, SampleRectEnum::from_buffer(buffer)),
        //     single: Single::new(buffer_id),
        //     hover_info: None,
        // })
        todo!()
    }
}
