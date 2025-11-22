use anyhow::anyhow;
use anyhow::{ensure, Result};
use std::collections::BTreeMap;
use std::fmt;
use tracing::{debug, instrument};

use itertools::Itertools;

use crate::audio::buffer2::Buffer;
use crate::audio::sample::{self, MinMaxSamples};
use crate::audio::sample2::Sample;

type SampPerPix = u64;

#[derive(Debug, Clone)]
pub enum ThumbnailE {
    F32(Thumbnail<f32>),
    I32(Thumbnail<i32>),
    I16(Thumbnail<i16>),
}

/// Represents the sample values for a single zoom level
/// A set of min/max values per pixel column
#[derive(Debug, Clone)]
pub struct LevelData<T: Sample> {
    pub samples_per_pixel: u64,
    pub data: Vec<sample::ValRange<T>>,
}

impl<T: Sample> LevelData<T> {
    // TODO: we should make all the level data in one pass, not one pass per zoom level
    pub fn from_buffer(buffer: &Buffer<T>, samples_per_pixel: u64) -> Self {
        let mut result = Self {
            samples_per_pixel,
            data: vec![],
        };
        for chunk in &buffer.iter().chunks(samples_per_pixel as usize) {
            let mut min_max = sample::ValRange::<T> { min: T::MAX, max: T::MIN };
            for &sample in chunk {
                min_max.min = min_max.min.min(sample);
                min_max.max = min_max.max.max(sample);
            }
            result.data.push(min_max);
        }
        result
    }

    pub fn from_level_data(level_data: &LevelData<T>, samples_per_pixel: u64) -> Self {
        assert!(samples_per_pixel > level_data.samples_per_pixel);

        let ratio = samples_per_pixel as f64 / level_data.samples_per_pixel as f64;
        let mut result = Self {
            samples_per_pixel,
            data: vec![],
        };
        for chunk in &level_data.data.iter().chunks(level_data.samples_per_pixel as usize) {
            let mut min_max = sample::ValRange::<T> { min: T::MAX, max: T::MIN };
            for val_range in chunk {
                min_max.min = min_max.min.min(val_range.min);
                min_max.max = min_max.max.max(val_range.max);
            }
            result.data.push(min_max);
        }
        result
    }
}

impl<T: Sample> LevelData<T> {
    pub fn get_sample_view(&self, sample_ix_range: sample::IxRange, samples_per_pixel: f64) -> Result<sample::ViewData<T>> {
        ensure!(
            samples_per_pixel >= self.samples_per_pixel as f64,
            "We can only zoom out, i.e. more samples per pixel"
        );

        let ratio = samples_per_pixel / self.samples_per_pixel as f64;
        dbg!(ratio);
        let nr_out_samples = sample_ix_range.len() as f64 / ratio;
        let nr_samples_available = (self.data.len() as i64 * self.samples_per_pixel as i64) - sample_ix_range.start;
        let mut out_mm = MinMaxSamples::<T>::with_capacity(nr_out_samples as usize);
        out_mm.extend(
            self.data
                .iter()
                .skip(self.get_ix(sample_ix_range.start))
                .enumerate()
                .chunk_by(|(ix, _)| ((*ix as f64) / ratio).floor() as usize)
                .into_iter()
                .map(|(_out_view_ix, group)| {
                    group.fold(
                        sample::ValRange::<T> { min: T::MAX, max: T::MIN },
                        |mut min_max, (_ix, val_range)| {
                            min_max.min = min_max.min.min(val_range.min);
                            min_max.max = min_max.max.max(val_range.max);
                            min_max
                        },
                    )
                }),
        );

        println!("nr_out_samples: {}", nr_out_samples);
        println!("out_mm.len(): {}", out_mm.len());
        println!("nr_samples_available: {}", nr_samples_available);
        let nr_out_samples_available = nr_samples_available as f64 / ratio;
        println!("nr_out_samples_available: {}", nr_out_samples_available);
        Ok(sample::ViewData::<T>::MinMax(out_mm))
    }

    pub fn get_ix(&self, sample_ix: sample::SampleIx) -> usize {
        (sample_ix as f64 / self.samples_per_pixel as f64).floor() as usize
    }
}

#[derive(Debug, Clone)]
pub struct ThumbnailConfig {
    /// Step size to determine each zoom level
    pub samples_per_pixel_delta: u64,
    pub min_nr_level_data_size: usize,
}

impl Default for ThumbnailConfig {
    fn default() -> Self {
        Self {
            samples_per_pixel_delta: 128,
            min_nr_level_data_size: 1024 * 4, // lowest resolution
        }
    }
}

///
/// Cache for downsampled data for drawing zoomed out audio samples.
///
/// We only ever need two sample values per pixel, i.e. the min/max sample values for all the
/// samples that coincide with a specific pixel column.
/// Running through all the samples that are on screen, when zoomed out enough, gets expensive. So
/// we cache min/max values for a set of zoom levels aka samples_per_pixel.
///
/// Named after JUCE's AudioThumbnail
#[derive(Debug, Clone)]
pub struct Thumbnail<T: Sample> {
    pub level_data: BTreeMap<SampPerPix, LevelData<T>>,
}

impl<T: Sample> Thumbnail<T> {
    pub fn from_buffer(buffer: &Buffer<T>, config: ThumbnailConfig) -> Self {
        let mut result = Self {
            level_data: BTreeMap::new(),
        };

        let mut samp_per_pix = config.samples_per_pixel_delta;
        loop {
            let level_data = LevelData::from_buffer(buffer, samp_per_pix);
            result.level_data.insert(samp_per_pix, level_data.clone());

            samp_per_pix *= 2;

            if level_data.data.len() <= config.min_nr_level_data_size || level_data.data.len() == 1 {
                break;
            }
        }
        result
    }
}

impl<T: Sample> Thumbnail<T> {
    #[instrument(skip(self), fields(self = %self))]
    pub fn get_sample_view(&self, sample_ix_range: sample::IxRange, samples_per_pixel: f64) -> Result<sample::ViewData<T>> {
        let spp_requested = samples_per_pixel as u64;

        // Find level data with ssp_key smaller than and closest to the given ssp_key
        let (spp_source, level_data) = self.level_data.range(..spp_requested).next_back().ok_or_else(|| {
            // Safely reference the smallest key if it exists, else fallback message:
            let min_spp = self.level_data.keys().next().map(|k| k.to_string()).unwrap_or("<empty>".into());
            anyhow!(
                "No level data available for samples_per_pixel {} (min is {})",
                samples_per_pixel,
                min_spp
            )
        })?;

        debug!(spp_source, spp_requested);

        level_data.get_sample_view(sample_ix_range, samples_per_pixel)
    }
}

impl<T: Sample> fmt::Display for Thumbnail<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Thumbnail:")?;
        for (samples_per_pixel, level_data) in &self.level_data {
            writeln!(f, "  samples_per_pixel {}: {} values", samples_per_pixel, level_data.data.len())?;
        }
        Ok(())
    }
}
