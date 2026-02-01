use super::channel::Channel;
use super::sample::{Sample, SampleType};
use anyhow::{Result, anyhow, ensure};
use std::cell::RefCell;
use std::ops::{Deref, DerefMut, Index, IndexMut};

/**
 * Buffer
 *
 * Represents a block of audio as a collection of non-interleaved channels
 */

#[derive(Debug, Clone)]
pub struct Buffer<T: Sample> {
    pub sample_rate: u32,
    pub bit_depth: u32,
    pub sample_type: SampleType, // NOTE: we might store SampleType::Int as f32, as long as the
    // bit_depth <= 24, this should be ok
    data: Vec<Channel<T>>,
}

impl<T: Sample> Buffer<T> {
    pub fn nr_channels(&self) -> usize {
        self.data.len()
    }
    pub fn channels(&self) -> impl Iterator<Item = &Channel<T>> {
        self.data.iter()
    }
    pub fn channels_mut(&mut self) -> impl Iterator<Item = &mut Channel<T>> {
        self.data.iter_mut()
    }
    pub fn nr_samples(&self) -> usize {
        self.data[0].len()
    }
}

// operator[]
impl<T: Sample> Index<usize> for Buffer<T> {
    type Output = Channel<T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}
impl<T: Sample> IndexMut<usize> for Buffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

// operator* and .
impl<T: Sample> Deref for Buffer<T> {
    type Target = Vec<Channel<T>>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl<T: Sample> DerefMut for Buffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

use std::fmt;
use std::rc::Rc;

impl<T: Sample + std::fmt::Debug + std::fmt::Display> fmt::Display for Buffer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Buffer:")?;
        writeln!(f, "  Sample Rate: {}", self.sample_rate)?;
        writeln!(f, "  Sample Type: {:?}", self.sample_type)?;
        writeln!(f, "  Bit Depth: {}", self.bit_depth)?;

        writeln!(f, "  Channels ({}):", self.data.len())?;
        for (i, channel) in self.data.iter().enumerate() {
            // This passes the format on, but then there is no indentation..
            // channel.fmt(f)?;

            let precision = f.precision();
            let width = f.width();

            match (width, precision) {
                (Some(width), Some(precision)) => {
                    writeln!(
                        f,
                        "    {}: {:width$.precision$}",
                        i,
                        channel,
                        width = width,
                        precision = precision
                    )?;
                }
                (Some(width), None) => {
                    writeln!(f, "    {}: {:width$}", i, channel, width = width)?;
                }
                (None, Some(precision)) => {
                    writeln!(
                        f,
                        "    {}: {:.precision$}",
                        i,
                        channel,
                        precision = precision
                    )?;
                }
                (None, None) => {
                    writeln!(f, "    {}: {}", i, channel)?;
                }
            }
        }

        Ok(())
    }
}

// TODO: add Deref trait
// TODO: add IntoIterator trait (remove channels/channels_mut?)
// TODO: add From/Into trait

/**
 * BufferBuilder
 *
 * NOTE: needs to be in the same file so we can access the private 'data' field
 */

#[derive(Debug, Clone)]
pub struct BufferBuilder {
    sample_rate: Option<u32>,
    bit_depth: Option<u32>,
    nr_channels: Option<usize>,
    nr_samples: Option<usize>, // nr samples per channel
    sample_type: Option<SampleType>,
}

impl BufferBuilder {
    pub fn new() -> Self {
        Self {
            sample_rate: None,
            bit_depth: None,
            nr_channels: None,
            nr_samples: None,
            sample_type: None,
        }
    }
    pub fn nr_channels(mut self, nr_channels: usize) -> Self {
        self.nr_channels = Some(nr_channels);
        self
    }
    pub fn nr_samples(mut self, block_size: usize) -> Self {
        self.nr_samples = Some(block_size);
        self
    }
    pub fn sample_rate(mut self, sample_rate: u32) -> Self {
        self.sample_rate = Some(sample_rate);
        self
    }
    // Bit depth should be set to 32 or None for f32, if None, the builder will set it to 32
    pub fn bit_depth(mut self, bit_depth: u32) -> Self {
        self.bit_depth = Some(bit_depth);
        self
    }
    // optional, if not provided will be derived from T
    pub fn sample_type(mut self, sample_type: SampleType) -> Self {
        self.sample_type = Some(sample_type);
        self
    }

    pub fn build<T: Sample>(self) -> Result<Buffer<T>> {
        let nr_channels = self
            .nr_channels
            .ok_or_else(|| anyhow!("Number of channels not set"))?;
        let nr_samples = self
            .nr_samples
            .ok_or_else(|| anyhow!("Number of samples not set"))?;
        let mut bit_depth = self.bit_depth;
        let sample_rate = self
            .sample_rate
            .ok_or_else(|| anyhow!("Sample rate not set"))?;

        // Derive sample_type from T if not provided
        let sample_type: SampleType = self.sample_type.unwrap_or_else(|| {
            if std::any::type_name::<T>() == "f32" {
                SampleType::Float
            } else {
                SampleType::Int
            }
        });

        // Some restrictions/invariants
        if sample_type == SampleType::Float {
            if self.bit_depth.is_none() {
                bit_depth = Some(32);
            }
            ensure!(
                bit_depth == Some(32),
                "Bit depth must be 32 for float audio, but is {:?}",
                bit_depth
            );
        } else if sample_type == SampleType::Int {
            let nr_bits_storage_type = std::mem::size_of::<T>() * 8;
            ensure!(
                bit_depth.is_some(),
                "Bit depth must be set for integer audio"
            );
            ensure!(
                nr_bits_storage_type >= bit_depth.unwrap() as usize,
                "Bit depth too large for storage type. Bit depth: {:?}, storage type: {}",
                bit_depth,
                std::any::type_name::<T>()
            );
        }

        // preallocate memory
        let data = vec![Channel::<T>::new(nr_samples); nr_channels];

        Ok(Buffer {
            data,
            sample_rate,
            bit_depth: bit_depth.unwrap(),
            sample_type,
        })
    }
    pub fn build_shared<T: Sample>(self) -> Result<Rc<RefCell<Buffer<T>>>> {
        // Use the existing build function and wrap its result
        self.build().map(|buffer| Rc::new(RefCell::new(buffer)))
    }
}

impl Default for BufferBuilder {
    fn default() -> Self {
        Self::new()
    }
}

mod tests {
    #[test]
    fn test_buffer_builder_and_index() {
        let mut ab = super::BufferBuilder::new()
            .nr_channels(2)
            .nr_samples(10)
            .sample_rate(44100)
            .bit_depth(32)
            .sample_type(super::SampleType::Float)
            .build::<f32>()
            .unwrap();
        // dbg!(ab);

        // ab[0][1] = 1.0;
        // dbg!(&ab);

        ab[1][1] = 1.0;
        println!("5.2 ab: {:5.2}", ab);
        println!("5.  ab: {:5.}", ab);
        println!(" .2 ab: {:.2}", ab);
        println!("    ab: {}", ab);
    }
}
