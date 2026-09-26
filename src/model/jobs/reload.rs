//! Prepare replacement audio for a reload batch without accessing the live model.
//!
//! Selected files pass through the usual WAV/thumbnail loader, followed by affected diffs in
//! dependency order. Inputs not selected for reload keep using their captured audio. Nothing
//! is installed here: the completion callback returns all replacements to the coordinator,
//! which validates the workspace and only then reports job success or failure.

use crate::model::jobs::{self, JobEvent, JobId};
use crate::{
    audio::{BufferId, buffer::BufferE, thumbnail::ThumbnailE},
    model::{
        files::disk::{Stamp, stamp},
        track::diff::Diff,
    },
    wav,
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::HashMap,
    sync::{Arc, mpsc::Sender},
};

/// A disk read recipe and the old buffer IDs its selected channels will replace.
#[derive(Debug)]
pub(crate) struct ReloadSourceInput {
    pub(crate) config: wav::ReadConfig,
    pub(crate) channels: wav::file::Channels,
}

/// Captured computation inputs. No live file collection or track collection is borrowed.
#[derive(Debug)]
pub(crate) struct ReloadJobInput {
    pub(crate) sources: Vec<ReloadSourceInput>,
    /// Already ordered so intermediate diffs are computed before the diffs using them.
    pub(crate) diffs: Vec<(BufferId, Diff)>,
    pub(crate) buffers: HashMap<BufferId, Arc<BufferE>>,
}

/// Complete replacement set, in the same source/diff order as the job input.
#[derive(Debug)]
pub(crate) struct PreparedReload {
    pub(crate) sources: Vec<PreparedSource>,
    pub(crate) diffs: Vec<PreparedDiff>,
}

/// Successfully read replacement audio for one requested file.
/// Channel buffers have been moved out of `loaded.channels` into `channels`, so the worker can
/// share them with diff computations without copying samples. `loaded` retains the metadata
/// and thumbnails needed when the UI installs those buffers.
#[derive(Debug)]
pub(crate) struct PreparedSource {
    pub(crate) loaded: wav::LoadedFile,
    pub(crate) channels: Vec<(wav::ChIx, Arc<BufferE>)>,
    pub(crate) stamp: Stamp,
}
/// Replacement diff audio, its thumbnail, and the sample offset used to display that audio.
#[derive(Debug)]
pub(crate) struct PreparedDiff {
    pub(crate) buffer: Arc<BufferE>,
    pub(crate) thumbnail: ThumbnailE,
    pub(crate) offset: i64,
}

/// Spawn computation and deliver its result without prematurely completing the job.
/// The caller retains workspace validation state in the callback, outside the worker input.
pub(crate) fn spawn_reload_job(
    job_id: JobId,
    input: ReloadJobInput,
    events: Sender<JobEvent>,
    completed: impl FnOnce(Result<PreparedReload, String>) + Send + 'static,
) {
    jobs::spawn_worker(move || {
        let result = prepare(job_id, &input, &events).map_err(|error| format!("{error:#}"));
        completed(result);
    });
}

/// Read all selected files, then recompute their dependent diffs using the captured alignment.
/// This runs on the worker and only changes its local buffer map. Output vectors follow the
/// input's source/diff order so the UI can match each replacement to its original target.
pub(crate) fn prepare(
    job_id: JobId,
    input: &ReloadJobInput,
    events: &std::sync::mpsc::Sender<JobEvent>,
) -> Result<PreparedReload> {
    // Keep the old buffer IDs as lookup keys while substituting new audio locally. Existing
    // diff recipes can then resolve both new selected inputs and unchanged unselected inputs.
    // New model buffer IDs are assigned later, only if the entire batch can be committed.
    let mut buffers = input.buffers.clone();
    let mut sources = Vec::new();
    let count = (input.sources.len() + input.diffs.len()) as f32;
    for (index, source) in input.sources.iter().enumerate() {
        let before = stamp(&source.config.filepath)?;
        let recipe = &source.config;
        let sink = jobs::load_wav::ThreadedLoadJobProgressSink::new_mapped(
            job_id,
            events.clone(),
            Some(source.config.filepath.display().to_string()),
            index as f32 / count,
            (index + 1) as f32 / count,
        );
        let channel_count = wav::read::peek_nr_channels(&source.config.filepath)?;
        ensure!(
            source.channels.keys().all(|ch| *ch < channel_count),
            "{} no longer contains all previously loaded channels",
            source.config.filepath.display()
        );
        let mut loaded = jobs::load_wav::load_wav_path_for_job(job_id, recipe, &sink)
            .with_context(|| format!("Cannot reload {}", source.config.filepath.display()))?;
        ensure!(
            before == stamp(&source.config.filepath)?,
            "{} changed during reading",
            source.config.filepath.display()
        );
        let mut channels = Vec::new();
        for (ch, channel) in &source.channels {
            let buffer = Arc::new(
                loaded
                    .channels
                    .remove(ch)
                    .context("A loaded channel is missing")?,
            );
            buffers.insert(channel.buffer_id, buffer.clone());
            channels.push((*ch, buffer));
        }
        sources.push(PreparedSource {
            loaded,
            channels,
            stamp: before,
        });
    }
    // The input already orders these by dependency. Store each computed output in the
    // local map so a later diff of that diff reads its new audio, not the captured old output.
    let mut diffs = Vec::new();
    for (index, (_, diff)) in input.diffs.iter().enumerate() {
        let a = buffers
            .get(&diff.buffer_id_a)
            .context("Missing diff input")?;
        let b = buffers
            .get(&diff.buffer_id_b)
            .context("Missing diff input")?;
        let computed = jobs::diff::compute_diff_buffer(
            a,
            b,
            diff.sample_ix_offset_a,
            diff.sample_ix_offset_b,
        )?;
        let thumbnail = ThumbnailE::from_buffer_e(&computed.buffer, None);
        let buffer = Arc::new(computed.buffer);
        buffers.insert(diff.buffer_id_diff, buffer.clone());
        diffs.push(PreparedDiff {
            buffer,
            thumbnail,
            offset: computed.sample_ix_offset_diff,
        });
        let done = input.sources.len() + index + 1;
        let _ = events.send(JobEvent::Progress(jobs::JobProgressEvent {
            job_id,
            progress: jobs::JobProgress {
                stage_name: "Recomputing diffs".into(),
                stage_current: done as u64,
                stage_total: count as u64,
                overall_fraction: done as f32 / count,
            },
            message: None,
        }));
    }
    Ok(PreparedReload { sources, diffs })
}
