// use anyhow::anyhow;
// use anyhow::{ensure, Result};
use std::collections::BTreeMap;
use std::fmt;
// use tracing::{debug, instrument};

use itertools::Itertools;

use crate::audio::buffer2::{Buffer, BufferE};
use crate::audio::sample::{self};
use crate::audio::sample2::Sample;

type SampPerPix = u64;

#[derive(Debug, Clone)]
pub enum ThumbnailE {
    F32(Thumbnail<f32>),
    I32(Thumbnail<i32>),
    I16(Thumbnail<i16>),
}

impl ThumbnailE {
    /// Get the level data for the given samples per pixel (closest smaller or equal)
    pub fn get_level_data(&self, samples_per_pixel: f32) -> Option<LevelDataERef<'_>> {
        match self {
            ThumbnailE::F32(thumbnail) => pick_level(thumbnail, samples_per_pixel as u64).map(LevelDataERef::F32),
            ThumbnailE::I32(thumbnail) => pick_level(thumbnail, samples_per_pixel as u64).map(LevelDataERef::I32),
            ThumbnailE::I16(thumbnail) => pick_level(thumbnail, samples_per_pixel as u64).map(LevelDataERef::I16),
        }
    }
    pub fn from_buffer_e(buffer: &BufferE, config: Option<ThumbnailConfig>) -> Self {
        match buffer {
            BufferE::F32(buffer) => ThumbnailE::F32(Thumbnail::from_buffer(buffer, config)),
            BufferE::I32(buffer) => ThumbnailE::I32(Thumbnail::from_buffer(buffer, config)),
            BufferE::I16(buffer) => ThumbnailE::I16(Thumbnail::from_buffer(buffer, config)),
        }
    }

    pub fn get_smallest_samples_per_pixel(&self) -> Option<u64> {
        match self {
            ThumbnailE::F32(thumbnail) => thumbnail.get_smallest_samples_per_pixel(),
            ThumbnailE::I32(thumbnail) => thumbnail.get_smallest_samples_per_pixel(),
            ThumbnailE::I16(thumbnail) => thumbnail.get_smallest_samples_per_pixel(),
        }
    }
}

#[derive(Debug)]
pub enum LevelDataERef<'a> {
    F32(&'a LevelData<f32>),
    I32(&'a LevelData<i32>),
    I16(&'a LevelData<i16>),
}

/// Represents the sample values for a single zoom level
/// A set of min/max values per pixel column
#[derive(Debug, Clone)]
pub struct LevelData<T: Sample> {
    pub samples_per_pixel: f64,
    pub data: Vec<sample::ValRange<T>>,
}
impl<T: Sample> LevelData<T> {
    /// Convert an index into data to a smallest sample index (in the original buffer)
    pub fn ix_to_sample_ix(&self, ix: usize) -> usize {
        (ix as f64 * self.samples_per_pixel).floor() as usize
    }
}

impl<T: Sample> LevelData<T> {
    // TODO: we should make all the level data in one pass, not one pass per zoom level
    pub fn from_buffer(buffer: &[T], samples_per_pixel: u64) -> Self {
        let mut result = Self {
            samples_per_pixel: samples_per_pixel as f64,
            data: vec![],
        };
        for chunk in &buffer.iter().chunks(samples_per_pixel as usize) {
            let mut min_max = sample::ValRange::<T> { min: T::MAX, max: T::MIN };
            for &sample in chunk {
                min_max.include(sample);
            }
            result.data.push(min_max);
        }
        // tracing::trace!("Created LevelData from buffer, spp: {}, data.len(): {}", samples_per_pixel, result.data.len());
        result
    }

    pub fn from_level_data(level_data: &LevelData<T>, samples_per_pixel: u64) -> Self {
        tracing::trace!(
            "Creating LevelData from level_data, spp: {}, ld.len(): {}, ld.spp: {}",
            samples_per_pixel,
            level_data.data.len(),
            level_data.samples_per_pixel
        );
        assert!(samples_per_pixel as f64 > level_data.samples_per_pixel);

        let ratio = samples_per_pixel as f64 / level_data.samples_per_pixel;
        let mut result = Self {
            samples_per_pixel: samples_per_pixel as f64,
            data: vec![],
        };
        result.data.reserve(level_data.data.len() / ratio as usize);
        for chunk in &level_data.data.iter().chunks(ratio as usize) {
            let mut min_max = sample::ValRange::<T> { min: T::MAX, max: T::MIN };
            for val_range in chunk {
                min_max.min = min_max.min.min(val_range.min);
                min_max.max = min_max.max.max(val_range.max);
            }
            result.data.push(min_max);
        }
        result
    }
    pub fn from_buffer_fractional_2(buffer: &[T], sample_ix_range: sample::IxRange, samples_per_pixel: f64) -> Self {
        let mut res = Self {
            samples_per_pixel,
            data: vec![],
        };
        res.from_buffer_fractional(buffer, sample_ix_range, samples_per_pixel);
        res
    }

    pub fn from_buffer_fractional(&mut self, buffer: &[T], sample_ix_range: sample::IxRange, samples_per_pixel: f64) {
        let start_ix = sample_ix_range.start.max(0) as usize;
        let end_ix = sample_ix_range.end.max(0).min(buffer.len() as i64) as usize;

        if start_ix == end_ix {
            return;
        }

        let mut cur_min_max = sample::ValRange::<T> { min: T::MAX, max: T::MIN };
        self.data.clear();
        self.data.reserve((end_ix - start_ix) / samples_per_pixel as usize);

        let mut cur_ix_out = 0;
        buffer
            .iter()
            .skip(start_ix)
            .take(end_ix - start_ix)
            .enumerate()
            .for_each(|(ix_in, val)| {
                let ix_out = ix_in as f64 / samples_per_pixel;
                let ix_out = ix_out.floor() as usize;
                if ix_out == cur_ix_out {
                    cur_min_max.include(*val);
                } else {
                    self.data.push(cur_min_max);
                    cur_min_max = sample::ValRange { min: *val, max: *val };
                    cur_ix_out = ix_out;
                }
            });
    }
}

impl<T: Sample> LevelData<T> {
    // TODO: need to redo this? ViewData is not positions only
    // pub fn get_sample_view(&self, sample_ix_range: sample::IxRange, samples_per_pixel: f64) -> Result<sample::ViewData<T>> {
    //     ensure!(
    //         samples_per_pixel >= self.samples_per_pixel as f64,
    //         "We can only zoom out, i.e. more samples per pixel"
    //     );

    //     let ratio = samples_per_pixel / self.samples_per_pixel as f64;
    //     dbg!(ratio);
    //     let nr_out_samples = sample_ix_range.len() as f64 / ratio;
    //     let nr_samples_available = (self.data.len() as i64 * self.samples_per_pixel as i64) - sample_ix_range.start;
    //     let mut out_mm = MinMaxSamples::<T>::with_capacity(nr_out_samples as usize);
    //     out_mm.extend(
    //         self.data
    //             .iter()
    //             .skip(self.get_ix(sample_ix_range.start))
    //             .enumerate()
    //             .chunk_by(|(ix, _)| ((*ix as f64) / ratio).floor() as usize)
    //             .into_iter()
    //             .map(|(_out_view_ix, group)| {
    //                 group.fold(
    //                     sample::ValRange::<T> { min: T::MAX, max: T::MIN },
    //                     |mut min_max, (_ix, val_range)| {
    //                         min_max.min = min_max.min.min(val_range.min);
    //                         min_max.max = min_max.max.max(val_range.max);
    //                         min_max
    //                     },
    //                 )
    //             }),
    //     );

    //     println!("nr_out_samples: {}", nr_out_samples);
    //     println!("out_mm.len(): {}", out_mm.len());
    //     println!("nr_samples_available: {}", nr_samples_available);
    //     let nr_out_samples_available = nr_samples_available as f64 / ratio;
    //     println!("nr_out_samples_available: {}", nr_out_samples_available);
    //     Ok(sample::ViewData::<T>::MinMax(out_mm))
    // }

    pub fn get_ix(&self, sample_ix: sample::Ix) -> usize {
        (sample_ix as f64 / self.samples_per_pixel).floor() as usize
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
            samples_per_pixel_delta: 64,
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
    pub fn from_buffer(buffer: &Buffer<T>, config: Option<ThumbnailConfig>) -> Self {
        let config = config.unwrap_or_default();
        let mut level_data = BTreeMap::new();

        tracing::trace!("Creating Thumbnail from buffer with nr_samples: {}", buffer.nr_samples());

        let do_from_buffer = false;

        if !do_from_buffer {
            let mut spp = config.samples_per_pixel_delta;
            let mut ld = LevelData::from_buffer(buffer, spp);
            loop {
                let ld_len = ld.data.len();
                if ld_len <= config.min_nr_level_data_size {
                    level_data.insert(spp, ld);
                    break;
                }
                spp *= 2;
                let new_ld = LevelData::from_level_data(&ld, spp);
                // This moves the data into level_data, so we have to calulate
                // the new one before adding
                level_data.insert(ld.samples_per_pixel as u64, ld);
                ld = new_ld;
            }
        } else {
            let mut samp_per_pix = config.samples_per_pixel_delta;

            // PERF: use from_level_data on each result
            // PERF: do all/multiple levels in one pass? Block-wise
            // PERF: use threading
            //  * maybe above this, each channel of a file can load in a different thread
            //  * or split track in parts and process in parallel for each level (with min size per
            //    work item)
            loop {
                let ld = LevelData::from_buffer(buffer, samp_per_pix);
                let len = ld.data.len();

                level_data.insert(samp_per_pix, ld);

                if len <= config.min_nr_level_data_size || len == 1 {
                    break;
                }

                samp_per_pix *= 2;
            }
        }

        let res = Self { level_data };
        tracing::trace!("{res}");
        res
    }
}

impl<T: Sample> Thumbnail<T> {
    // #[instrument(skip(self), fields(self = %self))]
    // pub fn get_sample_view(&self, sample_ix_range: sample::IxRange, samples_per_pixel: f64) -> Result<sample::ViewData<T>> {
    //     let spp_requested = samples_per_pixel as u64;

    //     // Find level data with ssp_key smaller than and closest to the given ssp_key
    //     let (spp_source, level_data) = self.level_data.range(..spp_requested).next_back().ok_or_else(|| {
    //         // Safely reference the smallest key if it exists, else fallback message:
    //         let min_spp = self.level_data.keys().next().map(|k| k.to_string()).unwrap_or("<empty>".into());
    //         anyhow!(
    //             "No level data available for samples_per_pixel {} (min is {})",
    //             samples_per_pixel,
    //             min_spp
    //         )
    //     })?;

    //     debug!(spp_source, spp_requested);

    //     level_data.get_sample_view(sample_ix_range, samples_per_pixel)
    // }

    pub fn get_smallest_samples_per_pixel(&self) -> Option<u64> {
        self.level_data.keys().next().copied()
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

/// Get the level data that has a samples_per_pixel smaller or equal and the closest to the given
fn pick_level<T: Sample>(t: &Thumbnail<T>, spp: SampPerPix) -> Option<&LevelData<T>> {
    t.level_data.range(..=spp).next_back().map(|(_, v)| v)
}
