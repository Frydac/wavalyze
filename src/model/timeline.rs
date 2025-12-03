use crate::{audio, rect::Rect};



/// Represents a range of audio samples that is visible in a view
///
/// Probably only be using this for a 'global' timeline, the tracks will be positioned relative to
/// this timeline.
/// Then we can do x-axis zooming and panning on the timeline and all tracks know what to do.
#[derive(Debug, Clone, Default)]
pub struct Timeline {
    /// The range of samples that is visible in the view
    /// Fractional, so we can have sub-sample resolution for zooming/panning
    pub ix_range: audio::sample::FracIxRange,

    /// The pixel rectangle in absolute screen coordinates for all the tracks?
    /// maybe only width necessary?
    pub screen_rect: Option<Rect>,
}

impl Timeline {
    pub fn samples_per_pixel(&self) -> Option<f32> {
        self.screen_rect.map(|rect| rect.width() / self.ix_range.len() as f32)
    }
}
