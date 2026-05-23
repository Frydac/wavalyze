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
        match wav::read::read_bytes_to_loaded_file_with_sink(&config, job_id, Some(&sink)) {
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
                    error: format!("Failed to load wav bytes: {error:#}"),
                }));
            }
        }
    });
}

// The sink is constructed inside `spawn_load_wav_job`'s worker closure, used only on that worker
// thread (the WAV reader is fully serial — no `par_iter`/`rayon::scope` internally), and dropped
// at end of read. `Cell` is sufficient; no synchronization is needed. `LoadProgressSink` has no
// `Send + Sync` bound (see `src/wav/read.rs:122`).
struct ThreadedLoadJobProgressSink {
    job_id: JobId,
    tx: Sender<JobEvent>,
    state: Cell<LoadJobProgress>,
}

impl ThreadedLoadJobProgressSink {
    fn new(job_id: JobId, tx: Sender<JobEvent>) -> Self {
        Self {
            job_id,
            tx,
            state: Cell::new(LoadJobProgress {
                stage: wav::read::LoadStage::Start,
                current: 0,
                total: 0,
            }),
        }
    }

    fn publish(&self, lp: LoadJobProgress) {
        let _ = self.tx.send(JobEvent::Progress(JobProgressEvent {
            job_id: self.job_id,
            progress: JobProgress {
                stage_name: lp.stage.label().to_string(),
                stage_current: lp.current,
                stage_total: lp.total,
                overall_fraction: lp.fraction(),
            },
            message: Some(format!(
                "{} ({}/{})",
                lp.stage.label(),
                lp.current,
                lp.total
            )),
        }));
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
