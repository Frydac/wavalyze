use crate::{
    audio::BufferId,
    model::{
        PixelCoord, hover_info::HoverInfoE, jobs, selection_info::SelectionInfoE, track::TrackId,
    },
    wav,
    wav::file2::FileId,
};
use anyhow::{Context, Result};

#[derive(Debug)]
pub enum Action {
    RemoveAllTracks, // TODO: still needed?

    CloseAll, // remove all tracks/buffers/files
    RemoveTrack(TrackId),
    /// Unload a whole file: all its channels, their tracks, the audio buffers, and the file spec.
    CloseFile {
        file_id: FileId,
    },

    /// Load a WAV from a filesystem path (native CLI startup, future native file-path flows).
    OpenFilePath(wav::ReadConfig),
    /// Load a WAV from in-memory bytes (file picker, drag-drop, wasm).
    OpenFileBytes(wav::ReadConfigBytes),
    /// Load two WAV files and compute a diff track from their selected channels. When the channel
    /// pairing is ambiguous (multichannel input without an explicit selection), this opens the
    /// channel-pairing matrix dialog instead.
    OpenDiffFilePaths {
        file_a: wav::ReadConfig,
        file_b: wav::ReadConfig,
    },
    /// OK in the channel-pairing dialog: take the pending pairing and start the selected diffs.
    ConfirmDiffPairing,
    /// Cancel in the channel-pairing dialog: discard the pending pairing.
    CancelDiffPairing,
    StartDemoJob(jobs::DemoTimedConfig),
    LoadDemo,
    /// Integrate a fully-loaded WAV file into the model. Pushed by background load jobs on success.
    IntegrateLoadedFile {
        generation: u64,
        loaded: wav::read::LoadedFile,
    },
    IntegrateLoadedDiff {
        generation: u64,
        diff: jobs::LoadedDiff,
    },

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
    DiffBuffers {
        buffer_id_a: BufferId,
        buffer_id_b: BufferId,
        sample_ix_offset_a: crate::audio::sample::Ix,
        sample_ix_offset_b: crate::audio::sample::Ix,
    },
    /// Diff two tracks (dragged onto dropped-on in the tracks panel). The diff is
    /// `dragged - dropped_on`, and the resulting diff track is inserted directly after `dropped_on`.
    DiffTracks {
        dragged: TrackId,
        dropped_on: TrackId,
    },
    IntegrateDiffBuffer {
        generation: u64,
        diff: jobs::ComputedDiff,
    },
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
            Action::CloseAll => {
                model.close_all();
            }
            Action::CloseFile { file_id } => {
                model.remove_file(file_id);
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
            Action::OpenDiffFilePaths { file_a, file_b } => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    // Fast path: both inputs resolve to a single channel, so the pairing is
                    // unambiguous and the dialog is skipped — the diff runs as a one-pair special
                    // case of the multichannel path.
                    let single_channel_ix = |config: &wav::ReadConfig| -> Option<wav::read::ChIx> {
                        match config.ch_ixs.as_deref() {
                            Some([ch_ix]) => Some(*ch_ix),
                            Some(_) => None,
                            None => wav::read::peek_nr_channels(&config.filepath)
                                .ok()
                                .filter(|nr_channels| *nr_channels == 1)
                                .map(|_| 0),
                        }
                    };
                    match (single_channel_ix(&file_a), single_channel_ix(&file_b)) {
                        (Some(ch_a), Some(ch_b)) => {
                            model.start_diff_pairs(file_a, file_b, vec![(ch_a, ch_b)]);
                        }
                        _ => {
                            model
                                .open_diff_pairing_dialog(file_a, file_b)
                                .context("Action::OpenDiffFilePaths failed")?;
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                {
                    let _ = (file_a, file_b);
                    tracing::warn!("Action::OpenDiffFilePaths ignored on wasm");
                }
            }
            Action::ConfirmDiffPairing => {
                if let Some(pending) = model.pending_diff_pairing.take() {
                    let pairs = pending.selected_pairs();
                    model.start_diff_pairs(pending.file_a, pending.file_b, pairs);
                }
            }
            Action::CancelDiffPairing => {
                model.pending_diff_pairing = None;
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
            Action::IntegrateLoadedFile { generation, loaded } => {
                if model.is_current_generation(generation) {
                    model
                        .add_loaded_file(loaded)
                        .context("Action::IntegrateLoadedFile failed")?;
                    model.actions.push(Action::ZoomToFull);
                    model.actions.push(Action::FillScreenHeight);
                }
            }
            Action::IntegrateLoadedDiff { generation, diff } => {
                if model.is_current_generation(generation) {
                    model
                        .add_loaded_diff(diff)
                        .context("Action::IntegrateLoadedDiff failed")?;
                    model.actions.push(Action::ZoomToFull);
                    model.actions.push(Action::FillScreenHeight);
                }
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
                model.tracks.pan_x(nr_pixels);
                let _ = model.tracks.update_tracks_to_camera(&model.audio);
            }
            Action::ZoomX {
                nr_pixels,
                center_x,
            } => {
                model.tracks.zoom_x(nr_pixels, center_x);
                let _ = model.tracks.update_tracks_to_camera(&model.audio);
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
            Action::DiffBuffers {
                buffer_id_a,
                buffer_id_b,
                sample_ix_offset_a,
                sample_ix_offset_b,
            } => {
                model
                    .start_diff_buffers_job(
                        buffer_id_a,
                        buffer_id_b,
                        sample_ix_offset_a,
                        sample_ix_offset_b,
                        None,
                    )
                    .context("Action::DiffBuffers failed")?;
            }
            Action::DiffTracks {
                dragged,
                dropped_on,
            } => {
                if dragged != dropped_on
                    && let (Some(track_a), Some(track_b)) = (
                        model.tracks.get_track(dragged),
                        model.tracks.get_track(dropped_on),
                    )
                {
                    let buffer_id_a = track_a.single.buffer_id;
                    let buffer_id_b = track_b.single.buffer_id;
                    let sample_ix_offset_a = track_a.single.sample_ix_offset.round() as i64;
                    let sample_ix_offset_b = track_b.single.sample_ix_offset.round() as i64;
                    model
                        .start_diff_buffers_job(
                            buffer_id_a,
                            buffer_id_b,
                            sample_ix_offset_a,
                            sample_ix_offset_b,
                            Some(dropped_on),
                        )
                        .context("Action::DiffTracks failed")?;
                }
            }
            Action::IntegrateDiffBuffer { generation, diff } => {
                if model.is_current_generation(generation) {
                    model
                        .add_diff_buffer(diff)
                        .context("Action::IntegrateDiffBuffer failed")?;
                    model.actions.push(Action::ZoomToFull);
                    model.actions.push(Action::FillScreenHeight);
                }
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
