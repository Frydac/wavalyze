//! Pending state for a track dropped *on* another track in the tracks panel: rather than diffing
//! immediately (which is easy to trigger by accident while reordering), the view shows a small
//! confirmation context menu while `model.pending_track_diff` is `Some`.

use crate::model::track::TrackId;

#[derive(Debug, Clone)]
pub struct PendingTrackDiff {
    /// The dragged track (minuend): the diff is `dragged - dropped_on`.
    pub dragged: TrackId,
    /// The track that was dropped on (subtrahend); the diff track lands directly after it.
    pub dropped_on: TrackId,
    /// Screen position where the menu is anchored (the drop point).
    pub screen_pos: (f32, f32),
    /// `false` on the frame the menu opens; flipped to `true` after the first render so the same
    /// pointer release that opened the menu can't immediately close it via click-outside.
    pub armed: bool,
}
