// TODO: I think this can be removed, most functionalyt moved to audio


#[derive(Debug, Default, Clone)]
#[allow(dead_code)]
pub struct SampleRange {
    pub start: usize,
    pub size: usize,
}


// Ideas: 
//   * Maybe better named AudioBuffer ? (borrowed from JUCE)
//   * to provide an iterator over the channels, and then an iterator over the samples, we probably
//     need to create a Channel type maybe that implements Iterator over samples
//     * or maybe a vec of vec's suffices?
//   * provide conversion?
//   * minmax function over range of samples to the the minimum and maximum value of a range
//
// Rerpesents an audio buffer with de-interleaved samples
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct Samples<T> {
    pub data: Vec<Vec<T>>,
    // TODO: maybe we want our own type here
    // Yes definitely, just flat fields better I think
    pub spec: WavSpec,

    pub sample_rate: u32,
    pub bit_depth: u16,
    pub sample_type: SampleType
}

impl Samples<f32> {
    pub fn new(nr_channels: usize, len: usize) -> Self {
        Self {
            data: vec![vec![0.0; len]; nr_channels],
            spec: WavSpec::default(),
            sample_rate: 0,
            bit_depth: 0,
            sample_type: SampleType::Float
        }
    }
    pub fn nr_channels(&self) -> usize {
        self.data.len()
    }
    
    pub fn channels(&self) -> impl Iterator<Item = &Vec<f32>> {
        self.data.iter()
    }

    pub fn channels_mut(&mut self) -> impl Iterator<Item = &mut Vec<f32>> {
        self.data.iter_mut()
    }
}

// use std::fmt;

// impl fmt::Debug for Samples<f32> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let nr_ch = self.data.len();
//         write!(f, "data_nr_ch: {}", nr_ch);
//         write!(f, "{:?}", self.spec)
//         // fmt::Debug::fmt(&self.data, f)
//     }
// }

// Represents an audio buffer with interleaved samples
#[allow(dead_code)]
pub struct SamplesInterleaved<T> {
    pub data: Vec<T>,
    pub spec: WavSpec,
}

pub struct FileSamplesInterleaved<T> {
    pub filename: String,
    // total nr samples in the file interleaved
    pub file_nr_samples: usize,
    // The actual samples read from the file, could be not all the samples
    pub samples: SamplesInterleaved<T>,
    // The absolute range of samples represented by the samples member
    pub sample_range: SampleRange,
}

#[derive(Debug, Clone, Default)]
pub struct FileSamples<T> {
    pub filename: String,
    // total nr samples in the file, per channel.
    pub file_nr_samples: usize,
    // The actual samples read from the file, could be not all the samples
    pub samples: Samples<T>,
    // The absolute range of samples represented by the samples member
    pub file_range: SampleRange,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct WavSpec {
    pub nr_channels: u16,
    pub sample_rate: u32,
    pub bit_depth: u16,
    pub sample_type: SampleType
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[allow(dead_code)]
pub enum SampleType {
    #[default] Float,
    Int
}
