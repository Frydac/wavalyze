use wavalyze::audio::buffer2::Buffer;

#[test]
fn test_buffer() {
    let buffer = Buffer::<f32>::new(44100, 32);
    assert_eq!(buffer.sample_rate, 44100);
    assert_eq!(buffer.bit_depth, 32);
    assert_eq!(buffer.data.len(), 0);
}

#[test]
fn test_buffer_with_capacity() {
    let buffer = Buffer::<f32>::with_capacity(44100, 32, 10);
    assert_eq!(buffer.sample_rate, 44100);
    assert_eq!(buffer.bit_depth, 32);
    assert_eq!(buffer.data.len(), 0);
    assert_eq!(buffer.data.capacity(), 10);
}

#[test]
fn test_buffer_index() {
    let mut buffer = Buffer::<f32>::new(44100, 32);
    buffer.data.push(1.0);
    buffer.data.push(2.0);
    assert_eq!(buffer[0], 1.0);
    assert_eq!(buffer[1], 2.0);
}

#[test]
fn test_buffer_index_mut() {
    let mut buffer = Buffer::<f32>::new(44100, 32);
    buffer.data.push(1.0);
    buffer.data.push(2.0);
    assert_eq!(buffer[0], 1.0);
    assert_eq!(buffer[1], 2.0);

    buffer[0] = 3.0;
    buffer[1] = 4.0;
    assert_eq!(buffer[0], 3.0);
    assert_eq!(buffer[1], 4.0);
}

#[test]
fn test_buffer_iter_mut() {
    let mut buffer = Buffer::<f32>::new(44100, 32);
    buffer.data.push(1.0);
    buffer.data.push(2.0);

    for sample in buffer.iter_mut() {
        *sample = 3.0;
    }
    assert_eq!(buffer.data, vec![3.0, 3.0]);
}
