use slotmap::new_key_type;

use crate::{audio::{buffer_pool::BufferId}, model::{self, hover_info}, rect::Rect};

new_key_type! { pub struct TrackId; }

#[derive(Debug, PartialEq, Clone)]
pub struct Track {
    // pub id: Option<TrackId>,
    pub track_id: TrackId, // Id of this track in storage, mabye better Optional, lets try without
                           // for now
    /// The pixel rectangle with absolute screen coordinates that should display self.sample_rect of
    /// samples
    pub screen_rect: Rect,

    // TODO: we might display multiple buffers
    // * need mutliple sample_rects
    // * render into one view_rect? Multiple maybe allow for moving 1 without recomputing the other
    buffer_id: BufferId, // SampleBuffer Id

    /// The rectangle of samples indices that are currently visible,
    /// indices into self.buffer.
    // TODO: make sample_rect like SampleBuffer
    // pub sample_rect: audio::SampleRect,

    // x range is pixel width starting at 0.0
    // y range is sample_rect sample range coordinates I think
    // NOTE: not really needed, we can just use screen_rect and 'normalize' it, they should be very similar
    pub view_rect: Rect,


    /// Zoom level in x direction
    ///
    /// * Doesn't change when updating the screen_rect to keep the zoom stable
    pub samples_per_pixel: Option<f32>,

    /// Contains all the samples as pixel positions relative to top_left (0,0), currently to be
    /// rendered by the track::View
    /// The final transformation to absolute screen coordinates is done in the view::Track
    view_buffer: model::ViewBuffer,

    pub hover_info: Option<hover_info::HoverInfo>,
}

impl Track {
    pub fn new(track_id: TrackId, buffer_id: BufferId) -> Self {
        Self {
            track_id,
            buffer_id,
            screen_rect: Rect::default(),
            // sample_rect: audio::SampleRect::default(),
            view_rect: Rect::default(),
            samples_per_pixel: None,
            view_buffer: model::ViewBuffer::SingleSamples(vec![]),
            hover_info: None,
        }
    }
}
