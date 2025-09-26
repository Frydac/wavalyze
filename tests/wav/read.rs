use wavalyze::wav::read::{ReadConfig,read_to_file};
use wavalyze::audio::BufferPool;
use wavalyze::audio::SampleBuffer;
use hound;


#[test]
fn test_read_to_file() {
    let config = ReadConfig {
        filepath: String::from("data/sine_16_signed_48000_1.wav"),
        ch_ixs: None,
        sample_range: None,
    };
    let mut buffer_pool = BufferPool::new();
    let file = read_to_file(config, &mut buffer_pool).unwrap();
    assert_eq!(file.sample_rate, 48000);
    assert_eq!(file.bit_depth, 16);
    assert_eq!(file.channels.len(), 1);
    let _buffer_id_i16 = file.channels[0].buffer;
    // let sample_buffer_i16 = file.channels[0].buffer.get_buffer().unwrap();
    // assert!(matches!(sample_buffer_i16, wavalyze::audio::buffer2::SampleBuffer::I16(_)));
}

#[test]
fn test_read_to_file_options() {
    // 1. Create a wav file with hound
    let spec = hound::WavSpec {
        channels: 3,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let test_output_dir = "target/test_output";
    std::fs::create_dir_all(test_output_dir).unwrap();
    let file_path = format!("{}/test_read_options.wav", test_output_dir);
    let mut writer = hound::WavWriter::create(&file_path, spec).unwrap();

    // Write some known interleaved samples
    // 4 frames of 3 channels each
    // Frame 0: 1, 2, 3
    // Frame 1: 4, 5, 6
    // Frame 2: 7, 8, 9
    // Frame 3: 10, 11, 12
    for i in 1..=12 {
        writer.write_sample(i as i16).unwrap();
    }
    writer.finalize().unwrap();

    // 2. Call read_to_file with options
    let config = ReadConfig {
        filepath: file_path,
        ch_ixs: Some(vec![0, 2]), // Read channels 0 and 2
        sample_range: Some(1..3), // Read frames 1 and 2 (exclusive end)
    };
    let mut buffer_pool = BufferPool::new();
    let file = read_to_file(config, &mut buffer_pool).unwrap();

    // 3. Assertions
    assert_eq!(file.sample_rate, 44100);
    assert_eq!(file.bit_depth, 16);
    assert_eq!(file.channels.len(), 2); // We read 2 channels

    // Check channel 0
    let ch0 = file.channels.iter().find(|c| c.ch_ix == 0).unwrap();
    let buffer0_id = ch0.buffer;
    let buffer0 = buffer_pool.get_buffer(buffer0_id).unwrap();
    if let SampleBuffer::I16(buf) = buffer0 {
        // Frames 1 and 2 for channel 0 are samples 4 and 7.
        assert_eq!(buf.data, vec![4, 7]);
    } else {
        panic!("Incorrect buffer type for channel 0");
    }

    // Check channel 2
    let ch2 = file.channels.iter().find(|c| c.ch_ix == 2).unwrap();
    let buffer2_id = ch2.buffer;
    let buffer2 = buffer_pool.get_buffer(buffer2_id).unwrap();
    if let SampleBuffer::I16(buf) = buffer2 {
        // Frames 1 and 2 for channel 2 are samples 6 and 9.
        assert_eq!(buf.data, vec![6, 9]);
    } else {
        panic!("Incorrect buffer type for channel 2");
    }
}