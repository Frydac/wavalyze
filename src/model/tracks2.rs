use crate::model::ruler;
use anyhow::Result;
use slotmap::SlotMap;

use crate::{
    model::track2::{Track, TrackId},
    pos,
    wav::file2::File,
};

#[derive(Default, Debug, Clone)]
pub struct Tracks {
    pub time_line: ruler::Time,
    pub tracks: SlotMap<TrackId, Track>,

    // hover
    // hover_info: Option<TracksHoverInfo>,
    // selection
    // zoom
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TracksHoverInfo {
    pub track_id: TrackId,
    pub screen_pos: pos::Pos,
}

impl Tracks {
    pub fn add_tracks_from_file(&mut self, file: &File) -> Result<()> {
        for (ch_ix, channel) in file.channels.iter() {
            let track = Track::new2(channel.buffer_id)?;
            self.tracks.insert(track);
        }

        Ok(())
    }

    pub fn zoom_to_full(&mut self) {
        todo!()
    }
}
