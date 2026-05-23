use crate::{
    audio::BufferId,
    model::{
        PixelCoord, hover_info::HoverInfoE, jobs, selection_info::SelectionInfoE, track::TrackId,
    },
    wav,
};
use anyhow::{Context, Result};

#[derive(Debug, PartialEq)]
pub enum Action {
    RemoveAllTracks,
    RemoveTrack(TrackId),

    /// Load a WAV from a filesystem path (native CLI startup, future native file-path flows).
    OpenFilePath(wav::ReadConfig),
    /// Load a WAV from in-memory bytes (file picker, drag-drop, wasm).
    OpenFileBytes(wav::ReadConfigBytes),
    StartDemoJob(jobs::DemoTimedConfig),
    LoadDemo,
    /// Integrate a fully-loaded WAV file into the model. Pushed by background load jobs on success.
    IntegrateLoadedFile(wav::read::LoadedFile),

    /// Set x-zoom so the longest track is full width
    /// Set y-zoom to fill the screen, with a minimum height per track
    ZoomToFull,
    /// Set x-zoom so the current selection fills the visible width.
    ZoomToSelection,
    /// Set x-zoom to sample-level detail, centered on the left edge of the current selection.
    ZoomToSelectionLeftEdge,
    /// Set x-zoom to sample-level detail, centered on the right edge of the current selection.
    ZoomToSelectionRightEdge,

    /// Adjust height of tracks to fit the screen, keeping in mind the min_height for each track
    FillScreenHeight,

    /// Move the _view_ of all the tracks to the lef (negative value) or right (positive value)
    PanX {
        nr_pixels: PixelCoord,
    },
    /// Zoom the _view_ of all the tracks, center_x should be absolute x-position of the
    /// mouse/center
    ZoomX {
        nr_pixels: PixelCoord,
        center_x: PixelCoord,
    },

    /// Move one track up or down wrt to the sample values
    PanY {
        track_id: TrackId,
        nr_pixels: PixelCoord,
    },
    /// Reset the sample value range to full-scale for a single track.
    RecenterY {
        track_id: TrackId,
    },
    /// Reset the sample value range to full-scale for all tracks.
    RecenterYAll,
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

    // SetSelection
    SetSelection(SelectionInfoE),

    /// Start a background job to compute the RMS (in dB) of a single buffer. Result lands via
    /// `Action::SetBufferRms` once the worker finishes.
    ComputeBufferRms(BufferId),
    /// Integrate a freshly computed RMS value. Pushed by the compute-rms worker via `actions_tx`.
    /// Silently dropped if the buffer no longer exists (e.g., file closed mid-flight).
    SetBufferRms {
        buffer_id: BufferId,
        rms_db: f32,
    },
}

impl Action {
    pub fn process(self, model: &mut crate::model::Model) -> Result<()> {
        if !matches!(self, Action::SetHoverInfo(_)) {
            tracing::trace!("Action::process: {:?}", self);
        }

        match self {
            Action::RemoveTrack(track_id) => {
                model.tracks.remove_track(track_id);
            }
            Action::RemoveAllTracks => {
                model.tracks.remove_all_tracks();
            }
            Action::OpenFilePath(read_config) => {
                #[cfg(not(target_arch = "wasm32"))]
                model.start_load_wav_path_job(read_config);
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = read_config;
                    tracing::warn!("Action::OpenFilePath ignored on wasm");
                }
            }
            Action::OpenFileBytes(read_config) => {
                model.start_load_wav_job(read_config);
            }
            Action::StartDemoJob(config) => {
                model.start_demo_job(config);
            }
            Action::LoadDemo => {
                model
                    .load_demo_waveform()
                    .context("Action::LoadDemo failed")?;
                model.actions.push(Action::ZoomToFull);
                model.actions.push(Action::FillScreenHeight);
            }
            Action::IntegrateLoadedFile(loaded) => {
                model
                    .add_loaded_file(loaded)
                    .context("Action::IntegrateLoadedFile failed")?;
                model.actions.push(Action::ZoomToFull);
                model.actions.push(Action::FillScreenHeight);
            }
            Action::ZoomToFull => {
                model.tracks.zoom_to_full(&model.audio)?;
            }
            Action::ZoomToSelection => {
                model.tracks.zoom_to_selection(&model.audio)?;
            }
            Action::ZoomToSelectionLeftEdge => {
                model
                    .tracks
                    .zoom_to_selection_edge(&model.audio, SelectionEdge::Left)?;
            }
            Action::ZoomToSelectionRightEdge => {
                model
                    .tracks
                    .zoom_to_selection_edge(&model.audio, SelectionEdge::Right)?;
            }
            Action::FillScreenHeight => {
                let min_height = model.user_config.track.min_height;
                model.tracks.fill_screen_height(min_height)?;
            }
            Action::PanX { nr_pixels } => {
                model.tracks.ruler.pan_x(nr_pixels);
            }
            Action::ZoomX {
                nr_pixels,
                center_x,
            } => {
                model.tracks.ruler.zoom_x(nr_pixels, center_x);
            }
            Action::PanY {
                track_id,
                nr_pixels,
            } => {
                model.tracks.pan_track_value_range(
                    track_id,
                    nr_pixels,
                    model.user_config.value_display_scale,
                )?;
            }
            Action::RecenterY { track_id } => {
                model.tracks.recenter_track_value_range(track_id)?;
            }
            Action::RecenterYAll => {
                model.tracks.recenter_all_value_ranges()?;
            }
            Action::ZoomY {
                track_id,
                nr_pixels,
                center_y,
            } => {
                model.tracks.zoom_track_value_range(
                    track_id,
                    nr_pixels,
                    center_y,
                    model.user_config.value_display_scale,
                )?;
            }
            Action::SetHoverInfo(hover_info) => {
                model.tracks.hover_info = hover_info;
            }
            Action::SetSelection(selection_info) => {
                model.tracks.selection_info = selection_info;
            }
            Action::ComputeBufferRms(buffer_id) => {
                model
                    .start_compute_rms_job(buffer_id)
                    .context("Action::ComputeBufferRms failed")?;
            }
            Action::SetBufferRms { buffer_id, rms_db } => {
                if model.audio.buffers.contains_key(buffer_id) {
                    model.audio.rms_db.insert(buffer_id, rms_db);
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionEdge {
    Left,
    Right,
}
