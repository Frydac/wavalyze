use crate::{
    model::track2::TrackId,
};

#[derive(Debug, PartialEq, Clone)]
pub enum Action {
    RemoveTrackOld(u64),
    RemoveAllTracks,
    RemoveTrack(TrackId),
}
