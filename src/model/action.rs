use crate::{model::track2::TrackId, wav};

#[derive(Debug, PartialEq, Clone)]
pub enum Action {
    RemoveTrackOld(u64),
    RemoveAllTracks,
    RemoveTrack(TrackId),

    OpenFile(wav::ReadConfig),

    /// Set x-zoom so the longest track is full width
    /// Set y-zoom to fill the screen, with a minimum height per track
    ZoomToFull,
}
