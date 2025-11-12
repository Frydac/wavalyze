use crate::audio::{self, sample_range2::SampleFractionalIx};

/// Repesents a time domain view of 2 audio buffers and their difference
#[derive(Debug, PartialEq, Clone)]
pub struct Diff {
    pub buffer_id_a: audio::BufferId,
    pub buffer_id_b: audio::BufferId,

    /// Rectangal view over the buffer's samples
    pub sample_rect: audio::SampleRectE,

    /// For positioning wrt the 'absolute' sample range of the track
    /// allow for separate offset for each buffer
    pub sample_ix_offset_a: SampleFractionalIx,
    pub sample_ix_offset_b: SampleFractionalIx,
}
