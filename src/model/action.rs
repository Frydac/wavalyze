//! Commands from the UI and completion messages from background workers.
//!
//! Both are queued and processed after drawing, so files and tracks are not
//! added or removed while the UI is still rendering them.

use crate::{
    audio::BufferId,
    model::{
        PixelCoord, hover_info::HoverInfoE, jobs, selection_info::SelectionInfoE, track::TrackId,
        track_selection::TrackSelectionMode,
    },
    wav,
    wav::file::FileId,
};
use anyhow::{Context, Result};

/// Workspace commands from UI interactions and completion messages from background workers.
/// Queuing structural changes avoids e.g. removing files or tracks while their UI is still drawing.
/// Variants are grouped by functionality, keeping worker results beside the commands that start
/// their work. Result validation is implemented by the relevant handlers.
#[derive(Debug)]
pub enum Action {
    ////////////////////////////////////////////////////////////////////////////////////////////////
    // Workspace and files
    CloseAll, // remove all tracks/buffers/files
    LoadDemo,
    /// Load a WAV from a native CLI, picker, or dropped path, retaining its reload recipe.
    OpenFilePath(wav::ReadConfig),
    /// Load supplied WAV bytes, including browser picker/drop inputs without disk access.
    OpenFileBytes(wav::ReadConfigBytes),
    /// Integrate a fully-loaded WAV file into the model. Pushed by background load jobs on success.
    IntegrateLoadedFile {
        generation: u64,
        loaded: wav::read::LoadedFile,
    },
    /// Unload a whole file: all its channels, their tracks, the audio buffers, and the file spec.
    CloseFile {
        file_id: FileId,
    },
    /// Request one atomic reload batch; both individual and Update all controls use this action.
    #[cfg(not(target_arch = "wasm32"))]
    ReloadFiles(Vec<FileId>),
    /// Validate and integrate a prepared batch on the UI thread, or expose its failure for retry.
    #[cfg(not(target_arch = "wasm32"))]
    FinishReload(Box<crate::model::files::reload::ReloadResult>),

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // Track organization and layout
    RemoveAllTracks, // TODO: still needed?
    RemoveTrack(TrackId),
    /// Reorder `dragged` to `to_gap_ix` (a gap index in the current track order). Pushed when a
    /// track is dropped *between* two tracks.
    ReorderTrack {
        dragged: TrackId,
        to_gap_ix: usize,
    },
    /// Enable or disable continuous equal-height layout. Enabling immediately fits visible tracks.
    SetEqualHeightLayout(bool),
    /// Manually set one track's height, disabling continuous equal-height layout.
    SetTrackHeight {
        track_id: TrackId,
        height: f32,
    },
    /// Manually set every track's height, disabling continuous equal-height layout.
    SetTracksHeight {
        height: f32,
    },

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // Diff creation and results
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
    DiffBuffers {
        buffer_id_a: BufferId,
        buffer_id_b: BufferId,
        sample_ix_offset_a: crate::audio::sample::Ix,
        sample_ix_offset_b: crate::audio::sample::Ix,
    },
    /// Diff two tracks (dragged onto dropped-on). The diff is
    /// `dragged - dropped_on`, and the resulting diff track is inserted directly after `dropped_on`.
    DiffTracks {
        dragged: TrackId,
        dropped_on: TrackId,
    },
    IntegrateLoadedDiff {
        generation: u64,
        diff: jobs::LoadedDiff,
    },
    IntegrateDiffBuffer {
        generation: u64,
        diff: jobs::ComputedDiff,
    },

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // File and track alignment
    /// Set a file's absolute sample offset and update channel tracks inheriting it.
    SetFileSampleIxOffset {
        file_id: FileId,
        sample_ix_offset: crate::audio::sample::Ix,
    },
    /// Enable or disable file-offset inheritance for a file-backed track.
    SetTrackUseFileOffset {
        track_id: TrackId,
        use_file_offset: bool,
    },
    /// Set a detached or standalone track's absolute sample offset.
    SetTrackSampleIxOffset {
        track_id: TrackId,
        sample_ix_offset: f64,
    },
    /// Scan one track buffer for its leading-silence boundary.
    DetectTrackOffset {
        track_id: TrackId,
        mode: jobs::OffsetDetectionMode,
    },
    /// Scan every loaded channel buffer in one file and use the minimum detected boundary.
    DetectFileOffset {
        file_id: FileId,
        mode: jobs::OffsetDetectionMode,
    },
    /// Buffer identity prevents a completed alignment job from changing newly reloaded audio.
    OffsetDetectedChecked {
        result: jobs::OffsetDetectionResult,
        buffers: Vec<std::sync::Arc<crate::audio::buffer::BufferE>>,
    },
    /// Result of asynchronously scanning buffers for a leading-silence boundary.
    OffsetDetected(jobs::OffsetDetectionResult),

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // View navigation and display
    // TODO: zoom rect?
    /// Set x-zoom so the longest track is full width
    /// Set y-zoom to fill the screen, with a minimum height per track
    ZoomToFull,
    /// Set x-zoom so the current selection fills the visible width.
    ZoomToSelection,
    /// Set x-zoom to sample-level detail, centered on the left edge of the current selection.
    ZoomToSelectionLeftEdge,
    /// Set x-zoom to sample-level detail, centered on the right edge of the current selection.
    ZoomToSelectionRightEdge,
    /// Move the _view_ of all the tracks to the left (negative value) or right (positive value)
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
    /// Zoom the _view_ of the given track, center_y should be absolute y-position of the
    /// mouse/center
    ZoomY {
        track_id: TrackId,
        nr_pixels: PixelCoord,
        center_y: PixelCoord,
    },
    /// Reset the sample value range to full-scale for a single track.
    RecenterY {
        track_id: TrackId,
    },
    /// Reset the sample value range to full-scale for all tracks.
    RecenterYAll,
    /// Detect the peak in the selected range, or the visible range when there is no valid
    /// selection, then auto-fit one track's value range.
    AutoFitY {
        track_id: TrackId,
    },
    /// Result of asynchronously scanning a track range for its normalized absolute peak.
    AutoFitPeakDetected(jobs::AutoFitPeakResult),
    /// Set global processing block size, clamped to at least one sample.
    SetBlockSize(u64),

    // Selection and hover
    /// Update the shared sample-range selection.
    SetSelection(SelectionInfoE),
    /// Apply plain, Ctrl/Cmd-toggle, or Shift-range selection after current UI frame.
    SelectTrack {
        track_id: TrackId,
        mode: TrackSelectionMode,
    },
    /// Update hover info on the next frame so all views stay in sync.
    SetHoverInfo(HoverInfoE),

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // Audio statistics
    /// Start a background job to gather statistics (dB-RMS, peak) over a buffer. The range is
    /// derived from the current selection (whole buffer when nothing is selected) and the track's
    /// sample offset. Result lands via `Action::SetBufferStats` once the worker finishes.
    ComputeBufferStats {
        buffer_id: BufferId,
        track_id: TrackId,
        options: crate::model::stats::StatsOptions,
    },
    /// Integrate freshly gathered buffer statistics. Pushed by the compute-stats worker via
    /// `actions_tx`. Silently dropped if the buffer no longer exists (e.g., file closed mid-flight).
    SetBufferStats {
        buffer_id: BufferId,
        stats: crate::model::stats::BufferStats,
    },

    ////////////////////////////////////////////////////////////////////////////////////////////////
    // Development jobs
    StartDemoJob(jobs::DemoTimedConfig),
}

impl Action {
    pub fn process(self, model: &mut crate::model::Model) -> Result<()> {
        self.log_debug();

        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Action::ReloadFiles(ids) => model.start_reload(ids)?,
            #[cfg(not(target_arch = "wasm32"))]
            Action::FinishReload(result) => model.finish_reload(*result),
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
                            model.start_diff_pairs(file_a, file_b, vec![(ch_a, ch_b)], None, None);
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
                    model.start_diff_pairs(
                        pending.file_a,
                        pending.file_b,
                        pairs,
                        pending.offset_detection_a,
                        pending.offset_detection_b,
                    );
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
                if model.tracks.equal_height_layout {
                    model.actions.push(Action::SetEqualHeightLayout(true));
                }
            }
            Action::IntegrateLoadedFile { generation, loaded } => {
                if model.is_current_generation(generation) {
                    model
                        .add_loaded_file(loaded)
                        .context("Action::IntegrateLoadedFile failed")?;
                    model.actions.push(Action::ZoomToFull);
                    if model.tracks.equal_height_layout {
                        model.actions.push(Action::SetEqualHeightLayout(true));
                    }
                }
            }
            Action::IntegrateLoadedDiff { generation, diff } => {
                if model.is_current_generation(generation) {
                    model
                        .add_loaded_diff(diff)
                        .context("Action::IntegrateLoadedDiff failed")?;
                    model.actions.push(Action::ZoomToFull);
                    if model.tracks.equal_height_layout {
                        model.actions.push(Action::SetEqualHeightLayout(true));
                    }
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
            Action::SetBlockSize(block_size) => {
                model.set_block_size(block_size);
            }
            Action::SetFileSampleIxOffset {
                file_id,
                sample_ix_offset,
            } => {
                model.set_file_sample_ix_offset(file_id, sample_ix_offset);
            }
            Action::SetTrackUseFileOffset {
                track_id,
                use_file_offset,
            } => {
                model.set_track_use_file_offset(track_id, use_file_offset);
            }
            Action::SetTrackSampleIxOffset {
                track_id,
                sample_ix_offset,
            } => {
                model.set_track_sample_ix_offset(track_id, sample_ix_offset);
            }
            Action::DetectTrackOffset { track_id, mode } => {
                let track = model
                    .tracks
                    .get_track(track_id)
                    .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
                if track.use_file_offset {
                    return Ok(());
                }
                let buffer_id = track.single.buffer_id;
                model.start_detect_offset_job(
                    jobs::OffsetDetectionTarget::Track {
                        track_id,
                        buffer_id,
                    },
                    mode,
                    [buffer_id],
                )?;
            }
            Action::DetectFileOffset { file_id, mode } => {
                let buffer_ids = model
                    .files
                    .get(file_id)
                    .ok_or_else(|| anyhow::anyhow!("File {:?} not found", file_id))?
                    .channels
                    .values()
                    .map(|channel| channel.buffer_id)
                    .collect::<Vec<_>>();
                model.start_detect_offset_job(
                    jobs::OffsetDetectionTarget::File { file_id },
                    mode,
                    buffer_ids,
                )?;
            }
            Action::OffsetDetectedChecked { result, buffers } => {
                let current =
                    match result.target {
                        jobs::OffsetDetectionTarget::File { file_id } => {
                            model.files.get(file_id).is_some_and(|file| {
                                file.channels.len() == buffers.len()
                                    && file.channels.values().zip(&buffers).all(|(ch, buffer)| {
                                        model.audio.buffers.get(ch.buffer_id).is_some_and(
                                            |current| std::sync::Arc::ptr_eq(current, buffer),
                                        )
                                    })
                            })
                        }
                        jobs::OffsetDetectionTarget::Track {
                            track_id,
                            buffer_id,
                        } => model
                            .tracks
                            .get_track(track_id)
                            .is_some_and(|t| t.single.buffer_id == buffer_id),
                    };
                if current {
                    Action::OffsetDetected(result).process(model)?;
                }
            }
            Action::OffsetDetected(result) => {
                if let Some(sample_ix_offset) = result.sample_ix_offset {
                    match result.target {
                        jobs::OffsetDetectionTarget::Track {
                            track_id,
                            buffer_id,
                        } => {
                            let target_is_current = model
                                .tracks
                                .get_track(track_id)
                                .is_some_and(|track| track.single.buffer_id == buffer_id);
                            if target_is_current {
                                model.set_track_sample_ix_offset(track_id, sample_ix_offset as f64);
                            }
                        }
                        jobs::OffsetDetectionTarget::File { file_id } => {
                            if model.files.contains_key(file_id) {
                                model.set_file_sample_ix_offset(file_id, sample_ix_offset);
                            }
                        }
                    }
                }
            }
            Action::SetEqualHeightLayout(true) => {
                model.tracks.equal_height_layout = true;
                let min_height = model.user_config.track.min_height;
                model.tracks.fill_screen_height(min_height)?;
            }
            Action::SetEqualHeightLayout(false) => {
                model.tracks.equal_height_layout = false;
            }
            Action::SetTrackHeight { track_id, height } => {
                model.tracks.equal_height_layout = false;
                model.tracks.set_track_height(track_id, height);
            }
            Action::SetTracksHeight { height } => {
                model.tracks.equal_height_layout = false;
                model.tracks.set_tracks_height(height);
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
            Action::AutoFitY { track_id } => {
                let local_range = model
                    .tracks
                    .auto_fit_local_ix_range(track_id, &model.audio)?;
                if let Some(local_range) = local_range {
                    let buffer_id = model
                        .tracks
                        .get_track(track_id)
                        .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?
                        .single
                        .buffer_id;
                    model.start_detect_peak_job(track_id, buffer_id, local_range)?;
                }
            }
            Action::RecenterY { track_id } => {
                model.tracks.recenter_track_value_range(track_id)?;
            }
            Action::RecenterYAll => {
                model.tracks.recenter_all_value_ranges()?;
                model.user_config.value_display_scale.skew_factor = 0.0;
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
            Action::SelectTrack { track_id, mode } => {
                let tracks = &mut model.tracks;
                tracks
                    .track_selection
                    .apply(track_id, mode, &tracks.tracks_order);
            }
            Action::ComputeBufferStats {
                buffer_id,
                track_id,
                options,
            } => {
                let offset = model
                    .tracks
                    .get_track(track_id)
                    .map(|track| track.single.sample_ix_offset.round() as i64)
                    .unwrap_or(0);
                let buffer_len = model.audio.get_buffer(buffer_id)?.nr_samples() as i64;
                // Global-timeline range: the selection if any, else the whole buffer mapped to
                // global space (local `0..len` shifted by `-offset`, since local = global + offset).
                let global_range = match model.tracks.selection_info {
                    crate::model::selection_info::SelectionInfoE::IsSelected(sel) => sel.ix_rng,
                    crate::model::selection_info::SelectionInfoE::NotSelected => {
                        crate::audio::sample::IxRange {
                            start: -offset,
                            end: buffer_len - offset,
                        }
                    }
                };
                model
                    .start_compute_stats_job(buffer_id, global_range, offset, options)
                    .context("Action::ComputeBufferStats failed")?;
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
            Action::ReorderTrack { dragged, to_gap_ix } => {
                model.tracks.move_track(dragged, to_gap_ix);
            }
            Action::IntegrateDiffBuffer { generation, diff } => {
                if model.is_current_generation(generation) {
                    model
                        .add_diff_buffer(diff)
                        .context("Action::IntegrateDiffBuffer failed")?;
                    model.actions.push(Action::ZoomToFull);
                    if model.tracks.equal_height_layout {
                        model.actions.push(Action::SetEqualHeightLayout(true));
                    }
                }
            }
            Action::SetBufferStats { buffer_id, stats } => {
                if model.audio.buffers.contains_key(buffer_id) {
                    model.audio.stats.insert(buffer_id, stats);
                }
            }
            Action::AutoFitPeakDetected(result) => {
                model.tracks.apply_auto_fit_peak(
                    result.track_id,
                    result.buffer_id,
                    result.magnitude_norm,
                    model.user_config.value_display_scale,
                );
            }
        }

        Ok(())
    }

    /// We don't want to log _every_ action at debug log level, it would be too noisy.
    fn log_debug(&self) {
        match self {
            Action::OffsetDetectedChecked { .. } => {
                tracing::debug!(action = "OffsetDetectedChecked", "Processing action")
            }
            #[cfg(not(target_arch = "wasm32"))]
            Action::FinishReload(_) => {
                tracing::debug!(action = "FinishReload", "Processing action")
            }
            // Continuous pointer and drag interactions would overwhelm debug logs.
            Action::PanX { .. }
            | Action::ZoomX { .. }
            | Action::PanY { .. }
            | Action::ZoomY { .. }
            | Action::SetHoverInfo(_)
            | Action::SetSelection(_)
            | Action::SetFileSampleIxOffset { .. }
            | Action::SetTrackSampleIxOffset { .. }
            | Action::SetTrackHeight { .. }
            | Action::SetTracksHeight { .. } => {}
            // Avoid formatting payloads containing complete files or decoded audio buffers.
            Action::OpenFileBytes(config) => tracing::debug!(
                action = "OpenFileBytes",
                name = ?config.name,
                bytes = config.bytes.len(),
                "Processing action"
            ),
            Action::IntegrateLoadedFile { generation, loaded } => tracing::debug!(
                action = "IntegrateLoadedFile",
                generation,
                load_id = loaded.load_id,
                channels = loaded.channels.len(),
                "Processing action"
            ),
            Action::IntegrateLoadedDiff { generation, diff } => tracing::debug!(
                action = "IntegrateLoadedDiff",
                generation,
                file_a_load_id = diff.file_a.load_id,
                file_b_load_id = diff.file_b.load_id,
                pairs = diff.pairs.len(),
                "Processing action"
            ),
            Action::IntegrateDiffBuffer { generation, diff } => tracing::debug!(
                action = "IntegrateDiffBuffer",
                generation,
                buffer_id_a = ?diff.buffer_id_a,
                buffer_id_b = ?diff.buffer_id_b,
                "Processing action"
            ),
            action => tracing::debug!(?action, "Processing action"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionEdge {
    Left,
    Right,
}

#[cfg(test)]
mod tests {
    use super::Action;
    use crate::model::{
        Model,
        config::TrackConfig,
        jobs,
        test_support::{add_buffer, make_file},
    };

    #[test]
    fn equal_height_actions_enable_and_disable_the_layout_mode() {
        let mut model = Model::default();
        model.tracks.available_height = 120.0;
        let config = TrackConfig::default();
        let buffer_a = add_buffer(&mut model);
        let buffer_b = add_buffer(&mut model);
        let track_a = model
            .tracks
            .add_track_to_end(buffer_a, 48_000, &config)
            .unwrap();
        let track_b = model
            .tracks
            .add_track_to_end(buffer_b, 48_000, &config)
            .unwrap();
        model.tracks.equal_height_layout = false;

        Action::SetEqualHeightLayout(true)
            .process(&mut model)
            .unwrap();

        assert!(model.tracks.equal_height_layout);
        assert_eq!(model.tracks.get_track_height(track_a), Some(60.0));
        assert_eq!(model.tracks.get_track_height(track_b), Some(60.0));

        Action::SetEqualHeightLayout(false)
            .process(&mut model)
            .unwrap();
        assert!(!model.tracks.equal_height_layout);
    }

    #[test]
    fn enabling_equal_height_layout_fits_tracks_for_shortcut_activation() {
        let mut model = Model::default();
        model.tracks.equal_height_layout = false;
        model.tracks.available_height = 80.0;
        let config = TrackConfig::default();
        let buffer = add_buffer(&mut model);
        let track_id = model
            .tracks
            .add_track_to_end(buffer, 48_000, &config)
            .unwrap();

        Action::SetEqualHeightLayout(true)
            .process(&mut model)
            .unwrap();

        assert!(model.tracks.equal_height_layout);
        assert_eq!(model.tracks.get_track_height(track_id), Some(80.0));
    }

    #[test]
    fn file_offset_result_updates_file_and_inherited_tracks() {
        let mut model = Model::default();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        let file_id = model.files.insert(file);
        let track_ids = buffers.map(|buffer_id| model.find_track_id_for_buffer(buffer_id).unwrap());
        model.set_track_use_file_offset(track_ids[1], false);
        model.set_track_sample_ix_offset(track_ids[1], 3.0);

        Action::OffsetDetected(jobs::OffsetDetectionResult {
            target: jobs::OffsetDetectionTarget::File { file_id },
            sample_ix_offset: Some(7),
        })
        .process(&mut model)
        .unwrap();
        Action::OffsetDetected(jobs::OffsetDetectionResult {
            target: jobs::OffsetDetectionTarget::File { file_id },
            sample_ix_offset: None,
        })
        .process(&mut model)
        .unwrap();

        assert_eq!(model.files[file_id].sample_ix_offset, 7);
        assert_eq!(
            model
                .tracks
                .get_track(track_ids[0])
                .unwrap()
                .single
                .sample_ix_offset,
            7.0
        );
        assert_eq!(
            model
                .tracks
                .get_track(track_ids[1])
                .unwrap()
                .single
                .sample_ix_offset,
            3.0
        );
    }

    #[test]
    fn track_offset_result_updates_detached_track_and_ignores_silence() {
        let mut model = Model::default();
        let buffer_id = add_buffer(&mut model);
        let track_id = model
            .tracks
            .add_track_to_end(buffer_id, 48_000, &TrackConfig::default())
            .unwrap();
        let target = jobs::OffsetDetectionTarget::Track {
            track_id,
            buffer_id,
        };

        Action::OffsetDetected(jobs::OffsetDetectionResult {
            target,
            sample_ix_offset: Some(5),
        })
        .process(&mut model)
        .unwrap();
        Action::OffsetDetected(jobs::OffsetDetectionResult {
            target,
            sample_ix_offset: None,
        })
        .process(&mut model)
        .unwrap();

        assert_eq!(
            model
                .tracks
                .get_track(track_id)
                .unwrap()
                .single
                .sample_ix_offset,
            5.0
        );
    }

    #[test]
    fn track_detection_is_unavailable_while_using_file_offset() {
        let mut model = Model::default();
        let buffer_id = add_buffer(&mut model);
        let file = make_file(&[buffer_id]);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        model.files.insert(file);
        let track_id = model.find_track_id_for_buffer(buffer_id).unwrap();

        Action::DetectTrackOffset {
            track_id,
            mode: jobs::OffsetDetectionMode::FirstNonZero,
        }
        .process(&mut model)
        .unwrap();

        assert_eq!(model.job_mgr.pending(), 0);
        assert_eq!(
            model
                .tracks
                .get_track(track_id)
                .unwrap()
                .single
                .sample_ix_offset,
            0.0
        );
    }

    #[test]
    fn offset_result_ignores_stale_track_and_file_targets() {
        let mut model = Model::default();
        let scanned_buffer = add_buffer(&mut model);
        let current_buffer = add_buffer(&mut model);
        let track_id = model
            .tracks
            .add_track_to_end(current_buffer, 48_000, &TrackConfig::default())
            .unwrap();

        Action::OffsetDetected(jobs::OffsetDetectionResult {
            target: jobs::OffsetDetectionTarget::Track {
                track_id,
                buffer_id: scanned_buffer,
            },
            sample_ix_offset: Some(7),
        })
        .process(&mut model)
        .unwrap();
        Action::OffsetDetected(jobs::OffsetDetectionResult {
            target: jobs::OffsetDetectionTarget::File {
                file_id: crate::wav::file::FileId::default(),
            },
            sample_ix_offset: Some(9),
        })
        .process(&mut model)
        .unwrap();

        assert_eq!(
            model
                .tracks
                .get_track(track_id)
                .unwrap()
                .single
                .sample_ix_offset,
            0.0
        );
    }

    #[test]
    fn manual_height_actions_disable_equal_height_layout() {
        let mut model = Model::default();
        let config = TrackConfig::default();
        let buffer_a = add_buffer(&mut model);
        let buffer_b = add_buffer(&mut model);
        let track_a = model
            .tracks
            .add_track_to_end(buffer_a, 48_000, &config)
            .unwrap();
        let track_b = model
            .tracks
            .add_track_to_end(buffer_b, 48_000, &config)
            .unwrap();

        Action::SetTrackHeight {
            track_id: track_a,
            height: 80.0,
        }
        .process(&mut model)
        .unwrap();
        assert!(!model.tracks.equal_height_layout);
        assert_eq!(model.tracks.get_track_height(track_a), Some(80.0));

        model.tracks.equal_height_layout = true;
        Action::SetTracksHeight { height: 90.0 }
            .process(&mut model)
            .unwrap();
        assert!(!model.tracks.equal_height_layout);
        assert_eq!(model.tracks.get_track_height(track_a), Some(90.0));
        assert_eq!(model.tracks.get_track_height(track_b), Some(90.0));
    }
}
