use wavalyze::audio::buffer2::BufferE;
use wavalyze::audio::manager::Buffers;
use wavalyze::wav::read::{read_to_file, ReadConfig};

// #[test]
// fn test_read_to_file() {
//     let config = ReadConfig {
//         filepath: String::from("data/sine_16_signed_48000_1.wav"),
//         ch_ixs: None,
//         sample_range: None,
//     };
//     let mut buffer_pool = BufferPool::new();
//     let file = read_to_file(config, &mut buffer_pool).unwrap();
//     assert_eq!(file.sample_rate, 48000);
//     assert_eq!(file.bit_depth, 16);
//     assert_eq!(file.channels.len(), 1);
//     let _buffer_id_i16 = file.channels[0].buffer;
// }

fn setup_test_wav_file<S: hound::Sample + Copy>(spec: hound::WavSpec, samples: &[S], test_name: &str) -> String {
    let test_output_dir = "target/test_output";
    std::fs::create_dir_all(test_output_dir).unwrap();
    let file_path = format!(
        "{}/{}_{}_{}.wav",
        test_output_dir,
        test_name,
        spec.bits_per_sample,
        match spec.sample_format {
            hound::SampleFormat::Int => "int",
            hound::SampleFormat::Float => "float",
        }
    );

    let mut writer = hound::WavWriter::create(&file_path, spec).unwrap();
    for &sample in samples {
        writer.write_sample(sample).unwrap();
    }
    writer.finalize().unwrap();
    file_path
}

#[test]
fn test_read_options_i16() {
    let spec = hound::WavSpec {
        channels: 3,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let samples: Vec<i16> = (1..=12).collect();
    let file_path = setup_test_wav_file(spec, &samples, "i16");

    let config = ReadConfig {
        filepath: file_path,
        ch_ixs: Some(vec![0, 2]),
        sample_range: Some(1..3),
    };
    let mut buffers = Buffers::default();
    let file = read_to_file(config, &mut buffers).unwrap();

    assert_eq!(file.sample_rate, spec.sample_rate);
    assert_eq!(file.bit_depth, spec.bits_per_sample);
    assert_eq!(file.channels.len(), 2);

    let ch0 = file.channels.get(&0).unwrap();
    if let BufferE::I16(buf) = buffers.get(ch0.buffer_id).unwrap() {
        assert_eq!(buf.data, &[4, 7]);
        assert_eq!(buf.sample_rate, spec.sample_rate);
        assert_eq!(buf.bit_depth, spec.bits_per_sample);
    } else {
        panic!("Incorrect buffer type");
    }

    let ch2 = file.channels.get(&2).unwrap();
    if let BufferE::I16(buf) = buffers.get(ch2.buffer_id).unwrap() {
        assert_eq!(buf.data, &[6, 9]);
        assert_eq!(buf.sample_rate, spec.sample_rate);
        assert_eq!(buf.bit_depth, spec.bits_per_sample);
    } else {
        panic!("Incorrect buffer type");
    }

    assert_eq!(file.sample_rate, spec.sample_rate);
    assert_eq!(file.bit_depth, spec.bits_per_sample);
    assert_eq!(file.channels.len(), 2);
}

#[test]
fn test_read_options_i24() {
    let spec = hound::WavSpec {
        channels: 3,
        sample_rate: 44100,
        bits_per_sample: 24,
        sample_format: hound::SampleFormat::Int,
    };
    let samples: Vec<i32> = (1..=12).map(|x| x * 1000).collect();
    let file_path = setup_test_wav_file(spec, &samples, "i24");

    let config = ReadConfig {
        filepath: file_path,
        ch_ixs: Some(vec![0, 2]),
        sample_range: Some(1..3),
    };
    let mut buffers = Buffers::default();
    let file = read_to_file(config, &mut buffers).unwrap();

    let ch0 = file.channels.get(&0).unwrap();
    if let BufferE::I32(buf) = buffers.get(ch0.buffer_id).unwrap() {
        assert_eq!(buf.data, &[4000, 7000]);
    } else {
        panic!("Incorrect buffer type");
    }
}

#[test]
fn test_read_options_i32() {
    let spec = hound::WavSpec {
        channels: 3,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Int,
    };
    let samples: Vec<i32> = (1..=12).map(|x| x * 100000).collect();
    let file_path = setup_test_wav_file(spec, &samples, "i32");

    let config = ReadConfig {
        filepath: file_path,
        ch_ixs: Some(vec![0, 2]),
        sample_range: Some(1..3),
    };
    let mut buffers = Buffers::default();
    let file = read_to_file(config, &mut buffers).unwrap();

    let ch0 = file.channels.get(&0).unwrap();
    if let BufferE::I32(buf) = buffers.get(ch0.buffer_id).unwrap() {
        assert_eq!(buf.data, &[400000, 700000]);
    } else {
        panic!("Incorrect buffer type");
    }
}

#[test]
fn test_read_options_f32() {
    let spec = hound::WavSpec {
        channels: 3,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let samples: Vec<f32> = (1..=12).map(|x| x as f32 * 0.1).collect();
    let file_path = setup_test_wav_file(spec, &samples, "f32");

    let config = ReadConfig {
        filepath: file_path,
        ch_ixs: Some(vec![0, 2]),
        sample_range: Some(1..3),
    };
    let mut buffers = Buffers::default();
    let file = read_to_file(config, &mut buffers).unwrap();

    let ch0 = file.channels.get(&0).unwrap();
    if let BufferE::F32(buf) = buffers.get(ch0.buffer_id).unwrap() {
        assert_eq!(buf.data, &[0.4, 0.7]);
    } else {
        panic!("Incorrect buffer type");
    }
}
