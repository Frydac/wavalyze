use crate::{model::track2::TrackId, model::PixelCoord, wav};
use anyhow::Result;

#[derive(Debug, PartialEq, Clone)]
pub enum Action {
    RemoveTrackOld(u64),
    RemoveAllTracks,
    RemoveTrack(TrackId),

    OpenFile(wav::ReadConfig),

    /// Set x-zoom so the longest track is full width
    /// Set y-zoom to fill the screen, with a minimum height per track
    ZoomToFull,

    /// Move the _view_ of all the tracks to the lef (negative value) or right (positive value)
    ShiftX {
        nr_pixels: PixelCoord,
    },
    /// Zoom the _view_ of all the tracks, center_x should be absolute x-position of the
    /// mouse/center
    ZoomX {
        nr_pixels: PixelCoord,
        center_x: PixelCoord,
    },

    /// Move one track up or down wrt to the sample values
    ShiftY {
        track_id: TrackId,
        nr_pixels: PixelCoord,
    },
    /// Zoom the _view_ of the given track, center_y should be absolute y-position of the
    /// mouse/center
    ZoomY {
        track_id: TrackId,
        nr_pixels: PixelCoord,
        center_y: PixelCoord,
    },
    // TODO: zoom rect?
}

impl Action {
    pub fn process(&self, model: &mut crate::model::Model) -> Result<()> {
        match self {
            Action::RemoveTrackOld(track_id) => {
                model.tracks.remove_track(*track_id);
            }
            Action::RemoveTrack(track_id) => {
                model.tracks2.remove_track(*track_id);
            }
            Action::RemoveAllTracks => {
                model.tracks.tracks.clear();
                model.tracks2.remove_all_tracks();
            }
            Action::OpenFile(read_config) => {
                // TODO: give extra info about the file?
                model.load_wav(read_config)?;
                model.actions.push(Action::ZoomToFull);
            }
            Action::ZoomToFull => {
                model.tracks2.zoom_to_full(&model.audio)?;
                // model.tracks.zoom_to_full();
                // todo!();
            }
            Action::ShiftX { nr_pixels } => {
                model.tracks2.ruler.shift_x(*nr_pixels);
            }
            Action::ZoomX { nr_pixels, center_x } => {
                model.tracks2.ruler.zoom_x(*nr_pixels, *center_x);
            }
            Action::ShiftY { track_id, nr_pixels } => todo!(),
            Action::ZoomY {
                track_id,
                nr_pixels,
                center_y,
            } => todo!(),
        }

        Ok(())
    }
}
