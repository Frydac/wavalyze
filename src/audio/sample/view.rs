use crate::{
    audio::{buffer2::Buffer, sample2::Sample, sample_rect2::SampleRect},
    model::{
        ruler::{sample_ix_to_screen_x, sample_value_to_screen_y},
        BitDepth, SampleRate,
    },
    rect::Rect,
    Pos,
};
use anyhow::{anyhow, ensure, Result};

/// Represents a pixel column defined by 2 positions with the same x coordinate.
#[derive(Debug, PartialEq, Clone)]
pub struct MinMaxPos {
    pub min: Pos,
    pub max: Pos,
}

impl MinMaxPos {
    pub fn include_y(&mut self, y: f32) {
        if self.min.y > y {
            self.min.y = y;
        }
        if self.max.y < y {
            self.max.y = y;
        }
    }
    pub fn from_pos(pos: Pos) -> Self {
        Self { min: pos, max: pos }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ViewData {
    Single(Vec<Pos>),
    MinMax(Vec<MinMaxPos>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct View {
    pub data: ViewData,
    pub sample_rate: SampleRate,
    pub bit_depth: BitDepth,
    pub samples_per_pixel: f32,
    pub sample_ix_start: f64,
}

impl View {
    pub fn from_buffer<T: Sample + std::ops::Sub<Output = T>>(buffer: &Buffer<T>, sample_rect: SampleRect<T>, screen_rect: Rect) -> Result<Self> {
        ensure!(screen_rect.width() > 0.0, "screen_rect width is 0");
        ensure!(sample_rect.ix_rng.len() > 0.0, "sample_rect ix_rng is empty");

        let val_rng = sample_rect.val_rng.ok_or_else(|| anyhow!("val_rng is missing"))?;
        ensure!(val_rng.len() > T::default(), "val_rng is empty");

        let samples_per_pixel = screen_rect.width() / sample_rect.width();

        // Get visible range of existing samples
        let start_ix = sample_rect.ix_rng.start.max(0.0).ceil() as usize;
        let end_ix = (sample_rect.ix_rng.end.max(0.0).floor() as usize).min(buffer.len());
        let nr_samples = end_ix - start_ix;

        // Get screen position for sample, with floored x coordinate.
        let get_pos = |ix: usize, sample: T| -> Result<Pos> {
            let pos_x = sample_ix_to_screen_x(ix as f64, sample_rect.ix_rng, screen_rect);
            let pos_x = pos_x.floor();
            let pos_y = sample_value_to_screen_y(sample, val_rng, screen_rect).ok_or(anyhow!("sample_value_to_screen_y failed"))?;
            Ok(Pos::new(pos_x, pos_y))
        };

        let view_data = match samples_per_pixel {
            spp if spp < 2.0 => {
                // We have less than 2 samples per pixel, we show all the samples.
                let mut data = Vec::<Pos>::with_capacity(nr_samples / samples_per_pixel as usize);
                for (ix, sample) in buffer.iter().enumerate().skip(start_ix).take(end_ix - start_ix) {
                    data.push(get_pos(ix, *sample)?);
                }
                ViewData::Single(data)
            }
            _ => {
                // We have 2 or more samples per pixel, we show the min/max of the samples per
                // pixel column (x coordinate).
                let mut data = Vec::<MinMaxPos>::with_capacity(nr_samples / samples_per_pixel as usize);
                let mut cur_min_max_pos;
                {
                    let val = *buffer.get(start_ix).ok_or(anyhow!("buffer not in range"))?;
                    let pos = get_pos(start_ix, val)?;
                    cur_min_max_pos = MinMaxPos::from_pos(pos);
                }
                for (ix, sample) in buffer.iter().enumerate().skip(start_ix).take(end_ix - start_ix) {
                    let pos = get_pos(ix, *sample)?;
                    if cur_min_max_pos.min.x == pos.x {
                        cur_min_max_pos.include_y(pos.y);
                    } else {
                        data.push(cur_min_max_pos);
                        cur_min_max_pos = MinMaxPos::from_pos(pos);
                    }
                }
                ViewData::MinMax(data)
            }
        };

        Ok(Self {
            data: view_data,
            sample_rate: buffer.sample_rate,
            bit_depth: buffer.bit_depth,
            samples_per_pixel,
            sample_ix_start: sample_rect.ix_rng.start,
        })
    }
}
