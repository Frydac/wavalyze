use crate::{
    audio::buffer::{Buffer, BufferBuilder},
    sample::SampleType,
};


// Create a 'test' audio::Buffer with a sine wave per channel
pub fn buffer_sine_float(nr_channels: usize, nr_samples: usize, amplitude: f32, sample_rate: u32) -> Buffer<f32> {
    let mut ab = BufferBuilder::new()
        .nr_channels(nr_channels)
        .sample_rate(sample_rate)
        .bit_depth(32)
        .nr_samples(nr_samples)
        .build::<f32>()
        .unwrap();

    use crate::*;

    for (index, channel) in ab.channels_mut().enumerate() {
        let sample_period = 20 + index * 5;
        let mut sine_gen = generator::Sine::new_with_sample_period(sample_period, amplitude, sample_rate);

        for sample in channel.iter_mut() {
            *sample = sine_gen.next().unwrap();
        }
    }

    ab
}

// Trait we will implement for i16, i32 and f32
// I tried different approaches, more C++ like, but they didn't work as a generic type T cannot
// satisfy both From<i32> and From<f32>, as it isn't implemented for f32.
// This adds functionality to existing types, let's see how this goes
// NOTE: what if we define a trait that has a function that returns a 1 in whatever type T is, then
// we could implement it for i16, i32 and f32 wihtout the from?
pub trait AudioSample: Copy {
    fn min_max(sample_type: SampleType, bit_depth: u32) -> (Self, Self);
}

pub trait IntegerAudioSample: Copy {
    fn from_i32(value: i32) -> Self;
}

impl<T: IntegerAudioSample> AudioSample for T {
    fn min_max(sample_type: SampleType, bit_depth: u32) -> (Self, Self) {
        match sample_type {
            SampleType::Int => {
                let max = 1i32 << (bit_depth - 1);
                let min = -max;
                (T::from_i32(min), T::from_i32(max - 1))
            }
            SampleType::Float => panic!("Integer type cannot represent float samples"),
        }
    }
}

// Implement for specific integer types
impl IntegerAudioSample for i16 {
    fn from_i32(value: i32) -> Self {
        value as i16
    }
}

impl IntegerAudioSample for i32 {
    fn from_i32(value: i32) -> Self {
        value
    }
}
impl AudioSample for f32 {
    fn min_max(_sample_type: SampleType, _bit_depth: u32) -> (Self, Self) {
        (-1.0, 1.0)
    }
}

pub trait One {
    fn one() -> Self;
}

impl One for f32 {
    fn one() -> Self {
        1.0
    }
}

impl One for i32 {
    fn one() -> Self {
        1
    }
}

impl One for i16 {
    fn one() -> Self {
        1
    }
}

// So this approach doesn't seem to work:
// we need literals that we convert to T, for which we need From<i32> and From<f32>
// but f32 doesnt satisfy From<i32>
// pub fn min_max<T>(sample_type: SampleType, bit_depth: u32) -> (T, T)
// where
//     T: Copy,
//     T: One + Shl<u32, Output = T> + Neg<Output = T> + Sub<Output = T> + Copy,
// {
//     match sample_type {
//         SampleType::Int => min_max_int::<T>(bit_depth),
//         SampleType::Float => min_max_float(),
//     }
// }

// pub fn min_max_float<T>() -> (T, T)
// where
//
// {
//     (T::from(-1.0), T::from(1.0))
// }

// pub fn min_max_int<T>(bit_depth: u32) -> (T, T)
// where
//     T: From<i32> + Shl<u32, Output = T> + Neg<Output = T> + Sub<Output = T> + Copy,
// {
//     let max = T::from(1) << (bit_depth - 1);
//     let min = -max - T::from(1);
//     (min, max)
// }

mod tests {
    #[cfg(test)]
    use super::*;

    #[test]
    fn test_min_max_i16() {
        let (min, max) = i16::min_max(SampleType::Int, 16);
        assert_eq!(min, -32768);
        assert_eq!(max, 32767);
    }

    #[test]
    fn test_min_max_i32() {
        let (min, max) = i32::min_max(SampleType::Int, 24);
        assert_eq!(min, -8388608);
        assert_eq!(max, 8388607);
    }

    #[test]
    fn test_min_max_f32() {
        let (min, max) = f32::min_max(SampleType::Float, 32);
        assert_eq!(min, -1.0);
        assert_eq!(max, 1.0);
    }

    #[test]
    fn test_buffer_sine_float() {
        let ab = buffer_sine_float(3, 40, 0.8, 48000);
        dbg!(ab);
    }
}
