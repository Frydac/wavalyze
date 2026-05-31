//! WAV-load background job: spawner + progress sink.
//!
//! Decodes WAV bytes on a worker thread. The sink converts the WAV reader's stage-aware
//! `LoadProgressSink` callbacks into generic `JobProgress` events (with stage label, within-stage
//! counts, and overall fraction). On success the loaded file is forwarded to the model via
//! `Action::IntegrateLoadedFile`, then a `Completed` event closes the job out so the recent-
//! finished list updates.

use std::cell::Cell;
use std::sync::mpsc::Sender;

use crate::model::Action;
use crate::wav::{self, read::LoadProgressSink};

use super::{
    JobCompletionEvent, JobEvent, JobFailureEvent, JobId, JobProgress, JobProgressEvent,
    spawn_worker,
};

/// Internal sink state — the WAV reader speaks `LoadStage` + (current, total), and we hold the
/// latest such triple so `set_current` can re-publish with the existing stage. Translated into a
/// generic `JobProgress` at publish time; no caller outside this module sees this type.
#[derive(Debug, Clone, Copy)]
struct LoadJobProgress {
    stage: wav::read::LoadStage,
    current: u64,
    total: u64,
}

impl LoadJobProgress {
    /// Unit fraction `[0.0, 1.0]` representing overall load completion across all stages.
    fn fraction(&self) -> f32 {
        self.stage.overall_fraction(self.current, self.total)
    }
}

/// Load a WAV from in-memory bytes on a worker thread. On success the loaded file is forwarded
/// to the model as `Action::IntegrateLoadedFile` (via `actions_tx`), then a `Completed` event is
/// emitted on the job channel so the recent-finished list updates.
pub fn spawn_load_wav_job(
    job_id: JobId,
    config: wav::ReadConfigBytes,
    events_tx: Sender<JobEvent>,
    actions_tx: Sender<Action>,
) {
    spawn_worker(move || {
        let sink = ThreadedLoadJobProgressSink::new(job_id, events_tx.clone());
        let result = load_wav_bytes_for_job(job_id, &config, &sink);
        finish_load_wav_job(job_id, result, &events_tx, &actions_tx);
    });
}

/// Load a WAV from a filesystem path on a worker thread. Native-only — wasm has no disk paths.
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_load_wav_path_job(
    job_id: JobId,
    config: wav::ReadConfig,
    events_tx: Sender<JobEvent>,
    actions_tx: Sender<Action>,
) {
    spawn_worker(move || {
        let sink = ThreadedLoadJobProgressSink::new(job_id, events_tx.clone());
        let result = load_wav_path_for_job(job_id, &config, &sink);
        finish_load_wav_job(job_id, result, &events_tx, &actions_tx);
    });
}

pub fn load_wav_bytes_for_job(
    job_id: JobId,
    config: &wav::ReadConfigBytes,
    sink: &dyn LoadProgressSink,
) -> anyhow::Result<wav::read::LoadedFile> {
    // Shared by normal file-open jobs and composite jobs (for example CLI diff) so they all get
    // the same decoder + thumbnail stages and progress labels.
    wav::read::read_bytes_to_loaded_file_with_sink(config, job_id, Some(sink)).map(|mut loaded| {
        build_thumbnails_in_worker(&mut loaded, sink);
        loaded
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_wav_path_for_job(
    job_id: JobId,
    config: &wav::ReadConfig,
    sink: &dyn LoadProgressSink,
) -> anyhow::Result<wav::read::LoadedFile> {
    // Keep path loading factored out of the spawner so parent jobs can reuse the exact same load
    // pipeline without spawning child jobs or integrating intermediate files.
    wav::read::read_path_to_loaded_file_with_sink(config, job_id, Some(sink)).map(|mut loaded| {
        build_thumbnails_in_worker(&mut loaded, sink);
        loaded
    })
}

/// Build per-channel thumbnails on the worker thread, emitting `LoadStage::Thumbnail` progress
/// (one tick per channel completed). Moves the cost off the UI thread that integrates the loaded
/// file via `Action::IntegrateLoadedFile`.
fn build_thumbnails_in_worker(loaded: &mut wav::read::LoadedFile, sink: &dyn LoadProgressSink) {
    let total = loaded.channels.len() as u64;
    sink.set_stage(wav::read::LoadStage::Thumbnail, total);
    let mut built = 0u64;
    for (&ch_ix, buffer) in &loaded.channels {
        let thumbnail = crate::audio::thumbnail::ThumbnailE::from_buffer_e(buffer, None);
        loaded.thumbnails.insert(ch_ix, thumbnail);
        built += 1;
        sink.set_current(built);
    }
}

fn finish_load_wav_job(
    job_id: JobId,
    result: anyhow::Result<wav::read::LoadedFile>,
    events_tx: &Sender<JobEvent>,
    actions_tx: &Sender<Action>,
) {
    match result {
        Ok(loaded) => {
            let summary = format!(
                "Loaded {} channels from {}",
                loaded.channels.len(),
                loaded
                    .path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or("file")
            );
            let _ = actions_tx.send(Action::IntegrateLoadedFile(loaded));
            let _ = events_tx.send(JobEvent::Completed(JobCompletionEvent { job_id, summary }));
        }
        Err(error) => {
            let _ = events_tx.send(JobEvent::Failed(JobFailureEvent {
                job_id,
                error: format!("Failed to load wav: {error:#}"),
            }));
        }
    }
}

// The sink is constructed inside `spawn_load_wav_job`'s worker closure, used only on that worker
// thread (the WAV reader is fully serial — no `par_iter`/`rayon::scope` internally), and dropped
// at end of read. `Cell` is sufficient; no synchronization is needed. `LoadProgressSink` has no
// `Send + Sync` bound (see `src/wav/read.rs:122`).
pub struct ThreadedLoadJobProgressSink {
    job_id: JobId,
    tx: Sender<JobEvent>,
    stage_prefix: Option<String>,
    overall_start: f32,
    overall_end: f32,
    state: Cell<LoadJobProgress>,
}

impl ThreadedLoadJobProgressSink {
    pub fn new(job_id: JobId, tx: Sender<JobEvent>) -> Self {
        Self::new_mapped(job_id, tx, None, 0.0, 1.0)
    }

    pub fn new_mapped(
        job_id: JobId,
        tx: Sender<JobEvent>,
        stage_prefix: Option<String>,
        overall_start: f32,
        overall_end: f32,
    ) -> Self {
        Self {
            job_id,
            tx,
            stage_prefix,
            overall_start,
            overall_end,
            state: Cell::new(LoadJobProgress {
                stage: wav::read::LoadStage::Start,
                current: 0,
                total: 0,
            }),
        }
    }

    fn publish(&self, lp: LoadJobProgress) {
        let stage_name = self.stage_name(lp.stage);
        // Composite jobs map the WAV loader's own 0..1 progress into one slice of the parent job
        // while preserving the loader's detailed stage names.
        let overall_fraction =
            self.overall_start + lp.fraction() * (self.overall_end - self.overall_start).max(0.0);
        let _ = self.tx.send(JobEvent::Progress(JobProgressEvent {
            job_id: self.job_id,
            progress: JobProgress {
                stage_name: stage_name.clone(),
                stage_current: lp.current,
                stage_total: lp.total,
                overall_fraction,
            },
            message: Some(format!("{} ({}/{})", stage_name, lp.current, lp.total)),
        }));
    }

    fn stage_name(&self, stage: wav::read::LoadStage) -> String {
        match &self.stage_prefix {
            Some(prefix) => format!("{prefix}: {}", stage.label()),
            None => stage.label().to_string(),
        }
    }
}

impl LoadProgressSink for ThreadedLoadJobProgressSink {
    fn set_stage(&self, stage: wav::read::LoadStage, total: u64) {
        let progress = LoadJobProgress {
            stage,
            current: 0,
            total,
        };
        self.state.set(progress);
        self.publish(progress);
    }

    fn set_current(&self, current: u64) {
        let mut progress = self.state.get();
        progress.current = current;
        self.state.set(progress);
        self.publish(progress);
    }
}
