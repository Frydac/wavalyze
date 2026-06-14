//! Background statistics gathering over a single audio buffer.
//!
//! Takes an `Arc<BufferE>` cloned cheaply from `AudioManager`, streams the requested sample range
//! in chunks while periodically publishing `JobProgress` events, and posts the final
//! `BufferStats` back to the model via `Action::SetBufferStats`. Magnitudes are computed in the
//! normalized `[-1.0, 1.0]` domain (via `Sample::to_norm`) so integer- and float-typed buffers
//! yield comparable dB values.

use std::sync::Arc;
use std::sync::mpsc::Sender;

use crate::audio::{self, BufferId, buffer::Buffer, buffer::BufferE, sample, sample::Sample};
use crate::model::Action;
use crate::model::stats::{BufferStats, PeakStat, SampleValueE, StatsOptions};

use super::{JobCompletionEvent, JobEvent, JobId, JobProgress, JobProgressEvent, spawn_worker};

const CHUNK: usize = 64 * 1024;
// One publish per ~512K samples — keeps the UI ticking visibly without flooding the mpsc.
const CHUNKS_PER_PUBLISH: usize = 8;

#[allow(clippy::too_many_arguments)]
pub fn spawn_compute_stats_job(
    job_id: JobId,
    buffer_id: BufferId,
    buffer: Arc<BufferE>,
    global_range: sample::IxRange,
    offset: sample::Ix,
    options: StatsOptions,
    events_tx: Sender<JobEvent>,
    actions_tx: Sender<Action>,
) {
    spawn_worker(move || {
        let stats =
            compute_stats_streaming(job_id, &buffer, global_range, offset, options, &events_tx);
        let summary = summarize(&stats);
        let _ = actions_tx.send(Action::SetBufferStats { buffer_id, stats });
        let _ = events_tx.send(JobEvent::Completed(JobCompletionEvent { job_id, summary }));
    });
}

fn summarize(stats: &BufferStats) -> String {
    let rms = stats
        .rms_db
        .map(|db| format!("RMS {db:.2} dB"))
        .unwrap_or_default();
    let peak = stats
        .peak
        .map(|p| format!("peak {:.2} dB @ {}", p.magnitude_db, p.global_ix))
        .unwrap_or_default();
    [rms, peak]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

fn compute_stats_streaming(
    job_id: JobId,
    buffer: &BufferE,
    global_range: sample::IxRange,
    offset: sample::Ix,
    options: StatsOptions,
    events_tx: &Sender<JobEvent>,
) -> BufferStats {
    match buffer {
        BufferE::F32(b) => stream_stats(
            job_id,
            b,
            global_range,
            offset,
            options,
            SampleValueE::F32,
            events_tx,
        ),
        BufferE::I32(b) => stream_stats(
            job_id,
            b,
            global_range,
            offset,
            options,
            SampleValueE::I32,
            events_tx,
        ),
        BufferE::I16(b) => stream_stats(
            job_id,
            b,
            global_range,
            offset,
            options,
            SampleValueE::I16,
            events_tx,
        ),
    }
}

/// Gather stats for one buffer variant over the slice of `buffer.data` that the global range maps
/// to. Generic over sample type so the enum dispatch above stays a short match; `wrap` is the
/// matching `SampleValueE` constructor so the raw peak value keeps its storage type.
#[allow(clippy::too_many_arguments)]
fn stream_stats<T: Sample>(
    job_id: JobId,
    buffer: &Buffer<T>,
    global_range: sample::IxRange,
    offset: sample::Ix,
    options: StatsOptions,
    wrap: fn(T) -> SampleValueE,
    events_tx: &Sender<JobEvent>,
) -> BufferStats {
    let bit_depth = buffer.bit_depth;
    let len = buffer.data.len() as i64;
    // Map the global range into this buffer's local indices (local = global + offset) and clamp to
    // the buffer's bounds.
    let local_start = (global_range.start + offset).clamp(0, len);
    let local_end = (global_range.end + offset).clamp(0, len);
    let slice = if local_end > local_start {
        &buffer.data[local_start as usize..local_end as usize]
    } else {
        &[][..]
    };

    let total = slice.len() as u64;
    let mut sum_sq: f64 = 0.0;
    let mut processed: u64 = 0;
    let mut chunks_since_publish: usize = 0;

    let mut best_abs: f64 = -1.0;
    let mut best_local_ix: i64 = local_start;
    let mut best_raw: T = T::default();

    publish_progress(job_id, events_tx, 0, total);
    for (chunk_ix, chunk) in slice.chunks(CHUNK).enumerate() {
        let chunk_base = local_start + (chunk_ix * CHUNK) as i64;
        for (i, &s) in chunk.iter().enumerate() {
            let v = s.to_norm(bit_depth);
            if options.rms {
                sum_sq += v * v;
            }
            if options.peak {
                let a = v.abs();
                if a > best_abs {
                    best_abs = a;
                    best_local_ix = chunk_base + i as i64;
                    best_raw = s;
                }
            }
        }
        processed += chunk.len() as u64;
        chunks_since_publish += 1;
        if chunks_since_publish >= CHUNKS_PER_PUBLISH {
            chunks_since_publish = 0;
            publish_progress(job_id, events_tx, processed, total);
        }
    }
    publish_progress(job_id, events_tx, total, total);

    let rms_db = options.rms.then(|| {
        let mean_sq = if total == 0 {
            0.0
        } else {
            sum_sq / total as f64
        };
        audio::db::gain_to_db(mean_sq.sqrt() as f32)
    });

    let peak = (options.peak && total > 0).then(|| PeakStat {
        // local = global + offset, so global = local - offset.
        global_ix: best_local_ix - offset,
        magnitude_norm: best_abs,
        magnitude_db: audio::db::gain_to_db(best_abs as f32),
        raw: wrap(best_raw),
    });

    BufferStats {
        range: global_range,
        rms_db,
        peak,
    }
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
            stage_name: "stats".to_string(),
            stage_current: current,
            stage_total: total,
            overall_fraction: fraction,
        },
        message: None,
    }));
}
