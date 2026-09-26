//! Compute leading-silence alignment without scanning audio on the UI thread.
//!
//! The worker reads buffer snapshots and returns an offset proposal through an action. The
//! action checks those snapshots before applying the proposal, so an in-flight scan cannot
//! overwrite alignment after its file has been reloaded.

use std::sync::{Arc, mpsc::Sender};

use crate::{
    audio::{BufferId, buffer::BufferE, sample, sample::Sample},
    model::{Action, track::TrackId},
    wav::file::FileId,
};

use super::{JobCompletionEvent, JobEvent, JobId, JobProgress, JobProgressEvent, spawn_worker};

/// Choose which boundary of the leading silence should align with timeline sample zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetDetectionMode {
    FirstNonZero, // when the signal starts with a non-zero
    LastLeadingZero, // e.g. when the signal starts with a zero (often sine waves are used for
                  // testing signal processing)
}

/// Destination for a detected offset: one track or a file and its inheriting channel tracks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetDetectionTarget {
    Track {
        track_id: TrackId,
        buffer_id: BufferId,
    },
    File {
        file_id: FileId,
    },
}

/// Alignment proposal from a completed scan; `None` means no qualifying boundary was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetDetectionResult {
    pub target: OffsetDetectionTarget,
    pub sample_ix_offset: Option<sample::Ix>,
}

pub fn spawn_detect_offset_job(
    job_id: JobId,
    target: OffsetDetectionTarget,
    mode: OffsetDetectionMode,
    buffers: Vec<Arc<BufferE>>,
    events_tx: Sender<JobEvent>,
    actions_tx: Sender<Action>,
) {
    spawn_worker(move || {
        let total = buffers
            .iter()
            .map(|buffer| buffer.nr_samples() as u64)
            .sum();
        publish_progress(job_id, &events_tx, 0, total);

        let mut processed = 0;
        let sample_ix_offset = buffers
            .iter()
            .filter_map(|buffer| {
                let offset = detect_offset(buffer, mode);
                processed += buffer.nr_samples() as u64;
                publish_progress(job_id, &events_tx, processed, total);
                offset
            })
            .min();

        let result = OffsetDetectionResult {
            target,
            sample_ix_offset,
        };
        let summary = sample_ix_offset
            .map(|offset| format!("offset {offset}"))
            .unwrap_or_else(|| "no non-zero sample".to_owned());
        let _ = actions_tx.send(Action::OffsetDetectedChecked { result, buffers });
        let _ = events_tx.send(JobEvent::Completed(JobCompletionEvent { job_id, summary }));
    });
}

pub(crate) fn detect_offset(buffer: &BufferE, mode: OffsetDetectionMode) -> Option<sample::Ix> {
    let first_non_zero = match buffer {
        BufferE::F32(buffer) => first_non_zero(&buffer.data),
        BufferE::I32(buffer) => first_non_zero(&buffer.data),
        BufferE::I16(buffer) => first_non_zero(&buffer.data),
    }?;

    match mode {
        OffsetDetectionMode::FirstNonZero => Some(first_non_zero),
        OffsetDetectionMode::LastLeadingZero => (first_non_zero > 0).then_some(first_non_zero - 1),
    }
}

fn first_non_zero<T: Sample>(samples: &[T]) -> Option<sample::Ix> {
    samples
        .iter()
        .position(|sample| *sample != T::ZERO)
        .map(|ix| ix as sample::Ix)
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
            stage_name: "leading silence".to_owned(),
            stage_current: current,
            stage_total: total,
            overall_fraction,
        },
        message: None,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::buffer::Buffer;

    fn f32_buffer(data: Vec<f32>) -> BufferE {
        BufferE::F32(Buffer {
            sample_rate: 48_000,
            bit_depth: 32,
            data,
        })
    }

    #[test]
    fn detects_first_non_zero_and_last_leading_zero() {
        let buffer = f32_buffer(vec![0.0, -0.0, 0.25]);

        assert_eq!(
            detect_offset(&buffer, OffsetDetectionMode::FirstNonZero),
            Some(2)
        );
        assert_eq!(
            detect_offset(&buffer, OffsetDetectionMode::LastLeadingZero),
            Some(1)
        );
    }

    #[test]
    fn sine_style_initial_zero_uses_sample_zero_as_last_leading_zero() {
        let buffer = f32_buffer(vec![0.0, 0.25, 0.5]);

        assert_eq!(
            detect_offset(&buffer, OffsetDetectionMode::FirstNonZero),
            Some(1)
        );
        assert_eq!(
            detect_offset(&buffer, OffsetDetectionMode::LastLeadingZero),
            Some(0)
        );
    }

    #[test]
    fn immediate_non_zero_has_no_leading_zero() {
        let buffer = f32_buffer(vec![0.25, 0.0]);

        assert_eq!(
            detect_offset(&buffer, OffsetDetectionMode::FirstNonZero),
            Some(0)
        );
        assert_eq!(
            detect_offset(&buffer, OffsetDetectionMode::LastLeadingZero),
            None
        );
    }

    #[test]
    fn silence_has_no_detected_offset() {
        let buffer = f32_buffer(vec![0.0, -0.0, 0.0]);

        assert_eq!(
            detect_offset(&buffer, OffsetDetectionMode::FirstNonZero),
            None
        );
        assert_eq!(
            detect_offset(&buffer, OffsetDetectionMode::LastLeadingZero),
            None
        );
    }

    #[test]
    fn supports_integer_buffers() {
        let i16_buffer = BufferE::I16(Buffer {
            sample_rate: 48_000,
            bit_depth: 16,
            data: vec![0, 0, -1],
        });
        let i32_buffer = BufferE::I32(Buffer {
            sample_rate: 48_000,
            bit_depth: 24,
            data: vec![0, 2],
        });

        assert_eq!(
            detect_offset(&i16_buffer, OffsetDetectionMode::FirstNonZero),
            Some(2)
        );
        assert_eq!(
            detect_offset(&i32_buffer, OffsetDetectionMode::LastLeadingZero),
            Some(0)
        );
    }

    #[test]
    fn worker_uses_minimum_valid_offset_across_buffers() {
        use std::{sync::mpsc, time::Duration};

        let buffers = vec![
            Arc::new(f32_buffer(vec![0.0, 0.0, 1.0])),
            Arc::new(f32_buffer(vec![0.0, 1.0])),
            Arc::new(f32_buffer(vec![0.0, 0.0, 0.0])),
        ];
        let (events_tx, events_rx) = mpsc::channel();
        let (actions_tx, actions_rx) = mpsc::channel();
        let target = OffsetDetectionTarget::File {
            file_id: FileId::default(),
        };

        spawn_detect_offset_job(
            7,
            target,
            OffsetDetectionMode::FirstNonZero,
            buffers,
            events_tx,
            actions_tx,
        );

        let action = actions_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let Action::OffsetDetectedChecked { result, .. } = action else {
            panic!("unexpected offset worker action");
        };
        assert_eq!(result.target, target);
        assert_eq!(result.sample_ix_offset, Some(1));

        loop {
            match events_rx.recv_timeout(Duration::from_secs(2)).unwrap() {
                JobEvent::Completed(completion) => {
                    assert_eq!(completion.job_id, 7);
                    assert_eq!(completion.summary, "offset 1");
                    break;
                }
                JobEvent::Progress(progress) => {
                    assert_eq!(progress.progress.stage_total, 8);
                }
                JobEvent::Failed(failure) => panic!("offset job failed: {}", failure.error),
            }
        }
    }
}
