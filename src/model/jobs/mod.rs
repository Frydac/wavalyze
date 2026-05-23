//! Background-job bookkeeping.
//!
//! Producer side: a worker (`std::thread` natively, `rayon::spawn` on wasm) runs the actual work
//! and sends `JobEvent`s through an mpsc channel. If the job has a follow-up effect on the model
//! (e.g., integrating a loaded WAV), the worker pushes an `Action` through a separate
//! `Sender<Action>` it received at spawn time.
//!
//! Consumer side: `JobManager` (held by `Model`) drains the event channel each frame, updates
//! `JobSnapshot`s for in-flight jobs, and rotates terminated jobs into a small recency ring. It
//! knows nothing about specific job kinds — adding a new kind means writing a worker + an
//! `Action` variant, with no edits to `JobManager` itself.
//!
//! Per-kind code lives in submodules — see `demo` (synthetic CPU job) and `load_wav` (WAV file
//! loading). Workers shape their progress into a generic `JobProgress` (stage name, within-stage
//! counts, overall fraction) so the manager and UI don't need to know about kind specifics.

use std::collections::{BTreeMap, VecDeque};
use std::sync::mpsc::{Receiver, Sender};

pub mod demo;
pub mod load_wav;
pub use demo::{DemoTimedConfig, spawn_demo_timed_job};
pub use load_wav::spawn_load_wav_job;
#[cfg(not(target_arch = "wasm32"))]
pub use load_wav::spawn_load_wav_path_job;

pub type JobId = u64;
const RECENT_FINISHED_CAP: usize = 12;

// ---------------------------------------------------------------------------
// Public types — kinds, status, per-kind data
// ---------------------------------------------------------------------------

/// UI tag for categorizing in-flight jobs. Purely descriptive — `JobManager` does not branch on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    DemoTimed,
    LoadWav,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Running,
    Completed,
    Failed,
}

/// Generic progress representation for any background job. Workers compute the overall fraction
/// however they like (uniform across stages, weighted, whatever); the manager just surfaces what
/// the worker sends.
#[derive(Debug, Clone, PartialEq)]
pub struct JobProgress {
    /// Human-readable label for the current stage. May be empty for jobs without explicit stages.
    pub stage_name: String,
    /// Progress within the current stage as `(current, total)`. For atomic / stageless jobs,
    /// `(0, 0)` or `(1, 1)` is fine — `overall_fraction` is what consumers normalize from.
    pub stage_current: u64,
    pub stage_total: u64,
    /// Overall job completion, `[0.0, 1.0]`.
    pub overall_fraction: f32,
}

impl JobProgress {
    /// Sentinel "just started" value used by `start_job` before the first worker tick arrives.
    fn starting() -> Self {
        Self {
            stage_name: "starting".to_string(),
            stage_current: 0,
            stage_total: 0,
            overall_fraction: 0.0,
        }
    }
}

/// What the UI reads each frame for a single in-flight job. Mutated by `JobManager::drain_events`
/// in response to `JobEvent`s from the worker.
#[derive(Debug, Clone, PartialEq)]
pub struct JobSnapshot {
    pub job_id: JobId,
    pub kind: JobKind,
    pub label: String,
    pub status: JobStatus,
    pub progress: JobProgress,
    pub message: Option<String>,
}

/// Recency-ring entry. Holds the post-termination view of a job for the "Recent" UI list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedJob {
    pub job_id: JobId,
    pub kind: JobKind,
    pub label: String,
    pub status: JobStatus,
    pub summary: String,
}

// ---------------------------------------------------------------------------
// Worker → manager event channel
// ---------------------------------------------------------------------------

/// One message from a worker. Workers send many `Progress` events then exactly one terminal
/// (`Completed` or `Failed`).
#[derive(Debug)]
pub enum JobEvent {
    Progress(JobProgressEvent),
    Completed(JobCompletionEvent),
    Failed(JobFailureEvent),
}

/// Progress update from a worker. The `progress` field is the unified shape the manager copies
/// onto the snapshot; the worker is responsible for computing overall_fraction however suits its
/// work.
#[derive(Debug, Clone)]
pub struct JobProgressEvent {
    pub job_id: JobId,
    pub progress: JobProgress,
    pub message: Option<String>,
}

/// Completion notification. The worker is responsible for any side effects (e.g., pushing an
/// `Action`) before sending this; the summary is shown to the user in the "recent jobs" list.
#[derive(Debug, Clone)]
pub struct JobCompletionEvent {
    pub job_id: JobId,
    pub summary: String,
}

#[derive(Debug)]
pub struct JobFailureEvent {
    pub job_id: JobId,
    pub error: String,
}

// ---------------------------------------------------------------------------
// JobManager — pure bookkeeper. No knowledge of specific job kinds beyond the `JobKind` tag.
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct JobManager {
    tx: Sender<JobEvent>,
    rx: Receiver<JobEvent>,
    next_id: JobId,
    // BTreeMap keyed on monotonic JobId gives natural iteration in start order without an
    // explicit sort in `jobs()`.
    jobs: BTreeMap<JobId, JobSnapshot>,
    finished: VecDeque<FinishedJob>,
}

impl JobManager {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx,
            next_id: 1,
            jobs: BTreeMap::new(),
            finished: VecDeque::new(),
        }
    }

    pub fn sender(&self) -> Sender<JobEvent> {
        self.tx.clone()
    }

    /// Register a new in-flight job. Caller is expected to spawn the worker immediately after
    /// (see the spawners in `demo` / `load_wav`) and pass the `JobId` plus a clone of `sender()`
    /// to it.
    pub fn start_job(&mut self, kind: JobKind, label: impl Into<String>) -> JobId {
        let job_id = self.next_id;
        self.next_id += 1;
        let label = label.into();
        self.jobs.insert(
            job_id,
            JobSnapshot {
                job_id,
                kind,
                label,
                // No queue exists — work starts immediately after start_job. Reflect that here so
                // the UI doesn't show a stale "queued" state while the worker hasn't yet emitted a
                // first progress tick.
                status: JobStatus::Running,
                progress: JobProgress::starting(),
                message: None,
            },
        );
        job_id
    }

    pub fn pending(&self) -> usize {
        self.jobs.len()
    }

    pub fn jobs(&self) -> impl Iterator<Item = &JobSnapshot> + '_ {
        self.jobs.values()
    }

    pub fn recent_finished(&self) -> impl Iterator<Item = &FinishedJob> + '_ {
        self.finished.iter()
    }

    pub fn primary_job(&self) -> Option<&JobSnapshot> {
        self.jobs.values().next()
    }

    /// Drain queued events from workers. Updates active snapshots and moves terminated jobs into
    /// the recency ring. Returns true if any events were processed.
    pub fn drain_events(&mut self) -> bool {
        let mut had_events = false;
        loop {
            match self.rx.try_recv() {
                Ok(JobEvent::Progress(progress)) => {
                    had_events = true;
                    if let Some(job) = self.jobs.get_mut(&progress.job_id) {
                        job.status = JobStatus::Running;
                        job.progress = progress.progress;
                        job.message = progress.message;
                    }
                }
                Ok(JobEvent::Completed(completion)) => {
                    had_events = true;
                    let Some(job) = self.jobs.remove(&completion.job_id) else {
                        tracing::warn!(
                            "JobEvent::Completed for unknown job_id {}",
                            completion.job_id
                        );
                        continue;
                    };
                    self.push_finished(FinishedJob {
                        job_id: job.job_id,
                        kind: job.kind,
                        label: job.label,
                        status: JobStatus::Completed,
                        summary: completion.summary,
                    });
                }
                Ok(JobEvent::Failed(failure)) => {
                    had_events = true;
                    let Some(job) = self.jobs.remove(&failure.job_id) else {
                        tracing::warn!("JobEvent::Failed for unknown job_id {}", failure.job_id);
                        continue;
                    };
                    self.push_finished(FinishedJob {
                        job_id: job.job_id,
                        kind: job.kind,
                        label: job.label,
                        status: JobStatus::Failed,
                        summary: failure.error,
                    });
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                // Disconnect is unreachable: JobManager owns `tx`, so the channel cannot close
                // while the manager is alive. `try_recv` only returns Empty in practice.
                Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
            }
        }
        had_events
    }

    fn push_finished(&mut self, finished: FinishedJob) {
        self.finished.push_front(finished);
        while self.finished.len() > RECENT_FINISHED_CAP {
            self.finished.pop_back();
        }
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Worker spawn helper — picks the platform-appropriate spawn primitive. Private to the module;
// child submodules (`demo`, `load_wav`) reach it via `super::spawn_worker`.
// ---------------------------------------------------------------------------

fn spawn_worker(f: impl FnOnce() + Send + 'static) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(f);
    }
    #[cfg(target_arch = "wasm32")]
    {
        rayon::spawn(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_manager_tracks_progress_and_completion() {
        let mut manager = JobManager::new();
        let job_id = manager.start_job(JobKind::DemoTimed, "demo");
        let tx = manager.sender();
        tx.send(JobEvent::Progress(JobProgressEvent {
            job_id,
            progress: JobProgress {
                stage_name: "working".to_string(),
                stage_current: 1,
                stage_total: 3,
                overall_fraction: 1.0 / 3.0,
            },
            message: Some("hello".to_string()),
        }))
        .unwrap();
        assert!(manager.drain_events());
        let job = manager.primary_job().unwrap();
        assert_eq!(job.status, JobStatus::Running);
        assert_eq!(job.progress.stage_current, 1);

        tx.send(JobEvent::Completed(JobCompletionEvent {
            job_id,
            summary: "demo done".to_string(),
        }))
        .unwrap();
        assert!(manager.drain_events());
        assert_eq!(manager.pending(), 0);
        let finished: Vec<_> = manager.recent_finished().collect();
        assert_eq!(finished.len(), 1);
        assert_eq!(finished[0].status, JobStatus::Completed);
        assert_eq!(finished[0].summary, "demo done");
    }
}
