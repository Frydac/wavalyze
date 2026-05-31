use crate::audio;

/// Represents a time domain view of 2 audio buffers and their difference.
#[derive(Debug, PartialEq, Clone)]
pub struct Diff {
    /// this buffer should be (buffer_a - buffer_b)
    pub buffer_id_diff: audio::BufferId,

    pub buffer_id_a: audio::BufferId,
    pub buffer_id_b: audio::BufferId,

    /// For positioning wrt the 'absolute' sample range of the track
    /// allow for separate offset for each buffer
    pub sample_ix_offset_a: i64,
    pub sample_ix_offset_b: i64,

    /// Offset used to place the computed diff buffer on the same absolute sample timeline.
    pub sample_ix_offset_diff: i64,
}
