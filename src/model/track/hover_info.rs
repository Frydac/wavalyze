use std::ops::Range;

use crate::{Pos, audio::SampleBuffer};

/// Stores information needed to render mouse hover position over the tracks
#[derive(Debug, PartialEq, Clone)]
pub struct HoverInfo {
    /// absolute (mouse) screen position in pixel coordinates
    pub screen_pos: Pos,

    /// sample values rendedred at screen_pos.x
    pub samples: SampleBuffer,
    /// Range of samples that are rendered at screen_pos.x
    pub sample_range: Range<u64>,

    /// We have hover info's for each track when mouse is over a track,
    /// but only one hover info is for the track with the mouse (x,y), the others only for (x)
    pub contains_pointer: bool,
}
