use crate::{model::PixelCoord, model::hover_info::HoverInfoE, model::track2::TrackId, wav};
use anyhow::{Context, Result};

#[derive(Debug, PartialEq, Clone)]
pub enum Action {
    RemoveTrackOld(u64),
    RemoveAllTracks,
    RemoveTrack(TrackId),

    OpenFile(wav::ReadConfig),
    OpenFileBytes(wav::ReadConfigBytes),
    LoadDemo,

    /// Set x-zoom so the longest track is full width
    /// Set y-zoom to fill the screen, with a minimum height per track
    ZoomToFull,

    /// Adjust height of tracks to fit the screen, keeping in mind the min_height for each track
    FillScreenHeight,

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
    /// Update hover info on the next frame so all views stay in sync.
    SetHoverInfo(HoverInfoE),
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
                // Native: load on a worker thread. Wasm: load synchronously (no threads).
                let progress = crate::wav::read::new_load_progress_handle();
                let load_id = model
                    .load_mgr
                    .start_load(read_config.filepath.clone(), progress.clone());
                let tx = model.load_mgr.sender();
                let read_config = read_config.clone();
                #[cfg(not(target_arch = "wasm32"))]
                std::thread::spawn(move || {
                    let result = crate::wav::read::read_to_loaded_file_with_progress(
                        &read_config,
                        load_id,
                        Some(progress.as_ref()),
                    )
                    .context("Action::OpenFile failed");
                    let _ = tx.send(match result {
                        Ok(loaded) => crate::wav::read::LoadResult::Ok(loaded),
                        Err(error) => crate::wav::read::LoadResult::Err { load_id, error },
                    });
                });
                #[cfg(target_arch = "wasm32")]
                {
                    let result = crate::wav::read::read_to_loaded_file_with_progress(
                        &read_config,
                        load_id,
                        Some(progress.as_ref()),
                    )
                    .context("Action::OpenFile failed");
                    let _ = tx.send(match result {
                        Ok(loaded) => crate::wav::read::LoadResult::Ok(loaded),
                        Err(error) => crate::wav::read::LoadResult::Err { load_id, error },
                    });
                }
            }
            Action::OpenFileBytes(read_config) => {
                // Byte-based loads are used by wasm drag-and-drop (no filesystem access).
                let progress = crate::wav::read::new_load_progress_handle();
                let label = read_config
                    .name
                    .clone()
                    .unwrap_or_else(|| "file".to_string());
                let load_id = model
                    .load_mgr
                    .start_load(std::path::PathBuf::from(label), progress.clone());
                let tx = model.load_mgr.sender();
                let read_config = read_config.clone();
                #[cfg(not(target_arch = "wasm32"))]
                std::thread::spawn(move || {
                    let result = crate::wav::read::read_bytes_to_loaded_file_with_progress(
                        &read_config,
                        load_id,
                        Some(progress.as_ref()),
                    )
                    .context("Action::OpenFileBytes failed");
                    let _ = tx.send(match result {
                        Ok(loaded) => crate::wav::read::LoadResult::Ok(loaded),
                        Err(error) => crate::wav::read::LoadResult::Err { load_id, error },
                    });
                });
                #[cfg(target_arch = "wasm32")]
                {
                    let result = crate::wav::read::read_bytes_to_loaded_file_with_progress(
                        &read_config,
                        load_id,
                        Some(progress.as_ref()),
                    )
                    .context("Action::OpenFileBytes failed");
                    let _ = tx.send(match result {
                        Ok(loaded) => crate::wav::read::LoadResult::Ok(loaded),
                        Err(error) => crate::wav::read::LoadResult::Err { load_id, error },
                    });
                }
            }
            Action::LoadDemo => {
                model
                    .load_demo_waveform()
                    .context("Action::LoadDemo failed")?;
                model.actions.push(Action::ZoomToFull);
                model.actions.push(Action::FillScreenHeight);
            }
            Action::ZoomToFull => {
                model.tracks2.zoom_to_full(&model.audio)?;
                // model.tracks.zoom_to_full();
                // todo!();
            }
            Action::FillScreenHeight => {
                let min_height = model.user_config.track.min_height;
                model.tracks2.fill_screen_height(min_height)?;
            }
            Action::ShiftX { nr_pixels } => {
                model.tracks2.ruler.shift_x(*nr_pixels);
            }
            Action::ZoomX {
                nr_pixels,
                center_x,
            } => {
                model.tracks2.ruler.zoom_x(*nr_pixels, *center_x);
            }
            Action::ShiftY {
                track_id,
                nr_pixels,
            } => todo!(),
            Action::ZoomY {
                track_id,
                nr_pixels,
                center_y,
            } => todo!(),
            Action::SetHoverInfo(hover_info) => {
                model.tracks2.hover_info = *hover_info;
            }
        }

        Ok(())
    }
}
