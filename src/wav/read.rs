use crate::audio;
use anyhow::{ensure, Result};
use hound;

/// Read from a file into an array of deinterleaved channels, convert to float, but not scale to
/// [-1, 1]
pub fn read_wav_file_to_float(path: &str) -> Result<audio::Buffer<f32>> {
    // open wav file with hound
    let mut reader = hound::WavReader::open(path).map_err(|err| anyhow::anyhow!("Failed to open wav file '{}': {}", path, err))?;
    let spec = reader.spec();
    dbg!(spec);

    let debug_take = Some(100);
    // let debug_take = None;

    // read float or int samples into float interleaved vector
    let interleaved_samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            let iter = reader.samples::<f32>();
            let iter = iter.take(debug_take.unwrap_or(usize::MAX));
            iter.map(|s| s.unwrap()).collect()
        }
        hound::SampleFormat::Int => {
            ensure!(
                spec.bits_per_sample <= 24,
                "integer PCM with bitdepth {} cannot be represented exactly as float, loss of precision may occur",
                spec.bits_per_sample
            );
            let iter = reader.samples::<i32>();
            let iter = iter.take(debug_take.unwrap_or(usize::MAX));
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
    use super::read_wav_file_to_float;
    #[allow(unused_imports)]
    use crate::util;
    #[test]
    fn test_read_wav_file_to_float() {
        #[allow(dead_code)]
        let buf = read_wav_file_to_float("/home/emile/repos/rust/wavalyze/data/sine_16_signed_48000_1.wav").unwrap();
        // println!("{:?}", buf);
    }
}
