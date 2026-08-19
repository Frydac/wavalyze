//! Background peak detection for per-track Y auto-fit.
//!
//! Input ranges are buffer-local and half-open. Samples are normalized before comparison so all
//! supported storage types produce a comparable absolute magnitude.

use std::sync::Arc;
use std::sync::mpsc::Sender;

use crate::audio::{BufferId, buffer::Buffer, buffer::BufferE, sample, sample::Sample};
use crate::model::{Action, track::TrackId};

use super::{JobCompletionEvent, JobEvent, JobId, JobProgress, JobProgressEvent, spawn_worker};

const CHUNK: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoFitPeakResult {
    pub track_id: TrackId,
    pub buffer_id: BufferId,
    /// Largest finite absolute normalized sample, or `None` when the range contains no finite
    /// samples.
    pub magnitude_norm: Option<f64>,
}

pub fn spawn_detect_peak_job(
    job_id: JobId,
    track_id: TrackId,
    buffer_id: BufferId,
    buffer: Arc<BufferE>,
    local_range: sample::IxRange,
    events_tx: Sender<JobEvent>,
    actions_tx: Sender<Action>,
) {
    spawn_worker(move || {
        let magnitude_norm =
            detect_peak_magnitude_with_progress(&buffer, local_range, |current, total| {
                publish_progress(job_id, &events_tx, current, total)
            });
        let result = AutoFitPeakResult {
            track_id,
            buffer_id,
            magnitude_norm,
        };
        let summary = magnitude_norm
            .map(|magnitude| format!("peak {magnitude:.6}"))
            .unwrap_or_else(|| "no finite samples".to_string());
        let _ = actions_tx.send(Action::AutoFitPeakDetected(result));
        let _ = events_tx.send(JobEvent::Completed(JobCompletionEvent { job_id, summary }));
    });
}

#[cfg(test)]
fn detect_peak_magnitude(buffer: &BufferE, local_range: sample::IxRange) -> Option<f64> {
    detect_peak_magnitude_with_progress(buffer, local_range, |_, _| {})
}

fn detect_peak_magnitude_with_progress(
    buffer: &BufferE,
    local_range: sample::IxRange,
    mut progress: impl FnMut(u64, u64),
) -> Option<f64> {
    match buffer {
        BufferE::F32(buffer) => detect_typed(buffer, local_range, &mut progress),
        BufferE::I32(buffer) => detect_typed(buffer, local_range, &mut progress),
        BufferE::I16(buffer) => detect_typed(buffer, local_range, &mut progress),
    }
}

fn detect_typed<T: Sample>(
    buffer: &Buffer<T>,
    local_range: sample::IxRange,
    progress: &mut impl FnMut(u64, u64),
) -> Option<f64> {
    let len = buffer.data.len() as i64;
    let start = local_range.start.clamp(0, len);
    let end = local_range.end.clamp(0, len);
    let slice = if end > start {
        &buffer.data[start as usize..end as usize]
    } else {
        &[]
    };
    let total = slice.len() as u64;
    let mut processed = 0;
    let mut peak: Option<f64> = None;

    progress(0, total);
    for chunk in slice.chunks(CHUNK) {
        for &sample in chunk {
            let magnitude = sample.to_norm(buffer.bit_depth).abs();
            if magnitude.is_finite() && peak.is_none_or(|current| magnitude > current) {
                peak = Some(magnitude);
            }
        }
        processed += chunk.len() as u64;
        progress(processed, total);
    }
    if total == 0 {
        progress(0, 0);
    }

    peak
}

fn publish_progress(job_id: JobId, tx: &Sender<JobEvent>, current: u64, total: u64) {
    let overall_fraction = if total == 0 {
        1.0
    } else {
        current as f32 / total as f32
    };
    let _ = tx.send(JobEvent::Progress(JobProgressEvent {
        job_id,
        progress: JobProgress {
            stage_name: "peak".to_string(),
            stage_current: current,
            stage_total: total,
            overall_fraction,
        },
        message: None,
    }));
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;
    use crate::audio::manager::AudioManager;
    use crate::model::{config::TrackConfig, tracks::Tracks};

    fn f32_buffer(data: Vec<f32>) -> BufferE {
        BufferE::F32(Buffer {
            sample_rate: 48_000,
            bit_depth: 32,
            data,
        })
    }

    #[test]
    fn detects_positive_peak() {
        let buffer = f32_buffer(vec![-0.2, 0.75, -0.5]);

        assert_eq!(detect_peak_magnitude(&buffer, (0..3).into()), Some(0.75));
    }

    #[test]
    fn detects_negative_peak_by_absolute_magnitude() {
        let buffer = f32_buffer(vec![0.5, -0.9, 0.8]);

        assert!((detect_peak_magnitude(&buffer, (0..3).into()).unwrap() - 0.9).abs() < 1e-6);
    }

    #[test]
    fn normalizes_i16_and_i32_peaks() {
        let i16_buffer = BufferE::I16(Buffer {
            sample_rate: 48_000,
            bit_depth: 16,
            data: vec![0, -16_384, 8_192],
        });
        let i32_buffer = BufferE::I32(Buffer {
            sample_rate: 48_000,
            bit_depth: 24,
            data: vec![0, -4_194_304, 2_097_152],
        });

        assert_eq!(detect_peak_magnitude(&i16_buffer, (0..3).into()), Some(0.5));
        assert_eq!(detect_peak_magnitude(&i32_buffer, (0..3).into()), Some(0.5));
    }

    #[test]
    fn empty_or_out_of_bounds_range_has_no_peak() {
        let buffer = f32_buffer(vec![0.5, -0.9]);

        assert_eq!(detect_peak_magnitude(&buffer, (1..1).into()), None);
        assert_eq!(detect_peak_magnitude(&buffer, (10..20).into()), None);
    }

    #[test]
    fn silence_reports_zero_peak() {
        let buffer = f32_buffer(vec![0.0, -0.0, 0.0]);

        assert_eq!(detect_peak_magnitude(&buffer, (0..3).into()), Some(0.0));
    }

    #[test]
    fn ignores_non_finite_float_samples() {
        let buffer = f32_buffer(vec![f32::NAN, f32::INFINITY, -0.4, f32::NEG_INFINITY]);
        let only_non_finite = f32_buffer(vec![f32::NAN, f32::INFINITY]);

        assert!((detect_peak_magnitude(&buffer, (0..4).into()).unwrap() - 0.4).abs() < 1e-6);
        assert_eq!(detect_peak_magnitude(&only_non_finite, (0..2).into()), None);
    }

    #[test]
    fn worker_reports_result_action_and_job_completion() {
        let mut audio = AudioManager::default();
        let buffer = Arc::new(f32_buffer(vec![0.1, -0.8, 0.4]));
        let buffer_id = audio.buffers.insert(buffer.clone());
        let mut tracks = Tracks::default();
        let track_id = tracks
            .add_track_to_end(buffer_id, 48_000, &TrackConfig::default())
            .unwrap();
        let (events_tx, events_rx) = mpsc::channel();
        let (actions_tx, actions_rx) = mpsc::channel();

        spawn_detect_peak_job(
            7,
            track_id,
            buffer_id,
            buffer,
            (0..3).into(),
            events_tx,
            actions_tx,
        );

        let action = actions_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let Action::AutoFitPeakDetected(result) = action else {
            panic!("unexpected peak worker action");
        };
        assert_eq!(result.track_id, track_id);
        assert_eq!(result.buffer_id, buffer_id);
        assert!((result.magnitude_norm.unwrap() - 0.8).abs() < 1e-6);

        loop {
            match events_rx.recv_timeout(Duration::from_secs(2)).unwrap() {
                JobEvent::Completed(completion) => {
                    assert_eq!(completion.job_id, 7);
                    assert!(completion.summary.contains("0.800000"));
                    break;
                }
                JobEvent::Progress(_) => {}
                JobEvent::Failed(failure) => panic!("peak worker failed: {}", failure.error),
            }
        }
    }
}
