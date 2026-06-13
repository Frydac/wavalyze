//! Background diff jobs for existing buffers and CLI file-pair diffing.

use std::sync::Arc;
use std::sync::mpsc::Sender;

use anyhow::Result;
#[cfg(not(target_arch = "wasm32"))]
use anyhow::{Context, anyhow};

use crate::audio::{
    BufferId,
    buffer::{Buffer, BufferE},
    sample::{self, Sample},
    thumbnail::ThumbnailE,
};
use crate::model::Action;
use crate::wav;

#[cfg(not(target_arch = "wasm32"))]
use super::load_wav;
use super::{
    JobCompletionEvent, JobEvent, JobFailureEvent, JobId, JobProgress, JobProgressEvent,
    spawn_worker,
};

const CHUNK: usize = 64 * 1024;

#[derive(Clone, Copy)]
struct ProgressRange {
    start: f32,
    end: f32,
}

impl ProgressRange {
    const FULL: Self = Self {
        start: 0.0,
        end: 1.0,
    };

    fn map(self, fraction: f32) -> f32 {
        // Parent jobs reserve progress ranges for sub-steps (load A, load B, diff, final thumbnail).
        self.start + fraction * (self.end - self.start).max(0.0)
    }
}

#[derive(Debug)]
pub struct DiffBufferResult {
    pub buffer: BufferE,
    pub sample_ix_offset_diff: sample::Ix,
}

#[derive(Debug)]
pub struct ComputedDiff {
    pub buffer_id_a: BufferId,
    pub buffer_id_b: BufferId,
    pub sample_ix_offset_a: sample::Ix,
    pub sample_ix_offset_b: sample::Ix,
    pub sample_ix_offset_diff: sample::Ix,
    pub diff_buffer: BufferE,
    pub diff_thumbnail: ThumbnailE,
}

/// One computed diff between a channel of file A and a channel of file B.
#[derive(Debug)]
pub struct LoadedDiffPair {
    pub ch_a: wav::read::ChIx,
    pub ch_b: wav::read::ChIx,
    pub sample_ix_offset_diff: sample::Ix,
    pub diff_buffer: BufferE,
    pub diff_thumbnail: ThumbnailE,
}

/// Result of loading two files and diffing one or more channel pairs between them. A single pair is
/// the degenerate (single-channel) case.
#[derive(Debug)]
pub struct LoadedDiff {
    pub file_a: wav::read::LoadedFile,
    pub file_b: wav::read::LoadedFile,
    pub sample_ix_offset_a: sample::Ix,
    pub sample_ix_offset_b: sample::Ix,
    pub pairs: Vec<LoadedDiffPair>,
}

pub struct DiffBuffersJobInput {
    pub buffer_id_a: BufferId,
    pub buffer_id_b: BufferId,
    pub buffer_a: Arc<BufferE>,
    pub buffer_b: Arc<BufferE>,
    pub sample_ix_offset_a: sample::Ix,
    pub sample_ix_offset_b: sample::Ix,
}

pub fn spawn_diff_buffers_job(
    job_id: JobId,
    generation: u64,
    input: DiffBuffersJobInput,
    events_tx: Sender<JobEvent>,
    actions_tx: Sender<Action>,
) {
    spawn_worker(move || {
        let result = compute_diff_buffer_with_progress(
            job_id,
            &input.buffer_a,
            &input.buffer_b,
            input.sample_ix_offset_a,
            input.sample_ix_offset_b,
            &events_tx,
            ProgressRange::FULL,
        )
        .map(|diff| {
            let diff_thumbnail = ThumbnailE::from_buffer_e(&diff.buffer, None);
            ComputedDiff {
                buffer_id_a: input.buffer_id_a,
                buffer_id_b: input.buffer_id_b,
                sample_ix_offset_a: input.sample_ix_offset_a,
                sample_ix_offset_b: input.sample_ix_offset_b,
                sample_ix_offset_diff: diff.sample_ix_offset_diff,
                diff_buffer: diff.buffer,
                diff_thumbnail,
            }
        });
        finish_computed_diff_job(job_id, generation, result, &events_tx, &actions_tx);
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_load_diff_paths_job(
    job_id: JobId,
    generation: u64,
    file_a: wav::ReadConfig,
    file_b: wav::ReadConfig,
    pairs: Vec<(wav::read::ChIx, wav::read::ChIx)>,
    events_tx: Sender<JobEvent>,
    actions_tx: Sender<Action>,
) {
    spawn_worker(move || {
        let result = load_and_compute_diff(job_id, file_a, file_b, pairs, &events_tx);
        finish_loaded_diff_job(job_id, generation, result, &events_tx, &actions_tx);
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn load_and_compute_diff(
    job_id: JobId,
    mut file_a: wav::ReadConfig,
    mut file_b: wav::ReadConfig,
    pairs: Vec<(wav::read::ChIx, wav::read::ChIx)>,
    events_tx: &Sender<JobEvent>,
) -> Result<LoadedDiff> {
    // Kept as one parent job so the model receives every source channel and diff in a single
    // integration action (deterministic ordering). The A/B loads use the normal WAV load helper and
    // detailed progress sink; each selected pair then produces one diff track.
    anyhow::ensure!(!pairs.is_empty(), "diff requires at least one channel pair");
    publish_progress(
        job_id,
        events_tx,
        "validating",
        0,
        1,
        ProgressRange {
            start: 0.0,
            end: 0.02,
        },
    );
    // Restrict each load to only the channels referenced by the selected pairs, so unselected
    // channels are never decoded.
    file_a.ch_ixs = Some(distinct_sorted(pairs.iter().map(|(ch_a, _)| *ch_a)));
    file_b.ch_ixs = Some(distinct_sorted(pairs.iter().map(|(_, ch_b)| *ch_b)));
    publish_progress(
        job_id,
        events_tx,
        "validating",
        1,
        1,
        ProgressRange {
            start: 0.0,
            end: 0.02,
        },
    );

    let sink_a = load_wav::ThreadedLoadJobProgressSink::new_mapped(
        job_id,
        events_tx.clone(),
        Some("A".to_string()),
        0.02,
        0.42,
    );
    // Use the regular WAV load path but map its detailed progress into the A slice of this job.
    let loaded_a = load_wav::load_wav_path_for_job(job_id, &file_a, &sink_a)
        .context("failed to load first diff input")?;

    let sink_b = load_wav::ThreadedLoadJobProgressSink::new_mapped(
        job_id,
        events_tx.clone(),
        Some("B".to_string()),
        0.42,
        0.82,
    );
    // Same for B; no intermediate `IntegrateLoadedFile` action is emitted before the diff exists.
    let loaded_b = load_wav::load_wav_path_for_job(job_id, &file_b, &sink_b)
        .context("failed to load second diff input")?;

    // Spread the remaining progress (0.82..1.0) evenly across the per-pair diff computations.
    let pair_count = pairs.len();
    let mut pair_results = Vec::with_capacity(pair_count);
    for (ix, (ch_a, ch_b)) in pairs.into_iter().enumerate() {
        let buffer_a = loaded_a
            .channels
            .get(&ch_a)
            .ok_or_else(|| anyhow!("first diff input is missing channel {ch_a}"))?;
        let buffer_b = loaded_b
            .channels
            .get(&ch_b)
            .ok_or_else(|| anyhow!("second diff input is missing channel {ch_b}"))?;
        let span = 1.0 - 0.82;
        let progress_range = ProgressRange {
            start: 0.82 + span * (ix as f32) / pair_count as f32,
            end: 0.82 + span * (ix as f32 + 1.0) / pair_count as f32,
        };
        let diff = compute_diff_buffer_with_progress(
            job_id,
            buffer_a,
            buffer_b,
            file_a.sample_ix_offset,
            file_b.sample_ix_offset,
            events_tx,
            progress_range,
        )?;
        let diff_thumbnail = ThumbnailE::from_buffer_e(&diff.buffer, None);
        pair_results.push(LoadedDiffPair {
            ch_a,
            ch_b,
            sample_ix_offset_diff: diff.sample_ix_offset_diff,
            diff_buffer: diff.buffer,
            diff_thumbnail,
        });
    }

    Ok(LoadedDiff {
        file_a: loaded_a,
        file_b: loaded_b,
        sample_ix_offset_a: file_a.sample_ix_offset,
        sample_ix_offset_b: file_b.sample_ix_offset,
        pairs: pair_results,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn distinct_sorted(iter: impl Iterator<Item = wav::read::ChIx>) -> Vec<wav::read::ChIx> {
    let mut ch_ixs: Vec<_> = iter.collect();
    ch_ixs.sort_unstable();
    ch_ixs.dedup();
    ch_ixs
}

fn finish_computed_diff_job(
    job_id: JobId,
    generation: u64,
    result: Result<ComputedDiff>,
    events_tx: &Sender<JobEvent>,
    actions_tx: &Sender<Action>,
) {
    match result {
        Ok(diff) => {
            let samples = diff.diff_buffer.nr_samples();
            let _ = actions_tx.send(Action::IntegrateDiffBuffer { generation, diff });
            let _ = events_tx.send(JobEvent::Completed(JobCompletionEvent {
                job_id,
                summary: format!("Computed diff ({samples} samples)"),
            }));
        }
        Err(error) => {
            let _ = events_tx.send(JobEvent::Failed(JobFailureEvent {
                job_id,
                error: format!("Failed to compute diff: {error:#}"),
            }));
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn finish_loaded_diff_job(
    job_id: JobId,
    generation: u64,
    result: Result<LoadedDiff>,
    events_tx: &Sender<JobEvent>,
    actions_tx: &Sender<Action>,
) {
    match result {
        Ok(diff) => {
            let pair_count = diff.pairs.len();
            let _ = actions_tx.send(Action::IntegrateLoadedDiff { generation, diff });
            let _ = events_tx.send(JobEvent::Completed(JobCompletionEvent {
                job_id,
                summary: format!("Loaded and computed {pair_count} diff(s)"),
            }));
        }
        Err(error) => {
            let _ = events_tx.send(JobEvent::Failed(JobFailureEvent {
                job_id,
                error: format!("Failed to load diff inputs: {error:#}"),
            }));
        }
    }
}

pub fn compute_diff_buffer(
    buffer_a: &BufferE,
    buffer_b: &BufferE,
    sample_ix_offset_a: sample::Ix,
    sample_ix_offset_b: sample::Ix,
) -> Result<DiffBufferResult> {
    compute_diff_buffer_impl(
        buffer_a,
        buffer_b,
        sample_ix_offset_a,
        sample_ix_offset_b,
        None,
    )
}

fn compute_diff_buffer_with_progress(
    job_id: JobId,
    buffer_a: &BufferE,
    buffer_b: &BufferE,
    sample_ix_offset_a: sample::Ix,
    sample_ix_offset_b: sample::Ix,
    events_tx: &Sender<JobEvent>,
    progress_range: ProgressRange,
) -> Result<DiffBufferResult> {
    compute_diff_buffer_impl(
        buffer_a,
        buffer_b,
        sample_ix_offset_a,
        sample_ix_offset_b,
        Some((job_id, events_tx, progress_range)),
    )
}

fn compute_diff_buffer_impl(
    buffer_a: &BufferE,
    buffer_b: &BufferE,
    sample_ix_offset_a: sample::Ix,
    sample_ix_offset_b: sample::Ix,
    progress: Option<(JobId, &Sender<JobEvent>, ProgressRange)>,
) -> Result<DiffBufferResult> {
    anyhow::ensure!(
        buffer_a.sample_rate() == buffer_b.sample_rate(),
        "diff inputs must have the same sample rate ({} != {})",
        buffer_a.sample_rate(),
        buffer_b.sample_rate()
    );

    let len_a = buffer_a.nr_samples() as sample::Ix;
    let len_b = buffer_b.nr_samples() as sample::Ix;
    let start_n = (-sample_ix_offset_a).min(-sample_ix_offset_b);
    let end_n = (len_a - sample_ix_offset_a).max(len_b - sample_ix_offset_b);
    let len = (end_n - start_n).max(0) as usize;
    let mut out = Buffer::with_capacity(buffer_a.sample_rate(), 32, len);

    if let Some((job_id, tx, range)) = progress {
        publish_progress(job_id, tx, "diff", 0, len as u64, range);
    }

    for base in (0..len).step_by(CHUNK) {
        let chunk_end = (base + CHUNK).min(len);
        for out_ix in base..chunk_end {
            let n = start_n + out_ix as sample::Ix;
            let a = sample_norm_at(buffer_a, n + sample_ix_offset_a);
            let b = sample_norm_at(buffer_b, n + sample_ix_offset_b);
            out.data.push(a - b);
        }
        if let Some((job_id, tx, range)) = progress {
            publish_progress(job_id, tx, "diff", chunk_end as u64, len as u64, range);
        }
    }

    Ok(DiffBufferResult {
        buffer: BufferE::F32(out),
        sample_ix_offset_diff: -start_n,
    })
}

fn sample_norm_at(buffer: &BufferE, ix: sample::Ix) -> f32 {
    if ix < 0 {
        return 0.0;
    }
    let ix = ix as usize;
    match buffer {
        BufferE::F32(buffer) => buffer
            .data
            .get(ix)
            .map(|sample| sample.to_norm(buffer.bit_depth) as f32)
            .unwrap_or(0.0),
        BufferE::I32(buffer) => buffer
            .data
            .get(ix)
            .map(|sample| sample.to_norm(buffer.bit_depth) as f32)
            .unwrap_or(0.0),
        BufferE::I16(buffer) => buffer
            .data
            .get(ix)
            .map(|sample| sample.to_norm(buffer.bit_depth) as f32)
            .unwrap_or(0.0),
    }
}

fn publish_progress(
    job_id: JobId,
    tx: &Sender<JobEvent>,
    stage_name: &str,
    current: u64,
    total: u64,
    progress_range: ProgressRange,
) {
    let fraction = if total == 0 {
        1.0
    } else {
        current as f32 / total as f32
    };
    let _ = tx.send(JobEvent::Progress(JobProgressEvent {
        job_id,
        progress: JobProgress {
            stage_name: stage_name.to_string(),
            stage_current: current,
            stage_total: total,
            overall_fraction: progress_range.map(fraction),
        },
        message: None,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f32_buffer(data: &[f32], sample_rate: u32) -> BufferE {
        BufferE::F32(Buffer {
            sample_rate,
            bit_depth: 32,
            data: data.to_vec(),
        })
    }

    fn diff_data(result: DiffBufferResult) -> Vec<f32> {
        match result.buffer {
            BufferE::F32(buffer) => buffer.data,
            _ => unreachable!(),
        }
    }

    #[test]
    fn diff_pads_shorter_buffer_with_zero() {
        let a = f32_buffer(&[1.0, 2.0, 3.0], 48_000);
        let b = f32_buffer(&[0.5], 48_000);

        let result = compute_diff_buffer(&a, &b, 0, 0).unwrap();

        assert_eq!(result.sample_ix_offset_diff, 0);
        assert_eq!(diff_data(result), vec![0.5, 2.0, 3.0]);
    }

    #[test]
    fn diff_applies_signed_offsets() {
        let a = f32_buffer(&[1.0, 2.0, 3.0], 48_000);
        let b = f32_buffer(&[10.0, 20.0, 30.0], 48_000);

        let result = compute_diff_buffer(&a, &b, 1, 0).unwrap();

        assert_eq!(result.sample_ix_offset_diff, 1);
        assert_eq!(diff_data(result), vec![1.0, -8.0, -17.0, -30.0]);
    }

    #[test]
    fn diff_rejects_sample_rate_mismatch() {
        let a = f32_buffer(&[1.0], 48_000);
        let b = f32_buffer(&[1.0], 44_100);

        assert!(compute_diff_buffer(&a, &b, 0, 0).is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn write_test_wav(name: &str, channels: u16) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from("target/test_output/diff_jobs");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let spec = hound::WavSpec {
            channels,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for _ in 0..channels {
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();
        path
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn load_diff_computes_one_pair_per_selection() {
        let path_a = write_test_wav("multi_a_2ch.wav", 2);
        let path_b = write_test_wav("multi_b_3ch.wav", 3);
        let (tx, _rx) = std::sync::mpsc::channel();

        let diff = load_and_compute_diff(
            9,
            wav::ReadConfig::new(path_a),
            wav::ReadConfig::new(path_b),
            vec![(0, 0), (1, 2)],
            &tx,
        )
        .unwrap();

        // Only the referenced channels are decoded.
        assert_eq!(diff.file_a.channels.len(), 2);
        assert_eq!(diff.file_b.channels.len(), 2);
        assert!(diff.file_b.channels.contains_key(&2));
        assert!(!diff.file_b.channels.contains_key(&1));

        let pairs: Vec<_> = diff.pairs.iter().map(|p| (p.ch_a, p.ch_b)).collect();
        assert_eq!(pairs, vec![(0, 0), (1, 2)]);
        assert!(diff.pairs.iter().all(|p| p.diff_buffer.nr_samples() == 1));
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn load_diff_reports_detailed_prefixed_load_progress() {
        let path_a = write_test_wav("progress_a.wav", 1);
        let path_b = write_test_wav("progress_b.wav", 1);
        let (tx, rx) = std::sync::mpsc::channel();

        let diff = load_and_compute_diff(
            7,
            wav::ReadConfig::new(path_a),
            wav::ReadConfig::new(path_b),
            vec![(0, 0)],
            &tx,
        )
        .unwrap();

        assert_eq!(diff.pairs.len(), 1);
        assert_eq!(diff.pairs[0].diff_buffer.nr_samples(), 1);
        let progress: Vec<_> = rx
            .try_iter()
            .filter_map(|event| match event {
                JobEvent::Progress(progress) => Some(progress.progress),
                _ => None,
            })
            .collect();
        assert!(
            progress
                .iter()
                .any(|progress| progress.stage_name == "A: reading samples")
        );
        assert!(
            progress
                .iter()
                .any(|progress| progress.stage_name == "A: thumbnails")
        );
        assert!(
            progress
                .iter()
                .any(|progress| progress.stage_name == "B: reading samples")
        );
        assert!(
            progress
                .iter()
                .any(|progress| progress.stage_name == "B: thumbnails")
        );
        assert!(progress.iter().any(|progress| {
            progress.stage_name == "diff" && progress.overall_fraction >= 0.82
        }));
    }
}
