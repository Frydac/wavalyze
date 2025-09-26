use std::path::PathBuf;

use slotmap::{new_key_type, SlotMap};
use crate::audio::buffer2::Buffer;

// TODO: move somewhere, also name? ok I guess
pub enum SampleBuffer {
    F32(Buffer<f32>),
    I32(Buffer<i32>),
    I16(Buffer<i16>),
}

// Convenience methods for accessing the buffer when you know the type of the buffer
// Doesn't really sove anything..
impl SampleBuffer {
    pub fn as_f32(&self) -> anyhow::Result<&Buffer<f32>> {
        if let SampleBuffer::F32(buf) = self {
            Ok(buf)
        } else {
            anyhow::bail!("Expected F32 buffer")
        }
    }

    // Similarly, for other variants:
    pub fn as_i32(&self) -> anyhow::Result<&Buffer<i32>> {
        if let SampleBuffer::I32(buf) = self {
            Ok(buf)
        } else {
            anyhow::bail!("Expected I32 buffer")
        }
    }

    pub fn as_i16(&self) -> anyhow::Result<&Buffer<i16>> {
        if let SampleBuffer::I16(buf) = self {
            Ok(buf)
        } else {
            anyhow::bail!("Expected I16 buffer")
        }
    }
}

new_key_type! { pub struct BufferId; }

// BufferManager?
#[derive(Default)]
pub struct BufferPool {
    buffers: SlotMap<BufferId, SampleBuffer>,
}

impl BufferPool {
    pub fn new() -> Self {
        Self {
            buffers: SlotMap::<BufferId, SampleBuffer>::with_key(),
        }
    }
    pub fn get_buffer_mut(&mut self, key: BufferId) -> Option<&mut SampleBuffer> {
        self.buffers.get_mut(key)
    }
    pub fn get_buffer(&self, key: BufferId) -> Option<&SampleBuffer> {
        self.buffers.get(key)
    }
    pub fn add_buffer(&mut self, buffer: SampleBuffer) -> BufferId {
        self.buffers.insert(buffer)
    }
}

struct AudioFile {
    path: PathBuf,
    sample_rate: u32,
    bit_depth: u8,
    channels: usize,
    nr_samples: usize,
    buffers: Vec<BufferId>,
}

enum ChannelId {
    Left = 0,
    Right = 1,
    Center = 2,
    Lfe = 3,
    LeftSurround = 4,
    RightSurround = 5,
    CenterSurround = 6,
    LeftBack = 7,
    RightBack = 8,
    HeightLeft = 9,
    HeightRight = 10,
    HeightCenter = 11,
    Top = 12,
    HeightLeftSurround = 13,
    HeightRightSurround = 14,
    HeightCenterSurround = 15,
    HeightLeftBack = 16,
    HeightRightBack = 17,
    LeftCenter = 18,
    RightCenter = 19,
    Lfe2 = 20,
    BottomLeft = 21,
    BottomRight = 22,
    BottomCenter = 23,
    BottomLeftSurround = 24,
    BottomRightSurround = 25,
    LeftWide = 26,
    RightWide = 27,
    TopLeft = 28,
    TopRight = 29,
    Mono = 30,
}
