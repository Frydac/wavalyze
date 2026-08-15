//! Per-buffer statistics gathered over a chosen sample range.
//!
//! Computed in the background (see `model::jobs::compute_stats`) and stored per-buffer in
//! `AudioManager::stats`. Today we gather dB-RMS and the peak (max-magnitude) sample; which stats
//! to gather is controlled by [`StatsOptions`] (both on by default — a config UI comes later).

use crate::audio::sample;

/// Which statistics to gather. Both default to enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatsOptions {
    pub rms: bool,
    pub peak: bool,
}

impl Default for StatsOptions {
    fn default() -> Self {
        Self {
            rms: true,
            peak: true,
        }
    }
}

/// A raw sample value, type-erased to mirror the buffer's storage type. Used so the peak readout
/// can show the original integer value for integer buffers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SampleValueE {
    F32(f32),
    I32(i32),
    I16(i16),
}

impl std::fmt::Display for SampleValueE {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SampleValueE::F32(value) => write!(formatter, "{value:.6}"),
            SampleValueE::I32(value) => value.fmt(formatter),
            SampleValueE::I16(value) => value.fmt(formatter),
        }
    }
}

/// The peak (maximum-magnitude) sample found in the range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakStat {
    /// Index of the peak in global-timeline space (buffer-local index minus the track offset).
    pub global_ix: sample::Ix,
    /// Magnitude of the peak as a normalized value in `[0, 1]`.
    pub magnitude_norm: f64,
    /// Magnitude of the peak in dB (`gain_to_db(magnitude_norm)`).
    pub magnitude_db: f32,
    /// The signed raw sample value at the peak, in the buffer's storage type.
    pub raw: SampleValueE,
}

/// Statistics gathered for one buffer over [`Self::range`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BufferStats {
    /// The global-timeline range the stats were computed over.
    pub range: sample::IxRange,
    pub rms_db: Option<f32>,
    pub peak: Option<PeakStat>,
}
