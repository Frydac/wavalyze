//! Reload changed files and their dependent diffs without showing a partly updated workspace.
//!
//! The UI first captures the selected files, their current audio, and the diffs that need to be
//! redone. A worker reads the files and computes replacement audio using the existing load and
//! diff helpers. The old audio remains available while that work runs.
//!
//! When the worker returns, the UI checks whether the files or relevant workspace state changed
//! in the meantime. If so, it keeps the old audio and leaves the files marked for retry. Otherwise
//! it installs all replacements together, preserving file/track IDs, layout, and alignment.
//!
//! Filesystem event handling lives in `monitoring`; background preparation lives in
//! `jobs::reload`. This module coordinates the request and applies the validated result.
use crate::model::tracks::dependencies::affected_diffs;
use crate::model::{
    Action, Model,
    jobs::{self, JobCompletionEvent, JobEvent, JobFailureEvent, JobId},
    track::{TrackId, diff::Diff},
};
use crate::model::{
    files::disk::stamp,
    jobs::reload::{PreparedReload, PreparedSource, ReloadJobInput, ReloadSourceInput},
};
use crate::{
    audio::BufferId,
    wav::file::{File, FileId},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

/// State of the user-requested update operation, separate from filesystem watching.
/// The coordinator sets this when starting a batch and clears it after handling its result.
#[derive(Debug, Default)]
pub(crate) struct ReloadState {
    pub(crate) active_job: Option<JobId>,
}

/// One Files-list entry selected for this update, as it existed when the user clicked Update.
/// Its old channel mapping identifies which live buffers the new samples will replace.
#[derive(Debug)]
struct Source {
    id: FileId,
    file: File,
    path: PathBuf,
    revision: u64,
}
/// Workspace state captured before starting a reload, used to validate and apply its result.
/// Audio snapshots belong to the separate job input and do not travel back with this request.
#[derive(Debug)]
struct ReloadRequest {
    job_id: JobId,
    generation: u64,
    sources: Vec<Source>,
    /// All diff definitions guard against dependencies added or removed while preparing.
    topology: Vec<(BufferId, Diff)>,
    /// Diffs to recompute, ordered so each intermediate result is ready before its users.
    affected: Vec<(BufferId, Diff)>,
}
/// Worker reply containing either the complete replacement set or an error.
/// The original request travels back too, so the UI can check it against the current workspace.
/// A successful worker reply does not yet mean the update has been applied.
#[derive(Debug)]
pub struct ReloadResult {
    request: ReloadRequest,
    result: Result<PreparedReload, String>,
}

impl ReloadRequest {
    /// Translate validation snapshots into only the data needed for worker computation.
    fn job_input(&self, audio: &crate::audio::manager::AudioManager) -> ReloadJobInput {
        ReloadJobInput {
            sources: self
                .sources
                .iter()
                .map(|source| {
                    let mut config = *source
                        .file
                        .source
                        .clone()
                        .expect("reload source has a disk recipe");
                    config.filepath = source.path.clone();
                    config.ch_ixs = Some(source.file.channels.keys().copied().collect());
                    config.sample_ix_offset = source.file.sample_ix_offset;
                    ReloadSourceInput {
                        config,
                        channels: source.file.channels.clone(),
                    }
                })
                .collect(),
            diffs: self.affected.clone(),
            buffers: audio
                .buffers
                .iter()
                .map(|(id, buffer)| (id, buffer.clone()))
                .collect(),
        }
    }
}

impl Model {
    /// List source and diff tracks affected by these files, for the notification hover preview.
    /// Hidden tracks are included; a removed intermediate diff is recomputed but has no UI row.
    pub fn affected_reload_tracks(&self, ids: &[FileId]) -> Vec<TrackId> {
        let changed: HashSet<_> = ids
            .iter()
            .filter_map(|id| self.files.get(*id))
            .flat_map(|f| f.channels.values().map(|c| c.buffer_id))
            .collect();
        self.tracks.affected_tracks_for_buffers(&changed)
    }
    /// Capture the selected files and start preparing their replacements on a worker.
    /// Only one batch runs at a time. No live audio changes until its result passes validation.
    pub fn start_reload(&mut self, ids: Vec<FileId>) -> Result<()> {
        if self.files.reload.active_job.is_some() {
            return Ok(());
        }
        self.files.changes.drain();
        let mut sources = Vec::new();
        let mut seen = HashSet::new();
        for id in ids {
            if !seen.insert(id) {
                continue;
            }
            let file = self.files.get(id).context("File was closed")?.clone();
            let recipe = file
                .source
                .as_ref()
                .context("File has no filesystem source")?;
            let path = crate::model::files::disk::absolute_path(&recipe.filepath);
            let change = self
                .files
                .changes
                .paths
                .get(&path)
                .context("File is not monitored")?;
            if !change.changed {
                continue;
            }
            ensure!(change.ready(), "File is still changing; retry shortly");
            sources.push(Source {
                id,
                file,
                path,
                revision: change.revision,
            });
        }
        if sources.is_empty() {
            return Ok(());
        }
        let changed = sources
            .iter()
            .flat_map(|s| s.file.channels.values().map(|c| c.buffer_id))
            .collect();
        let topology = self.tracks.diff_topology();
        let affected = affected_diffs(&topology, &changed);
        let job_id = self.job_mgr.start_job(
            jobs::JobKind::Reload,
            format!("Update {} file(s)", sources.len()),
        );
        self.files.reload.active_job = Some(job_id);
        for source in &sources {
            self.files
                .changes
                .paths
                .get_mut(&source.path)
                .unwrap()
                .error = None;
        }
        let request = ReloadRequest {
            job_id,
            generation: self.generation(),
            sources,
            topology,
            affected,
        };
        let actions = self.actions_tx.clone();
        let events = self.job_mgr.sender();
        jobs::reload::spawn_reload_job(
            request.job_id,
            request.job_input(&self.audio),
            events,
            move |result| {
                let _ = actions.send(Action::FinishReload(Box::new(ReloadResult {
                    request,
                    result,
                })));
            },
        );
        Ok(())
    }
    /// Handle a worker reply: apply the whole update or keep the old audio and display an error.
    /// Finish the job here, after validation, so worker success alone cannot report a successful update.
    pub fn finish_reload(&mut self, result: ReloadResult) {
        let ReloadResult { request, result } = result;
        self.files.changes.drain();
        let outcome = result.map_err(anyhow::Error::msg).and_then(|prepared| {
            self.validate_reload(&request, &prepared.sources)?;
            self.commit_reload(&request, prepared);
            Ok(())
        });
        if self.files.reload.active_job == Some(request.job_id) {
            self.files.reload.active_job = None;
        }
        let event = match outcome {
            Ok(()) => JobEvent::Completed(JobCompletionEvent {
                job_id: request.job_id,
                summary: "Updated files and associated diffs".into(),
            }),
            Err(error) => {
                let error = format!("Update kept previous data: {error:#}");
                for source in &request.sources {
                    if self.files.contains_key(source.id)
                        && let Some(change) = self.files.changes.paths.get_mut(&source.path)
                    {
                        change.error = Some(error.clone());
                        change.changed = true;
                    }
                }
                JobEvent::Failed(JobFailureEvent {
                    job_id: request.job_id,
                    error,
                })
            }
        };
        let _ = self.job_mgr.sender().send(event);
    }
    /// Check that the request still describes the workspace and disk files we are about to update.
    /// This must succeed before committing any replacement; a mismatch leaves the whole batch unchanged.
    fn validate_reload(&self, request: &ReloadRequest, prepared: &[PreparedSource]) -> Result<()> {
        ensure!(
            self.is_current_generation(request.generation),
            "Workspace was cleared"
        );
        ensure!(
            self.tracks.diff_topology() == request.topology,
            "Diff dependencies changed; retry"
        );
        for (source, prepared) in request.sources.iter().zip(prepared) {
            ensure!(
                self.files.get(source.id) == Some(&source.file),
                "File or offset changed; retry"
            );
            let change = self
                .files
                .changes
                .paths
                .get(&source.path)
                .context("File was closed")?;
            ensure!(
                change.revision == source.revision && stamp(&source.path)? == prepared.stamp,
                "{} changed while updating; retry",
                source.path.display()
            );
        }
        for (_, diff) in &request.affected {
            ensure!(
                self.audio.buffers.contains_key(diff.buffer_id_a)
                    && self.audio.buffers.contains_key(diff.buffer_id_b),
                "Diff source was closed"
            );
        }
        Ok(())
    }
    /// Install a fully prepared and validated batch while retaining existing files and tracks.
    /// Allocate replacement buffers first, redirect their users, then remove the old audio and statistics.
    fn commit_reload(&mut self, request: &ReloadRequest, prepared: PreparedReload) {
        // All fallible work and validation precede this point. Fresh IDs make old async results
        // harmless: their buffers disappear instead of silently referring to different audio.
        let mut replacements = HashMap::new();
        for (source, prepared) in request.sources.iter().zip(prepared.sources) {
            let mut loaded = prepared.loaded;
            let file = self.files.get_mut(source.id).unwrap();
            file.total_nr_channels = loaded.total_nr_channels;
            file.sample_type = loaded.sample_type;
            file.bit_depth = loaded.bit_depth;
            file.sample_rate = loaded.sample_rate;
            file.nr_samples = loaded.nr_samples;
            file.layout = loaded.layout;
            for (ch, buffer) in prepared.channels {
                let old = file.channels.get_mut(&ch).unwrap();
                let id = self.audio.buffers.insert(buffer);
                self.audio
                    .thumbnails
                    .insert(id, loaded.thumbnails.remove(&ch).unwrap());
                replacements.insert(old.buffer_id, id);
                old.buffer_id = id;
            }
        }
        for ((_, diff), prepared) in request.affected.iter().zip(prepared.diffs) {
            let id = self.audio.buffers.insert(prepared.buffer);
            self.audio.thumbnails.insert(id, prepared.thumbnail);
            replacements.insert(diff.buffer_id_diff, id);
            // Input alignment is unchanged, so the generated timeline origin is unchanged too.
            debug_assert_eq!(diff.sample_ix_offset_diff, prepared.offset);
        }
        self.tracks
            .apply_buffer_replacements(&replacements, &self.audio);
        for old in replacements.keys() {
            self.audio.remove_buffer(*old);
        }
        for source in &request.sources {
            self.files.acknowledge_reload(source.id, source.revision);
        }
    }
}

#[cfg(test)]
#[path = "reload_tests.rs"]
mod tests;
