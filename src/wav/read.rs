use crate::audio::buffer2::{Buffer, BufferE};
use crate::audio::manager::Buffers;
use crate::audio::sample;
// use crate::audio::{BufferPool, SampleBuffer};
use crate::audio::SampleType;
use crate::wav::file2::{Channel, File};
use anyhow::{Result, ensure};
use hound;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
#[cfg(target_arch = "wasm32")]
use std::{cell::Cell, rc::Rc};
use thousands::Separable;

pub type ChIx = usize; // Channel index

/// File-based read options (path + optional filters).
#[derive(Debug, Clone, PartialEq)]
pub struct ReadConfig {
    /// Path to wav file to read
    pub filepath: PathBuf,

    /// Indices of channels to read from file, default: all
    pub ch_ixs: Option<Vec<ChIx>>,

    /// Range of samples indices per channel to read, default: all
    // pub sample_range: Option<sample::IxRange>,
    pub sample_range: sample::OptIxRange,
}

/// In-memory read options for wasm drag-and-drop (no filesystem).
#[derive(Debug, Clone, PartialEq)]
pub struct ReadConfigBytes {
    // Optional filename for UI labels; bytes hold the entire file content.
    pub name: Option<String>,
    pub bytes: Vec<u8>,
    pub ch_ixs: Option<Vec<ChIx>>,
    pub sample_range: sample::OptIxRange,
}

/// Shared subset of read options for both file and byte sources.
#[derive(Debug, Clone)]
struct ReadOptions {
    ch_ixs: Option<Vec<ChIx>>,
    sample_range: sample::OptIxRange,
}

/// Fully decoded data, ready to be integrated into the model.
#[derive(Debug)]
pub struct LoadedFile {
    pub load_id: LoadId,
    pub channels: BTreeMap<ChIx, BufferE>,
    pub sample_type: SampleType,
    pub bit_depth: u16,
    pub sample_rate: u32,
    pub layout: Option<crate::audio::Layout>,
    pub path: Option<PathBuf>,
    /// Number of samples per channel
    pub nr_samples: u64,
}

pub type LoadId = u64;

/// Message from loader to UI thread with the decoded file or an error.
pub enum LoadResult {
    Ok(LoadedFile),
    Err {
        load_id: LoadId,
        error: anyhow::Error,
    },
}

/// Stages used by the progress UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStage {
    Start,
    ReadingSamples,
    Deinterleaving,
    Converting,
    Thumbnail,
    Finalizing,
    Done,
}

/// Minimal progress tracking for loaders; atomics on native, Cells on wasm.
#[derive(Debug)]
pub struct LoadProgressAtomic {
    #[cfg(target_arch = "wasm32")]
    stage: Cell<u8>,
    #[cfg(not(target_arch = "wasm32"))]
    stage: AtomicU8,
    #[cfg(target_arch = "wasm32")]
    current: Cell<u64>,
    #[cfg(not(target_arch = "wasm32"))]
    current: AtomicU64,
    #[cfg(target_arch = "wasm32")]
    total: Cell<u64>,
    #[cfg(not(target_arch = "wasm32"))]
    total: AtomicU64,
}

#[cfg(target_arch = "wasm32")]
pub type LoadProgressHandle = Rc<LoadProgressAtomic>;
#[cfg(not(target_arch = "wasm32"))]
pub type LoadProgressHandle = Arc<LoadProgressAtomic>;

/// Wasm doesn't support threads/atomics, so we use Rc/Cell there.
pub fn new_load_progress_handle() -> LoadProgressHandle {
    #[cfg(target_arch = "wasm32")]
    {
        Rc::new(LoadProgressAtomic::new())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Arc::new(LoadProgressAtomic::new())
    }
}

impl ReadConfig {
    pub fn new(filepath: impl Into<PathBuf>) -> Self {
        Self {
            filepath: filepath.into(),
            ch_ixs: None,
            sample_range: sample::OptIxRange::default(),
        }
    }

    pub fn with_ch_ixs(self, ch_ixs: impl Into<Vec<ChIx>>) -> Self {
        Self {
            ch_ixs: Some(ch_ixs.into()),
            ..self
        }
    }

    pub fn with_sample_range(self, sample_range: sample::OptIxRange) -> Self {
        Self {
            sample_range,
            ..self
        }
    }
}

impl ReadConfigBytes {
    pub fn new(name: Option<String>, bytes: Vec<u8>) -> Self {
        Self {
            name,
            bytes,
            ch_ixs: None,
            sample_range: sample::OptIxRange::default(),
        }
    }

    pub fn with_ch_ixs(self, ch_ixs: impl Into<Vec<ChIx>>) -> Self {
        Self {
            ch_ixs: Some(ch_ixs.into()),
            ..self
        }
    }

    pub fn with_sample_range(self, sample_range: sample::OptIxRange) -> Self {
        Self {
            sample_range,
            ..self
        }
    }
}

impl From<&ReadConfig> for ReadOptions {
    fn from(value: &ReadConfig) -> Self {
        Self {
            ch_ixs: value.ch_ixs.clone(),
            sample_range: value.sample_range,
        }
    }
}

impl From<&ReadConfigBytes> for ReadOptions {
    fn from(value: &ReadConfigBytes) -> Self {
        Self {
            ch_ixs: value.ch_ixs.clone(),
            sample_range: value.sample_range,
        }
    }
}

// TODO: think of better name :)
pub fn read_to_file(config: &ReadConfig, buffers: &mut Buffers) -> Result<File> {
    let loaded = read_to_loaded_file(config)?;
    Ok(loaded.into_file(buffers))
}

pub fn read_to_loaded_file(config: &ReadConfig) -> Result<LoadedFile> {
    read_to_loaded_file_with_progress(config, 0, None)
}

pub fn read_to_loaded_file_with_progress(
    config: &ReadConfig,
    load_id: LoadId,
    progress: Option<&LoadProgressAtomic>,
) -> Result<LoadedFile> {
    let Some(filepath) = config.filepath.to_str() else {
        return Err(anyhow::anyhow!("Invalid filepath"));
    };
    let options = ReadOptions::from(config);
    let reader = hound::WavReader::open(&config.filepath)
        .map_err(|err| anyhow::anyhow!("Failed to open wav file '{}': {}", filepath, err))?;
    read_to_loaded_file_from_reader(
        reader,
        &options,
        load_id,
        progress,
        filepath,
        Some(PathBuf::from(&config.filepath)),
    )
}

// Same pipeline as file-based reading, but from an in-memory cursor.
pub fn read_bytes_to_loaded_file_with_progress(
    config: &ReadConfigBytes,
    load_id: LoadId,
    progress: Option<&LoadProgressAtomic>,
) -> Result<LoadedFile> {
    let options = ReadOptions::from(config);
    let label = config.name.as_deref().unwrap_or("bytes");
    let reader = hound::WavReader::new(std::io::Cursor::new(&config.bytes))
        .map_err(|err| anyhow::anyhow!("Failed to open wav bytes '{}': {}", label, err))?;
    read_to_loaded_file_from_reader(
        reader,
        &options,
        load_id,
        progress,
        label,
        config.name.as_deref().map(PathBuf::from),
    )
}

// Shared implementation for file paths and byte buffers.
fn read_to_loaded_file_from_reader<R: std::io::Read + std::io::Seek>(
    mut reader: hound::WavReader<R>,
    options: &ReadOptions,
    load_id: LoadId,
    progress: Option<&LoadProgressAtomic>,
    source_label: &str,
    path: Option<PathBuf>,
) -> Result<LoadedFile> {
    if let Some(progress) = progress {
        progress.set_stage(LoadStage::Start, 0);
    }

    tracing::trace!("Start reading wav file '{}'", source_label);
    #[cfg(not(target_arch = "wasm32"))]
    let start = std::time::Instant::now();

    let spec = reader.spec();
    tracing::trace!("{spec:?}");
    tracing::trace!(
        "Wav duration: {}s ({} samples/ch)",
        format!("{:.1}", reader.duration() as f64 / spec.sample_rate as f64).separate_with_commas(),
        reader.duration().separate_with_commas()
    );

    // read samples into appropriate type and associate with channel index
    let chix_buffers: BTreeMap<ChIx, BufferE> = match spec.sample_format {
        hound::SampleFormat::Float => match spec.bits_per_sample {
            bit_depth if bit_depth <= 32 => convert_samples(
                read_to_buffers::<f32, _>(&mut reader, options, progress)?,
                BufferE::F32,
            ),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported bit depth for float: {}",
                    spec.bits_per_sample
                ));
            }
        },
        hound::SampleFormat::Int => match spec.bits_per_sample {
            bit_depth if bit_depth <= 16 => convert_samples(
                read_to_buffers::<i16, _>(&mut reader, options, progress)?,
                BufferE::I16,
            ),
            bit_depth if bit_depth <= 32 => convert_samples(
                read_to_buffers::<i32, _>(&mut reader, options, progress)?,
                BufferE::I32,
            ),
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported bit depth for int: {}",
                    spec.bits_per_sample
                ));
            }
        },
    };

    if let Some(progress) = progress {
        progress.set_stage(LoadStage::Finalizing, 1);
        progress.set_current(1);
    }

    let file = LoadedFile {
        load_id,
        channels: chix_buffers,
        layout: None, // TODO: first need to extend hound to 'publish' the wavextended channel mask?
        sample_rate: spec.sample_rate,
        bit_depth: spec.bits_per_sample,
        sample_type: spec.sample_format.into(),
        path,
        nr_samples: reader.duration() as u64,
    };

    #[cfg(not(target_arch = "wasm32"))]
    tracing::trace!("Read wav data in {:?}: {}", start.elapsed(), file);
    #[cfg(target_arch = "wasm32")]
    tracing::trace!("Read wav data: {}", file);

    Ok(file)
}

// TODO: maybe use Vec<(ChIx, Vec<T>)> instead of HashMap<ChIx, Vec<T>>?
fn convert_samples<T>(
    samples: BTreeMap<ChIx, T>,
    converter: impl Fn(T) -> BufferE,
) -> BTreeMap<ChIx, BufferE> {
    samples
        .into_iter()
        .map(|(index, sample)| (index, converter(sample)))
        .collect()
}

/// Reads interleaved samples into buffers, honoring channel/range filters.
fn read_to_buffers<S, R>(
    reader: &mut hound::WavReader<R>,
    options: &ReadOptions,
    progress: Option<&LoadProgressAtomic>,
) -> Result<BTreeMap<ChIx, Buffer<S>>>
where
    R: std::io::Read + std::io::Seek,
    S: crate::audio::sample2::Sample + hound::Sample,
{
    let nr_channels = reader.spec().channels as usize;
    let reader_duration = reader.duration() as i64;
    let sample_range = options.sample_range.to_ix_range(0, reader_duration);
    anyhow::ensure!(
        sample_range.end <= reader_duration,
        "sample range end {} is larger than file duration {}",
        sample_range.end,
        reader.duration()
    );

    // Seek to the start position
    if sample_range.start > 0 {
        reader.seek(sample_range.start as u32)?;
    }

    // Read the desired number of interleaved samples
    if let Some(progress) = progress {
        let total = sample_range.len() as u64 * nr_channels as u64;
        progress.set_stage(LoadStage::ReadingSamples, total);
    }
    // Read in one pass, updating progress in coarse steps to reduce overhead.
    let mut interleaved_samples = Vec::with_capacity(sample_range.len() as usize * nr_channels);
    let mut read_count: u64 = 0;
    const READ_UPDATE_STEP: u64 = 1 << 18;
    for sample in reader
        .samples::<S>()
        .take(sample_range.len() as usize * nr_channels)
    {
        interleaved_samples.push(sample?);
        read_count += 1;
        if read_count.is_multiple_of(READ_UPDATE_STEP)
            && let Some(progress) = progress
        {
            progress.set_current(read_count);
        }
    }
    if let Some(progress) = progress {
        progress.set_current(read_count);
    }

    // channel indices we want to deinterleave
    // NOTE: this gets a refernce in to the Option if it has a value, otherwise it gets an owned
    // value for all channels
    let ch_ixs: Cow<'_, [ChIx]> = match options.ch_ixs.as_deref() {
        Some(v) => Cow::Borrowed(v),
        None => Cow::Owned((0..nr_channels).collect()),
    };

    if let Some(progress) = progress {
        progress.set_stage(LoadStage::Deinterleaving, interleaved_samples.len() as u64);
    }
    let channels = deinterleave(&interleaved_samples, nr_channels, &ch_ixs, progress)?;

    // associate spec with buffers
    let spec = reader.spec();
    if let Some(progress) = progress {
        progress.set_stage(LoadStage::Converting, channels.len() as u64);
    }
    let mut converted_count: u64 = 0;
    let buffers = channels
        .into_iter()
        .map(|(ch_ix, samples)| {
            let mut buffer = Buffer::new(spec.sample_rate, spec.bits_per_sample);
            buffer.data = samples;
            converted_count += 1;
            if let Some(progress) = progress {
                progress.set_current(converted_count);
            }
            (ch_ix, buffer)
        })
        .collect();

    Ok(buffers)
}

// Split interleaved/deinterleave samples into per-channel buffers.
// TODO: make version that doesn't allocate new buffers, we might want to reuse the same buffers
// when deinterleaving a large file in batches
fn deinterleave<S>(
    interleaved_samples: &[S],
    nr_channels: usize,
    channel_indices: &[ChIx],
    progress: Option<&LoadProgressAtomic>,
) -> Result<HashMap<ChIx, Vec<S>>>
where
    S: Copy,
{
    let nr_samples_per_ch = interleaved_samples.len() / nr_channels;
    ensure!(
        nr_samples_per_ch > 0,
        format!("{:?} {:?}", interleaved_samples.len(), nr_channels)
    );

    let nr_channels_to_deinterleave = channel_indices.len();
    ensure!(nr_channels_to_deinterleave > 0);
    ensure!(nr_channels_to_deinterleave <= nr_channels);

    let mut result = HashMap::new();

    // pre-allocate output
    for ch_ix in channel_indices.iter() {
        let ch_ix = *ch_ix;
        ensure!(
            ch_ix < nr_channels,
            format!("{:?} {:?}", ch_ix, nr_channels)
        );
        result.insert(ch_ix, Vec::with_capacity(nr_samples_per_ch));
    }

    // deinterleave
    // PERF: looping over each channel and 'striding' through the deinterleaved buffer, probably faster.
    let mut deinterleave_count: u64 = 0;
    const DEINTERLEAVE_UPDATE_STEP: u64 = 1 << 19;
    for ch_ix in channel_indices.iter() {
        let ch_ix = *ch_ix;
        ensure!(
            ch_ix < nr_channels,
            format!("{:?} {:?}", ch_ix, nr_channels)
        );
        let buffer = &mut result.get_mut(&ch_ix).unwrap();
        for sample_ix in 0..nr_samples_per_ch {
            buffer.push(interleaved_samples[sample_ix * nr_channels + ch_ix]);
            deinterleave_count += 1;
            if deinterleave_count.is_multiple_of(DEINTERLEAVE_UPDATE_STEP)
                && let Some(progress) = progress
            {
                progress.set_current(deinterleave_count);
            }
        }
    }
    if let Some(progress) = progress {
        progress.set_current(deinterleave_count);
    }

    Ok(result)
}

impl std::fmt::Display for LoadedFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LoadedFile:")?;
        write!(f, " path: {:?}", self.path)?;
        write!(f, ", nr_channels: {}", self.channels.len())?;
        write!(f, ", sample_type: {:?}", self.sample_type)?;
        write!(f, ", bit_depth: {}", self.bit_depth)?;
        write!(f, ", sample_rate: {}", self.sample_rate)?;
        if let Some(layout) = &self.layout {
            write!(f, ", layout: {:?}", layout)?;
        }
        write!(f, ", nr_samples: {}", self.nr_samples)?;
        Ok(())
    }
}

impl LoadedFile {
    pub fn into_file(self, buffers: &mut Buffers) -> File {
        File {
            // move buffers to storage and store it's id in file
            channels: self
                .channels
                .into_iter()
                .map(|(ch_ix, buffer)| {
                    (
                        ch_ix,
                        Channel {
                            ch_ix,
                            buffer_id: buffers.insert(buffer),
                            channel_id: None,
                        },
                    )
                })
                .collect(),
            layout: self.layout,
            sample_rate: self.sample_rate,
            bit_depth: self.bit_depth,
            sample_type: self.sample_type,
            path: self.path,
            nr_samples: self.nr_samples,
        }
    }
}

impl LoadProgressAtomic {
    pub fn new() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            stage: Cell::new(LoadStage::Start as u8),
            #[cfg(not(target_arch = "wasm32"))]
            stage: AtomicU8::new(LoadStage::Start as u8),
            #[cfg(target_arch = "wasm32")]
            current: Cell::new(0),
            #[cfg(not(target_arch = "wasm32"))]
            current: AtomicU64::new(0),
            #[cfg(target_arch = "wasm32")]
            total: Cell::new(0),
            #[cfg(not(target_arch = "wasm32"))]
            total: AtomicU64::new(0),
        }
    }

    pub fn set_stage(&self, stage: LoadStage, total: u64) {
        #[cfg(target_arch = "wasm32")]
        {
            self.stage.set(stage as u8);
            self.total.set(total);
            self.current.set(0);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.stage.store(stage as u8, Ordering::Release);
            self.total.store(total, Ordering::Release);
            self.current.store(0, Ordering::Release);
        }
    }

    pub fn set_current(&self, current: u64) {
        #[cfg(target_arch = "wasm32")]
        {
            self.current.set(current);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.current.store(current, Ordering::Release);
        }
    }

    pub fn snapshot(&self) -> (LoadStage, u64, u64) {
        #[cfg(target_arch = "wasm32")]
        let stage = LoadStage::from_u8(self.stage.get());
        #[cfg(not(target_arch = "wasm32"))]
        let stage = LoadStage::from_u8(self.stage.load(Ordering::Acquire));
        #[cfg(target_arch = "wasm32")]
        let current = self.current.get();
        #[cfg(not(target_arch = "wasm32"))]
        let current = self.current.load(Ordering::Acquire);
        #[cfg(target_arch = "wasm32")]
        let total = self.total.get();
        #[cfg(not(target_arch = "wasm32"))]
        let total = self.total.load(Ordering::Acquire);
        (stage, current, total)
    }
}

impl Default for LoadProgressAtomic {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadStage {
    pub fn from_u8(value: u8) -> Self {
        match value {
            1 => LoadStage::ReadingSamples,
            2 => LoadStage::Deinterleaving,
            3 => LoadStage::Converting,
            4 => LoadStage::Thumbnail,
            5 => LoadStage::Finalizing,
            6 => LoadStage::Done,
            _ => LoadStage::Start,
        }
    }
}

// fn deinterleave_samples<S>(interleaved_samples: &[S], nr_channels: usize) -> Result<Vec<Vec<S>>>
// where
//     S: Copy,
// {
//     let nr_samples_per_ch = interleaved_samples.len() / nr_channels;
//     ensure!(nr_samples_per_ch > 0, format!("{:?} {:?}", interleaved_samples.len(), nr_channels));

//     // pre-allocate
//     let mut result = Vec::with_capacity(nr_channels);
//     for _ in 0..nr_channels {
//         result.push(Vec::with_capacity(nr_samples_per_ch));
//     }

//     // deinterleave
//     // PERF: looping over each channel and 'striding' through the deinterleaved buffer.
//     for ch_ix in 0..nr_channels {
//         let buffer = &mut result[ch_ix];
//         for sample_ix in 0..nr_samples_per_ch {
//             buffer.push(interleaved_samples[sample_ix * nr_channels + ch_ix]);
//         }
//     }
//     Ok(result)
// }

// fn deinterleave_samples_for_channels<S>(interleaved_samples: &[S], nr_channels: usize, channels: &[usize]) -> Result<HashMap<usize, Vec<S>>>
// where
//     S: Copy,
// {
//     let nr_samples_per_ch = interleaved_samples.len() / nr_channels;
//     ensure!(nr_samples_per_ch > 0, format!("{:?} {:?}", interleaved_samples.len(), nr_channels));

//     let nr_channels_to_deinterleave = channels.len();
//     ensure!(nr_channels_to_deinterleave > 0);
//     ensure!(nr_channels_to_deinterleave <= nr_channels);

//     let mut result = HashMap::new();

//     // pre-allocate output
//     for ch_ix in channels.iter() {
//         let ch_ix = *ch_ix;
//         ensure!(ch_ix < nr_channels, format!("{:?} {:?}", ch_ix, nr_channels));
//         result.insert(ch_ix, Vec::with_capacity(nr_samples_per_ch));
//     }

//     // deinterleave
//     // PERF: looping over each channel and 'striding' through the deinterleaved buffer, probably faster.
//     for ch_ix in channels.iter() {
//         let ch_ix = *ch_ix;
//         ensure!(ch_ix < nr_channels, format!("{:?} {:?}", ch_ix, nr_channels));
//         let buffer = &mut result.get_mut(&ch_ix).unwrap();
//         for sample_ix in 0..nr_samples_per_ch {
//             buffer.push(interleaved_samples[sample_ix * nr_channels + ch_ix]);
//         }
//     }

//     Ok(result)
// }

// fn deinterleave_samples_in_buffers<S>(
//     interleaved_samples: &[S],
//     nr_channels: usize,
//     sample_rate: u32,
//     bit_depth: u16,
// ) -> Result<Vec<Buffer<S>>>
// where
//     S: crate::audio::buffer2::Sample,
// {
//     let nr_samples = interleaved_samples.len() / nr_channels;
//     ensure!(
//         nr_samples > 0,
//         format!(
//             "Number of samples is zero, cannot deinterleave {} samples to {} channels",
//             interleaved_samples.len(),
//             nr_channels
//         )
//     );

//     // pre-allocate buffers
//     let mut buffers: Vec<Buffer<S>> = (0..nr_channels)
//         .map(|_| Buffer::with_size(sample_rate, bit_depth, nr_samples))
//         .collect();

//     // deinterleave into buffers
//     // NOTE: looping over each channel and 'striding' through the deinterleaved buffer, probably faster.
//     for ch_ix in 0..nr_channels {
//         let buffer = &mut buffers[ch_ix];
//         for sample_ix in 0..nr_samples {
//             buffer[sample_ix] = interleaved_samples[sample_ix * nr_channels + ch_ix];
//         }
//     }
//     Ok(buffers)
// }

// deinterleave samples for specific channel indices
// fn deinterleave_samples_for_channels_into_buffers<S>(
//     interleaved_samples: &[S],
//     nr_channels: usize,
//     sample_rate: u32,
//     bit_depth: u16,
//     channels: &[usize],
// ) -> Result<HashMap<usize, Buffer<S>>>
// where
//     S: crate::audio::buffer2::Sample,
// {
//     let nr_samples_per_ch = interleaved_samples.len() / nr_channels;
//     ensure!(nr_samples_per_ch > 0, format!("{:?} {:?}", interleaved_samples.len(), nr_channels));

//     let nr_channels_to_deinterleave = channels.len();
//     ensure!(nr_channels_to_deinterleave > 0);
//     ensure!(nr_channels_to_deinterleave <= nr_channels);

//     let mut result = HashMap::new();

//     // pre-allocate buffers
//     for ch_ix in channels.iter() {
//         let ch_ix = *ch_ix;
//         ensure!(ch_ix < nr_channels, format!("{:?} {:?}", ch_ix, nr_channels));
//         let buffer = Buffer::with_size(sample_rate, bit_depth, nr_samples_per_ch);
//         result.insert(ch_ix, buffer);
//     }

//     // deinterleave into buffers
//     // NOTE: looping over each channel and 'striding' through the deinterleaved buffer, probably faster.
//     for ch_ix in 0..nr_channels {
//         if !channels.contains(&ch_ix) {
//             continue;
//         }

//         let buffer = &mut result.get_mut(&ch_ix).unwrap();
//         for sample_ix in 0..nr_samples_per_ch {
//             buffer[sample_ix] = interleaved_samples[sample_ix * nr_channels + ch_ix];
//         }
//     }
//     Ok(result)
// }

// fn read_and_deinterleave_samples<S>(reader: &mut hound::WavReader<std::io::BufReader<std::fs::File>>) -> Result<Vec<Buffer<S>>>
// where
//     S: hound::Sample + crate::audio::buffer2::Sample,
// {
//     // read samples into interleaved vector
//     let spec = reader.spec();
//     let interleaved_samples: Vec<S> = reader.samples::<S>().map(|s| s.unwrap()).collect();
//     let nr_samples = interleaved_samples.len() / spec.channels as usize;
//     let nr_channels = spec.channels as usize;

//     let vectors = deinterleave_samples(&interleaved_samples, nr_channels)?;

//     let mut result = Vec::with_capacity(nr_channels);
//     for (ch_ix, vector) in vectors.into_iter().enumerate() {
//         let mut buffer = Buffer::<S>::new(spec.sample_rate, spec.bits_per_sample);
//         buffer.data = vector;
//         result.push(buffer);
//     }
//     Ok(result)
// }

// fn read_and_deinterleave_samples_for_channels<S>(
//     reader: &mut hound::WavReader<std::io::BufReader<std::fs::File>>,
//     channels: &[usize],
// ) -> Result<HashMap<usize, Buffer<S>>>
// where
//     S: hound::Sample + crate::audio::buffer2::Sample,
// {
//     let nr_channels_to_deinterleave = channels.len();
//     ensure!(nr_channels_to_deinterleave > 0);
//     ensure!(nr_channels_to_deinterleave <= reader.spec().channels as usize);

//     let spec = reader.spec();
//     let interleaved_samples: Vec<S> = reader.samples::<S>().map(|s| s.unwrap()).collect();
//     let nr_samples = interleaved_samples.len() / spec.channels as usize;
//     let nr_channels = spec.channels as usize;

//     let vectors = deinterleave_samples_for_channels(&interleaved_samples, nr_channels, channels)?;

//     let mut result = HashMap::new();
//     for (ix, vector) in vectors.into_iter() {
//         let mut buffer = Buffer::<S>::new(spec.sample_rate, spec.bits_per_sample);
//         let wav_ch_ix = &channels[ix];
//         buffer.data = vector;
//         result.insert(*wav_ch_ix, buffer);
//     }

//     Ok(result)
// }

// TODO: read part of file into File
// pub fn read_wav_file_to_buffers(path: &str, buffer_pool: &mut BufferPool) -> Result<File> {
//     // open wav file with hound
//     let mut reader = hound::WavReader::open(path).map_err(|err| anyhow::anyhow!("Failed to open wav file '{}': {}", path, err))?;
//     let spec = reader.spec();
//     dbg!(spec);

//     let buffers: Vec<SampleBuffer> = match spec.sample_format {
//         hound::SampleFormat::Float => read_and_deinterleave_samples::<f32>(&mut reader)?
//             .into_iter()
//             .map(SampleBuffer::F32)
//             .collect(),
//         hound::SampleFormat::Int => match spec.bits_per_sample {
//             bps if bps <= 16 => read_and_deinterleave_samples::<i16>(&mut reader)?
//                 .into_iter()
//                 .map(SampleBuffer::I16)
//                 .collect(),
//             bps if bps <= 32 => read_and_deinterleave_samples::<i32>(&mut reader)?
//                 .into_iter()
//                 .map(SampleBuffer::I32)
//                 .collect(),
//             _ => {
//                 return Err(anyhow::anyhow!(
//                     "Unsupported bit depth for integer format: {}",
//                     spec.bits_per_sample
//                 ));
//             }
//         },
//     };

//     let mut new_file = File {
//         channels: Vec::<Channel>::with_capacity(buffers.len()),
//         layout: None,
//         sample_rate: spec.sample_rate,
//         bit_depth: spec.bits_per_sample,
//         sample_type: spec.sample_format.into(),
//     };

//     // move buffers to pool and store id in new_file
//     for (i, buffer) in buffers.into_iter().enumerate() {
//         new_file.channels.push(Channel {
//             ch_ix: i,
//             buffer: buffer_pool.add_buffer(buffer),
//             channel_id: None,
//         });
//     }
//     return Ok(new_file);
// }

// pub fn read_wav_file_to_float(path: &str) -> Result<audio::Buffer<f32>> {
//     // open wav file with hound
//     let mut reader = hound::WavReader::open(path).map_err(|err| anyhow::anyhow!("Failed to open wav file '{}': {}", path, err))?;
//     let spec = reader.spec();
//     dbg!(spec);

//     // let debug_take = Some(100);
//     let debug_take = usize::MAX;
//     // let debug_take = None;

//     // read float or int samples into float interleaved vector
//     let interleaved_samples: Vec<f32> = match spec.sample_format {
//         hound::SampleFormat::Float => {
//             let iter = reader.samples::<f32>();
//             let iter = iter.take(debug_take);
//             iter.map(|s| s.unwrap()).collect()
//         }
//         hound::SampleFormat::Int => {
//             ensure!(
//                 spec.bits_per_sample <= 24,
//                 "integer PCM with bitdepth {} cannot be represented exactly as float, loss of precision may occur",
//                 spec.bits_per_sample
//             );
//             let iter = reader.samples::<i32>();
//             let iter = iter.take(debug_take);
//             iter.map(|s| s.unwrap() as f32).collect()
//         }
//     };

//     // deinterleaved storage
//     let mut buffer = audio::BufferBuilder::new()
//         .nr_channels(spec.channels.into())
//         // .nr_samples(reader.duration() as usize)
//         .nr_samples(interleaved_samples.len() / spec.channels as usize)
//         .sample_rate(spec.sample_rate)
//         .bit_depth(spec.bits_per_sample.into())
//         .sample_type(spec.sample_format.into())
//         .build::<f32>()?;

//     // deinterleave
//     // NOTE: looping over each channel and 'striding' through the deinterleaved buffer, probably faster
//     // than loopin over the interleaved buffer and jumping between channels?
//     // Maybe it is better to use the iterator of the file and do everyting in one go?
//     for (ch_ix, ch) in buffer.channels_mut().enumerate() {
//         for (sample_ix, sample) in ch.samples_mut().enumerate() {
//             *sample = interleaved_samples[sample_ix * spec.channels as usize + ch_ix];
//         }
//     }

//     Ok(buffer)
// }

mod tests {

    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use crate::util;

    // #[test]
    // fn test_read_wav_file_to_float() {
    //     #[allow(dead_code)]
    //     let buf = read_wav_file_to_float("/home/emile/repos/rust/wavalyze/data/sine_16_signed_48000_1.wav").unwrap();
    //     // println!("{:?}", buf);
    // }

    // #[test]
    // fn test_read_wav_file_to_buffers() {
    //     let mut pool = BufferPool::new();

    //     // Test with signed 16-bit integer file
    //     let file_i16 = read_wav_file_to_buffers("/home/emile/repos/rust/wavalyze/data/sine_16_signed_48000_1.wav", &mut pool).unwrap();

    //     assert_eq!(file_i16.sample_rate, 48000);
    //     assert_eq!(file_i16.bit_depth, 16);
    //     assert_eq!(file_i16.channels.len(), 1);
    //     let buffer_id_i16 = file_i16.channels[0].buffer;
    //     let sample_buffer_i16 = pool.get_buffer(buffer_id_i16).unwrap();
    //     assert!(matches!(sample_buffer_i16, SampleBuffer::I16(_)));

    //     // Test with 32-bit float file
    //     let file_f32 = read_wav_file_to_buffers(
    //         "/home/emile/repos/rust/wavalyze/data/sine_32_float_48000_1.wav", // Assuming this file exists
    //         &mut pool,
    //     )
    //     .unwrap();

    //     assert_eq!(file_f32.sample_rate, 48000);
    //     assert_eq!(file_f32.bit_depth, 32);
    //     assert_eq!(file_f32.channels.len(), 1);
    //     let buffer_id_f32 = file_f32.channels[0].buffer;
    //     let sample_buffer_f32 = pool.get_buffer(buffer_id_f32).unwrap();
    //     assert!(matches!(sample_buffer_f32, SampleBuffer::F32(_)));
    // }
}
