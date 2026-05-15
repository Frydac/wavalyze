use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use serde::{Deserialize, Serialize};

use crate::audio::{SampleType, buffer::BufferE, channel::Layout};
use crate::wav::{self, read::LoadProgressSink};

pub type JobId = u64;
const JOB_PROGRESS_TOTAL: u64 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    DemoTimed,
    LoadWav,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JobResultData {
    DemoTimed(DemoTimedSummary),
    LoadWav(TransferLoadedFile),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemoTimedConfig {
    pub stage_count: u32,
    pub work_units: u32,
}

impl Default for DemoTimedConfig {
    fn default() -> Self {
        Self {
            stage_count: 4,
            work_units: 5_000_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoTimedSummary {
    pub stage_count: u32,
    pub checksum: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadJobProgress {
    pub stage: wav::read::LoadStage,
    pub current: u64,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobProgressData {
    Units { current: u64, total: u64 },
    Load(LoadJobProgress),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobSnapshot {
    pub job_id: JobId,
    pub kind: JobKind,
    pub label: String,
    pub status: JobStatus,
    pub stage_label: String,
    pub current: u64,
    pub total: u64,
    pub message: Option<String>,
    pub load_progress: Option<LoadJobProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedJob {
    pub job_id: JobId,
    pub kind: JobKind,
    pub label: String,
    pub status: JobStatus,
    pub summary: String,
}

#[derive(Debug)]
pub enum JobEvent {
    Progress(JobProgressEvent),
    Completed(JobCompletionEvent),
    Failed(JobFailureEvent),
}

#[derive(Debug, Clone)]
pub struct JobProgressEvent {
    pub job_id: JobId,
    pub stage_label: String,
    pub progress: JobProgressData,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JobCompletionEvent {
    pub job_id: JobId,
    pub result: JobResultData,
}

#[derive(Debug)]
pub struct JobFailureEvent {
    pub job_id: JobId,
    pub error: String,
}

#[derive(Debug)]
pub struct JobManager {
    tx: Sender<JobEvent>,
    rx: Receiver<JobEvent>,
    next_id: JobId,
    jobs: HashMap<JobId, JobSnapshot>,
    finished: VecDeque<FinishedJob>,
    completed: VecDeque<JobCompletionEvent>,
}

impl JobManager {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx,
            next_id: 1,
            jobs: HashMap::new(),
            finished: VecDeque::new(),
            completed: VecDeque::new(),
        }
    }

    pub fn sender(&self) -> Sender<JobEvent> {
        self.tx.clone()
    }

    pub fn start_job(&mut self, kind: JobKind, label: impl Into<String>) -> JobId {
        let job_id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let label = label.into();
        self.jobs.insert(
            job_id,
            JobSnapshot {
                job_id,
                kind,
                label,
                status: JobStatus::Queued,
                stage_label: "queued".to_string(),
                current: 0,
                total: 0,
                message: None,
                load_progress: None,
            },
        );
        job_id
    }

    pub fn pending(&self) -> usize {
        self.jobs.len()
    }

    pub fn jobs(&self) -> Vec<&JobSnapshot> {
        let mut jobs = self.jobs.values().collect::<Vec<_>>();
        jobs.sort_by_key(|job| job.job_id);
        jobs
    }

    pub fn recent_finished(&self) -> Vec<&FinishedJob> {
        self.finished.iter().collect()
    }

    pub fn drain_completed(&mut self) -> Vec<JobCompletionEvent> {
        self.completed.drain(..).collect()
    }

    pub fn primary_job(&self) -> Option<&JobSnapshot> {
        self.jobs().into_iter().next()
    }

    pub fn drain_events(&mut self) -> bool {
        let mut had_events = false;
        loop {
            match self.rx.try_recv() {
                Ok(JobEvent::Progress(progress)) => {
                    had_events = true;
                    if let Some(job) = self.jobs.get_mut(&progress.job_id) {
                        job.status = JobStatus::Running;
                        job.stage_label = progress.stage_label;
                        match progress.progress {
                            JobProgressData::Units { current, total } => {
                                job.current = current;
                                job.total = total;
                                job.load_progress = None;
                            }
                            JobProgressData::Load(load_progress) => {
                                job.current = (load_progress
                                    .stage
                                    .overall_fraction(load_progress.current, load_progress.total)
                                    * JOB_PROGRESS_TOTAL as f32)
                                    .round() as u64;
                                job.total = JOB_PROGRESS_TOTAL;
                                job.load_progress = Some(load_progress);
                            }
                        }
                        job.message = progress.message;
                    }
                }
                Ok(JobEvent::Completed(completion)) => {
                    had_events = true;
                    let Some(job) = self.jobs.remove(&completion.job_id) else {
                        continue;
                    };
                    let summary = format_result(&job.label, &completion.result);
                    self.push_finished(FinishedJob {
                        job_id: job.job_id,
                        kind: job.kind,
                        label: job.label,
                        status: JobStatus::Completed,
                        summary,
                    });
                    self.completed.push_back(completion);
                }
                Ok(JobEvent::Failed(failure)) => {
                    had_events = true;
                    let Some(job) = self.jobs.remove(&failure.job_id) else {
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
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    tracing::error!("Job event channel disconnected");
                    break;
                }
            }
        }
        had_events
    }

    fn push_finished(&mut self, finished: FinishedJob) {
        self.finished.push_front(finished);
        while self.finished.len() > 12 {
            self.finished.pop_back();
        }
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransferLoadedFile {
    pub load_id: wav::read::LoadId,
    pub channels: BTreeMap<wav::read::ChIx, BufferE>,
    pub sample_type: SampleType,
    pub bit_depth: u16,
    pub sample_rate: u32,
    pub layout_bits: Option<u64>,
    pub path: Option<PathBuf>,
    pub nr_samples: u64,
}

impl From<wav::read::LoadedFile> for TransferLoadedFile {
    fn from(value: wav::read::LoadedFile) -> Self {
        Self {
            load_id: value.load_id,
            channels: value.channels,
            sample_type: value.sample_type,
            bit_depth: value.bit_depth,
            sample_rate: value.sample_rate,
            layout_bits: value.layout.map(|layout| layout.bits()),
            path: value.path,
            nr_samples: value.nr_samples,
        }
    }
}

impl From<TransferLoadedFile> for wav::read::LoadedFile {
    fn from(value: TransferLoadedFile) -> Self {
        Self {
            load_id: value.load_id,
            channels: value.channels,
            sample_type: value.sample_type,
            bit_depth: value.bit_depth,
            sample_rate: value.sample_rate,
            layout: value.layout_bits.map(Layout::from_bits_retain),
            path: value.path,
            nr_samples: value.nr_samples,
        }
    }
}

fn format_result(label: &str, result: &JobResultData) -> String {
    match result {
        JobResultData::DemoTimed(summary) => {
            format!(
                "{} stages complete, checksum {}",
                summary.stage_count, summary.checksum
            )
        }
        JobResultData::LoadWav(loaded) => {
            format!(
                "Loaded {} channels from {}",
                loaded.channels.len(),
                loaded
                    .path
                    .as_ref()
                    .and_then(|path| path.file_name())
                    .and_then(|name| name.to_str())
                    .unwrap_or(label)
            )
        }
    }
}

pub fn spawn_demo_timed_job(job_id: JobId, config: DemoTimedConfig, tx: Sender<JobEvent>) {
    #[cfg(not(target_arch = "wasm32"))]
    spawn_demo_timed_job_native(job_id, config, tx);

    #[cfg(target_arch = "wasm32")]
    spawn_demo_timed_job_wasm(job_id, config, tx);
}

pub fn spawn_load_wav_job(job_id: JobId, config: wav::ReadConfigBytes, tx: Sender<JobEvent>) {
    #[cfg(not(target_arch = "wasm32"))]
    spawn_load_wav_job_native(job_id, config, tx);

    #[cfg(target_arch = "wasm32")]
    spawn_load_wav_job_wasm(job_id, config, tx);
}

fn demo_stage_name(stage_ix: u32, total: u32) -> String {
    match (stage_ix, total) {
        (0, _) => "preparing".to_string(),
        (ix, total) if ix + 1 == total => "finalizing".to_string(),
        _ => format!("processing {}", stage_ix + 1),
    }
}

fn run_demo_iterations(mut value: u64, stage_ix: u32, start_ix: u32, end_ix: u32) -> u64 {
    for ix in start_ix..end_ix {
        let ix = ix as u64;
        value = value
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223)
            .wrapping_add(ix.rotate_left((stage_ix % 13) + 1));
        value ^= value >> 17;
        value = value.rotate_left(((ix % 23) + 1) as u32);
    }
    value
}

fn compute_demo_stage(stage_ix: u32, work_units: u32, seed: u64) -> u64 {
    let iterations = work_units.max(10_000);
    let initial = seed ^ ((stage_ix as u64) << 32);
    run_demo_iterations(initial, stage_ix, 0, iterations)
}

fn run_demo_timed_job(job_id: JobId, config: DemoTimedConfig, tx: Sender<JobEvent>) {
    let mut checksum = 0_u64;
    for stage_ix in 0..config.stage_count {
        let stage_label = demo_stage_name(stage_ix, config.stage_count);
        let _ = tx.send(JobEvent::Progress(JobProgressEvent {
            job_id,
            stage_label: stage_label.clone(),
            progress: JobProgressData::Units {
                current: stage_ix as u64,
                total: config.stage_count as u64,
            },
            message: Some(format!("running {stage_label}")),
        }));

        checksum ^= compute_demo_stage(stage_ix, config.work_units, job_id);

        let _ = tx.send(JobEvent::Progress(JobProgressEvent {
            job_id,
            stage_label,
            progress: JobProgressData::Units {
                current: (stage_ix + 1) as u64,
                total: config.stage_count as u64,
            },
            message: Some(format!("completed stage {}", stage_ix + 1)),
        }));
    }

    let _ = tx.send(JobEvent::Completed(JobCompletionEvent {
        job_id,
        result: JobResultData::DemoTimed(DemoTimedSummary {
            stage_count: config.stage_count,
            checksum,
        }),
    }));
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_demo_timed_job_native(job_id: JobId, config: DemoTimedConfig, tx: Sender<JobEvent>) {
    std::thread::spawn(move || run_demo_timed_job(job_id, config, tx));
}

#[cfg(target_arch = "wasm32")]
fn spawn_demo_timed_job_wasm(job_id: JobId, config: DemoTimedConfig, tx: Sender<JobEvent>) {
    rayon::spawn(move || run_demo_timed_job(job_id, config, tx));
}

struct ThreadedLoadJobProgressSink {
    job_id: JobId,
    tx: Sender<JobEvent>,
    state: std::sync::Mutex<LoadJobProgress>,
}

impl ThreadedLoadJobProgressSink {
    fn new(job_id: JobId, tx: Sender<JobEvent>) -> Self {
        Self {
            job_id,
            tx,
            state: std::sync::Mutex::new(LoadJobProgress {
                stage: wav::read::LoadStage::Start,
                current: 0,
                total: 0,
            }),
        }
    }

    fn publish(&self, progress: LoadJobProgress) {
        let _ = self.tx.send(JobEvent::Progress(JobProgressEvent {
            job_id: self.job_id,
            stage_label: progress.stage.label().to_string(),
            progress: JobProgressData::Load(progress),
            message: Some(format!(
                "{} ({}/{})",
                progress.stage.label(),
                progress.current,
                progress.total
            )),
        }));
    }
}

impl LoadProgressSink for ThreadedLoadJobProgressSink {
    fn set_stage(&self, stage: wav::read::LoadStage, total: u64) {
        let mut state = self.state.lock().expect("progress lock poisoned");
        state.stage = stage;
        state.current = 0;
        state.total = total;
        self.publish(*state);
    }

    fn set_current(&self, current: u64) {
        let mut state = self.state.lock().expect("progress lock poisoned");
        state.current = current;
        self.publish(*state);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_load_wav_job_native(job_id: JobId, config: wav::ReadConfigBytes, tx: Sender<JobEvent>) {
    std::thread::spawn(move || {
        let sink = ThreadedLoadJobProgressSink::new(job_id, tx.clone());
        match wav::read::read_bytes_to_loaded_file_with_sink(&config, job_id, Some(&sink))
            .map(TransferLoadedFile::from)
        {
            Ok(loaded) => {
                let _ = tx.send(JobEvent::Completed(JobCompletionEvent {
                    job_id,
                    result: JobResultData::LoadWav(loaded),
                }));
            }
            Err(error) => {
                let _ = tx.send(JobEvent::Failed(JobFailureEvent {
                    job_id,
                    error: format!("Failed to load wav bytes: {error:#}"),
                }));
            }
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn spawn_load_wav_job_wasm(job_id: JobId, config: wav::ReadConfigBytes, tx: Sender<JobEvent>) {
    rayon::spawn(move || {
        let sink = ThreadedLoadJobProgressSink::new(job_id, tx.clone());
        match wav::read::read_bytes_to_loaded_file_with_sink(&config, job_id, Some(&sink))
            .map(TransferLoadedFile::from)
        {
            Ok(loaded) => {
                let _ = tx.send(JobEvent::Completed(JobCompletionEvent {
                    job_id,
                    result: JobResultData::LoadWav(loaded),
                }));
            }
            Err(error) => {
                let _ = tx.send(JobEvent::Failed(JobFailureEvent {
                    job_id,
                    error: format!("Failed to load wav bytes: {error:#}"),
                }));
            }
        }
    });
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
            stage_label: "working".to_string(),
            progress: JobProgressData::Units {
                current: 1,
                total: 3,
            },
            message: Some("hello".to_string()),
        }))
        .unwrap();
        assert!(manager.drain_events());
        let job = manager.primary_job().unwrap();
        assert_eq!(job.status, JobStatus::Running);
        assert_eq!(job.current, 1);

        tx.send(JobEvent::Completed(JobCompletionEvent {
            job_id,
            result: JobResultData::DemoTimed(DemoTimedSummary {
                stage_count: 3,
                checksum: 42,
            }),
        }))
        .unwrap();
        assert!(manager.drain_events());
        assert_eq!(manager.pending(), 0);
        assert_eq!(manager.recent_finished().len(), 1);
    }
}
