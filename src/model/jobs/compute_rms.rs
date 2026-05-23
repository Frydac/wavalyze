//! Background dB-RMS computation over a single audio buffer.
//!
//! Takes an `Arc<BufferE>` cloned cheaply from `AudioManager`, streams samples in chunks while
//! periodically publishing `JobProgress` events, and posts the final value back to the model via
//! `Action::SetBufferRms`. RMS is computed in the normalized `[-1.0, 1.0]` domain (via
//! `Sample::to_norm`) so integer- and float-typed buffers yield comparable dB values.

use std::sync::Arc;
use std::sync::mpsc::Sender;

use crate::audio::{self, BufferId, buffer::Buffer, buffer::BufferE, sample::Sample};
use crate::model::Action;

use super::{JobCompletionEvent, JobEvent, JobId, JobProgress, JobProgressEvent, spawn_worker};

const CHUNK: usize = 64 * 1024;
// One publish per ~512K samples — keeps the UI ticking visibly without flooding the mpsc.
const CHUNKS_PER_PUBLISH: usize = 8;

pub fn spawn_compute_rms_job(
    job_id: JobId,
    buffer_id: BufferId,
    buffer: Arc<BufferE>,
    events_tx: Sender<JobEvent>,
    actions_tx: Sender<Action>,
) {
    spawn_worker(move || {
        let rms_db = compute_rms_db_streaming(job_id, &buffer, &events_tx);
        let _ = actions_tx.send(Action::SetBufferRms { buffer_id, rms_db });
        let _ = events_tx.send(JobEvent::Completed(JobCompletionEvent {
            job_id,
            summary: format!("RMS = {rms_db:.2} dB"),
        }));
    });
}

fn compute_rms_db_streaming(job_id: JobId, buffer: &BufferE, events_tx: &Sender<JobEvent>) -> f32 {
    let (sum_sq, count) = match buffer {
        BufferE::F32(b) => stream_sum_sq(job_id, b, events_tx),
        BufferE::I32(b) => stream_sum_sq(job_id, b, events_tx),
        BufferE::I16(b) => stream_sum_sq(job_id, b, events_tx),
    };
    let mean_sq = if count == 0 {
        0.0
    } else {
        sum_sq / count as f64
    };
    let rms = mean_sq.sqrt() as f32;
    audio::db::gain_to_db(rms)
}

/// Sum-of-squares of normalized samples for one buffer variant. Generic over sample type so the
/// enum dispatch above stays a 3-line match instead of duplicating the loop per variant.
fn stream_sum_sq<T: Sample>(
    job_id: JobId,
    buffer: &Buffer<T>,
    events_tx: &Sender<JobEvent>,
) -> (f64, u64) {
    let total = buffer.data.len() as u64;
    let bit_depth = buffer.bit_depth;
    let mut sum_sq: f64 = 0.0;
    let mut processed: u64 = 0;
    let mut chunks_since_publish: usize = 0;

    publish_progress(job_id, events_tx, 0, total);
    for chunk in buffer.data.chunks(CHUNK) {
        for &s in chunk {
            let v = s.to_norm(bit_depth);
            sum_sq += v * v;
        }
        processed += chunk.len() as u64;
        chunks_since_publish += 1;
        if chunks_since_publish >= CHUNKS_PER_PUBLISH {
            chunks_since_publish = 0;
            publish_progress(job_id, events_tx, processed, total);
        }
    }
    publish_progress(job_id, events_tx, total, total);
    (sum_sq, total)
}

fn publish_progress(job_id: JobId, tx: &Sender<JobEvent>, current: u64, total: u64) {
    let fraction = if total == 0 {
        1.0
    } else {
        current as f32 / total as f32
    };
    let _ = tx.send(JobEvent::Progress(JobProgressEvent {
        job_id,
        progress: JobProgress {
            stage_name: "rms".to_string(),
            stage_current: current,
            stage_total: total,
            overall_fraction: fraction,
        },
        message: None,
    }));
}
