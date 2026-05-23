//! Demo background job — synthetic stage-based CPU work used to exercise the progress
//! pipeline without touching the model. No completion `Action` is pushed.

use std::sync::mpsc::Sender;

use serde::{Deserialize, Serialize};

use crate::model::Action;

use super::{JobCompletionEvent, JobEvent, JobId, JobProgress, JobProgressEvent, spawn_worker};

/// Inputs to a demo job. `stage_count` is the number of progress checkpoints; `work_units`
/// controls how much CPU is burned per stage.
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

/// Synthetic stage-based job for exercising the progress/recent-finished pipeline. No completion
/// `Action` — `_actions_tx` is intentionally unused.
pub fn spawn_demo_timed_job(
    job_id: JobId,
    config: DemoTimedConfig,
    events_tx: Sender<JobEvent>,
    _actions_tx: Sender<Action>,
) {
    spawn_worker(move || run_demo_timed_job(job_id, config, events_tx));
}

// ---------------------------------------------------------------------------
// Demo job internals — pure CPU work, no model interaction. Safe to skip when reading.
// ---------------------------------------------------------------------------

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
    let stage_count = config.stage_count as f32;
    let mut checksum = 0_u64;
    for stage_ix in 0..config.stage_count {
        let stage_label = demo_stage_name(stage_ix, config.stage_count);
        let _ = tx.send(JobEvent::Progress(JobProgressEvent {
            job_id,
            progress: JobProgress {
                stage_name: stage_label.clone(),
                stage_current: 0,
                stage_total: 1,
                overall_fraction: stage_ix as f32 / stage_count,
            },
            message: Some(format!("running {stage_label}")),
        }));

        checksum ^= compute_demo_stage(stage_ix, config.work_units, job_id);

        let _ = tx.send(JobEvent::Progress(JobProgressEvent {
            job_id,
            progress: JobProgress {
                stage_name: stage_label,
                stage_current: 1,
                stage_total: 1,
                overall_fraction: (stage_ix + 1) as f32 / stage_count,
            },
            message: Some(format!("completed stage {}", stage_ix + 1)),
        }));
    }

    let _ = tx.send(JobEvent::Completed(JobCompletionEvent {
        job_id,
        summary: format!(
            "{} stages complete, checksum {}",
            config.stage_count, checksum
        ),
    }));
}
