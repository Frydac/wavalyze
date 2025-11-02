use slotmap::SlotMap;

use crate::{
    model::track2::{Track, TrackId},
    pos,
};

#[derive(Default, Debug, Clone)]
pub struct Tracks {
    pub tracks: SlotMap<TrackId, Track>,

    // hover
    hover_info: Option<TracksHoverInfo>,
    // selection
    // zoom
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TracksHoverInfo {
    pub track_id: TrackId,
    pub screen_pos: pos::Pos,
}
