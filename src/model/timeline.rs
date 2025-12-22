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

// Initialization is a bit tricky, I try to describe some scenarios (pre code, read code ;) )
//
// scenarios:
// first we make a timeline when we don't ahve tracks
// whe might load tracks with cli -> now we know the ix range
// we might not load tracks with cli -> no ix range
// render UI - get screen rect first time
//   -> we also have ix_range -> determine samples_per_pixel
//   -> we don't have ix_range -> can't render
//
// we might render again - get new screen rect
//   -> with ix_rang -> we have samples_per_pixel, determine new ix_range
//   -> no ix_range -> can't render
//
// load wav file
//   -> with ix_range -> do noting, or update ix_range to match largest file
//   -> no ix_range -> set ix_range first time -> determine samples_per_pixel
//

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Timeline2 {
    /// The zoom level for all tracks
    pub samples_per_pixel: f64,

    /// The sample index offset in order to draw the ruler at the correct position
    pub ix_start: audio::sample::FracIx,
}

impl Timeline2 {
    pub fn get_ix_range(&self, pixel_width: f64) -> audio::sample::FracIxRange {
        let start = self.ix_start;
        let end = start + pixel_width * self.samples_per_pixel;
        audio::sample::FracIxRange { start, end }
    }

    // pub fn zoom_to_ix_range(&mut self, ix_range: audio::sample::FracIxRange) {
    //     let start = ix_range.start - self.ix_start;
    //     let end = ix_range.end - self.ix_start;
    //     self.ix_start = ix_range.start;
    //     self.samples_per_pixel = end - start;
    // }
}

// #[derive(Debug, Clone, Default)]
// pub enum TimelineE {
//     #[default]
//     Uninitialized,
//     HasSamplesPerPixel {
//         samples_per_pixel: f64,
//     },
//     Ready(Timeline2),
// }

// impl TimelineE {
//     pub fn set_samples_per_pixel(&mut self, new_samples_per_pixel: f64) {
//         match self {
//             Self::Uninitialized => {
//                 *self = Self::HasSamplesPerPixel {
//                     samples_per_pixel: new_samples_per_pixel,
//                 };
//             }
//             Self::HasSamplesPerPixel { .. } => {
//                 *self = Self::HasSamplesPerPixel {
//                     samples_per_pixel: new_samples_per_pixel,
//                 };
//             }
//             Self::Ready(timeline) => {
//                 *self = Self::Ready(Timeline2 {
//                     samples_per_pixel: new_samples_per_pixel,
//                     ix_start: timeline.ix_start,
//                 });
//             }
//         }
//     }

//     pub fn get_mut(&mut self) -> Option<&mut Timeline2> {
//         match self {
//             Self::Ready(timeline) => Some(timeline),
//             _ => None,
//         }
//     }

//     pub fn get(&self) -> Option<&Timeline2> {
//         match self {
//             Self::Ready(timeline) => Some(timeline),
//             _ => None,
//         }
//     }

//     pub fn get_ix_range(&self, pixel_width: f64) -> Option<audio::sample::FracIxRange> {
//         Some(self.get()?.get_ix_range(pixel_width))
//     }
// }
