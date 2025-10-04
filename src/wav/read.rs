use crate::audio;
use crate::audio::buffer2::Buffer;
use crate::audio::{BufferPool, SampleBuffer};
use crate::wav::file2::{Channel, File};

use anyhow::{ensure, Result};
use hound;
use std::collections::HashMap;
use std::ops::Range;

pub type ChIx = usize; // Channel index

pub struct ReadConfig {
    /// Path to wav file to read
    pub filepath: String,

    /// Indices of channels to read from file, default: all
    pub ch_ixs: Option<Vec<ChIx>>,

    /// Range of samples indices per channel to read, default: all
    pub sample_range: Option<Range<usize>>,
}

pub fn read_to_file(config: ReadConfig, buffer_pool: &mut BufferPool) -> Result<File> {
    // open wav file with hound
    let mut reader = hound::WavReader::open(&config.filepath)
        .map_err(|err| anyhow::anyhow!("Failed to open wav file '{}': {}", &config.filepath, err))?;
    let spec = reader.spec();
    dbg!(spec);

    // read samples into appropriate type
    let buffers: HashMap<ChIx, SampleBuffer> = match spec.sample_format {
        hound::SampleFormat::Float => match spec.bits_per_sample {
            bit_depth if bit_depth <= 32 => convert_samples(read_to_buffers::<f32>(&mut reader, config)?, SampleBuffer::F32),
            _ => {
                return Err(anyhow::anyhow!("Unsupported bit depth for float: {}", spec.bits_per_sample));
            }
        },
        hound::SampleFormat::Int => match spec.bits_per_sample {
            bit_depth if bit_depth <= 16 => convert_samples(read_to_buffers::<i16>(&mut reader, config)?, SampleBuffer::I16),
            bit_depth if bit_depth <= 32 => convert_samples(read_to_buffers::<i32>(&mut reader, config)?, SampleBuffer::I32),
            _ => {
                return Err(anyhow::anyhow!("Unsupported bit depth for int: {}", spec.bits_per_sample));
            }
        },
    };

    let file = File {
        // move buffers to storage and store it's id in file
        channels: buffers
            .into_iter()
            .map(|(ch_ix, buffer)| {
                (
                    ch_ix,
                    Channel {
                        ch_ix,
                        buffer_id: buffer_pool.add_buffer(buffer),
                        channel_id: None,
                    },
                )
            })
            .collect(),
        layout: None, // TODO: first need to extend hound to 'publish' the wavextended channel mask?
        sample_rate: spec.sample_rate,
        bit_depth: spec.bits_per_sample,
        sample_type: spec.sample_format.into(),
    };
    return Ok(file);
}

// TODO: maybe use Vec<(ChIx, Vec<T>)> instead of HashMap<ChIx, Vec<T>>?
fn convert_samples<T>(samples: HashMap<ChIx, T>, converter: impl Fn(T) -> SampleBuffer) -> HashMap<ChIx, SampleBuffer> {
    samples.into_iter().map(|(index, sample)| (index, converter(sample))).collect()
}

// read samples into buffers knowing the sample type S
fn read_to_buffers<S>(
    reader: &mut hound::WavReader<std::io::BufReader<std::fs::File>>,
    config: ReadConfig,
) -> Result<HashMap<ChIx, Buffer<S>>>
where
    S: crate::audio::sample2::Sample + hound::Sample,
{
    let nr_channels = reader.spec().channels as usize;
    let sample_range = config.sample_range.unwrap_or(0..reader.duration() as usize);
    anyhow::ensure!(
        sample_range.end <= reader.duration() as usize,
        "sample range end {} is larger than file duration {}",
        sample_range.end,
        reader.duration()
    );

    // Seek to the start position
    if sample_range.start > 0 {
        reader.seek(sample_range.start as u32)?;
    }

    // Read the desired number of interleaved samples
    let interleaved_samples: Result<Vec<S>, _> = reader.samples::<S>().take(sample_range.len() * nr_channels).collect();
    let interleaved_samples = interleaved_samples?;

    // channel indices we want to deinterleave
    let ch_ixs = config.ch_ixs.unwrap_or((0..nr_channels).collect());

    let channels = deinterleave(&interleaved_samples, nr_channels, &ch_ixs)?;

    // associate spec with buffers
    let spec = reader.spec();
    let buffers = channels
        .into_iter()
        .map(|(ch_ix, samples)| {
            let mut buffer = Buffer::new(spec.sample_rate, spec.bits_per_sample);
            buffer.data = samples;
            (ch_ix, buffer)
        })
        .collect();

    Ok(buffers)
}

// Deinterleave specified channels
// TODO: make version that doesn't allocate new buffers, we might want to reuse the same buffers
// when deinterleaving a large file in batches
fn deinterleave<S>(interleaved_samples: &[S], nr_channels: usize, channel_indices: &[ChIx]) -> Result<HashMap<ChIx, Vec<S>>>
where
    S: Copy,
{
    let nr_samples_per_ch = interleaved_samples.len() / nr_channels;
    ensure!(nr_samples_per_ch > 0, format!("{:?} {:?}", interleaved_samples.len(), nr_channels));

    let nr_channels_to_deinterleave = channel_indices.len();
    ensure!(nr_channels_to_deinterleave > 0);
    ensure!(nr_channels_to_deinterleave <= nr_channels);

    let mut result = HashMap::new();

    // pre-allocate output
    for ch_ix in channel_indices.iter() {
        let ch_ix = *ch_ix;
        ensure!(ch_ix < nr_channels, format!("{:?} {:?}", ch_ix, nr_channels));
        result.insert(ch_ix, Vec::with_capacity(nr_samples_per_ch));
    }

    // deinterleave
    // PERF: looping over each channel and 'striding' through the deinterleaved buffer, probably faster.
    for ch_ix in channel_indices.iter() {
        let ch_ix = *ch_ix;
        ensure!(ch_ix < nr_channels, format!("{:?} {:?}", ch_ix, nr_channels));
        let buffer = &mut result.get_mut(&ch_ix).unwrap();
        for sample_ix in 0..nr_samples_per_ch {
            buffer.push(interleaved_samples[sample_ix * nr_channels + ch_ix]);
        }
    }

    Ok(result)
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

/// Read from a file into an array of deinterleaved channels, convert to float, but not scale to
/// [-1, 1]
pub fn read_wav_file_to_float(path: &str) -> Result<audio::Buffer<f32>> {
    // open wav file with hound
    let mut reader = hound::WavReader::open(path).map_err(|err| anyhow::anyhow!("Failed to open wav file '{}': {}", path, err))?;
    let spec = reader.spec();
    dbg!(spec);

    // let debug_take = Some(100);
    let debug_take = usize::MAX;
    // let debug_take = None;

    // read float or int samples into float interleaved vector
    let interleaved_samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            let iter = reader.samples::<f32>();
            let iter = iter.take(debug_take);
            iter.map(|s| s.unwrap()).collect()
        }
        hound::SampleFormat::Int => {
            ensure!(
                spec.bits_per_sample <= 24,
                "integer PCM with bitdepth {} cannot be represented exactly as float, loss of precision may occur",
                spec.bits_per_sample
            );
            let iter = reader.samples::<i32>();
            let iter = iter.take(debug_take);
            iter.map(|s| s.unwrap() as f32).collect()
        }
    };

    // deinterleaved storage
    let mut buffer = audio::BufferBuilder::new()
        .nr_channels(spec.channels.into())
        // .nr_samples(reader.duration() as usize)
        .nr_samples(interleaved_samples.len() / spec.channels as usize)
        .sample_rate(spec.sample_rate)
        .bit_depth(spec.bits_per_sample.into())
        .sample_type(spec.sample_format.into())
        .build::<f32>()?;

    // deinterleave
    // NOTE: looping over each channel and 'striding' through the deinterleaved buffer, probably faster
    // than loopin over the interleaved buffer and jumping between channels?
    // Maybe it is better to use the iterator of the file and do everyting in one go?
    for (ch_ix, ch) in buffer.channels_mut().enumerate() {
        for (sample_ix, sample) in ch.samples_mut().enumerate() {
            *sample = interleaved_samples[sample_ix * spec.channels as usize + ch_ix];
        }
    }

    Ok(buffer)
}

mod tests {

    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use crate::util;

    #[test]
    fn test_read_wav_file_to_float() {
        #[allow(dead_code)]
        let buf = read_wav_file_to_float("/home/emile/repos/rust/wavalyze/data/sine_16_signed_48000_1.wav").unwrap();
        // println!("{:?}", buf);
    }

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
