use crate::{
    audio::{
        buffer2::{Buffer, BufferE},
        sample,
        sample2::Sample,
        sample_rect2::SampleRect,
        thumbnail::{LevelData, LevelDataERef},
        SampleRectE,
    },
    model::ruler::{sample_ix_to_screen_x, sample_value_to_screen_y},
    pos,
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
    pub fn new(x: pos::Coord, y_min: pos::Coord, y_max: pos::Coord) -> Self {
        Self {
            min: Pos::new(x, y_min),
            max: Pos::new(x, y_max),
        }
    }
    pub fn include_y(&mut self, y: pos::Coord) {
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

    pub fn shrink_min(&mut self, y: pos::Coord) {
        if self.min.y < y {
            self.min.y = y;
        }
    }
    pub fn shrink_max(&mut self, y: pos::Coord) {
        if self.max.y > y {
            self.max.y = y;
        }
    }

    pub fn is_outside(&self, min_y: pos::Coord, max_y: pos::Coord) -> bool {
        self.min.y >= max_y || self.max.y <= min_y
    }

    /// Specific rule to shrink the min/max y values to fit the screen rect for drawing.
    /// This means, that if the min/max y is completely outside of the screen rect, we leave it, as
    /// we don't neeed to draw anything.
    pub fn shrink_to_fit_if_partial(&mut self, min_y: pos::Coord, max_y: pos::Coord) {
        if !self.is_outside(min_y, max_y) {
            self.shrink_min(min_y);
            self.shrink_max(max_y);
        }
    }

    pub fn make_at_least_one_high(&mut self) {
        if (self.max.y - self.min.y) < 1.0 {
            // self.max.y = self.min.y + 1.0;
            self.min.y = self.max.y - 1.0;
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum ViewData {
    Single(Vec<Pos>),
    MinMax(Vec<MinMaxPos>),
}

impl ViewData {
    pub fn as_min_max_mut(&mut self) -> Option<&mut Vec<MinMaxPos>> {
        match self {
            ViewData::Single(_) => None,
            ViewData::MinMax(ref mut data) => Some(data),
        }
    }
}

/// Represents a view in screen positions of a waveform.
/// Depending on zoom level we have a list of positions to draw as single samples or a linegraph,
/// or we have min/max positions per pixel column.
#[derive(Debug, PartialEq, Clone)]
pub struct View {
    pub data: ViewData,
    pub samples_per_pixel: f32,
    pub sample_ix_start: f64,
    // TODO: store screen_rect/sample_rect in here?
}

impl View {
    pub fn from_buffere(buffere: &BufferE, sample_rect: SampleRectE, screen_rect: Rect) -> Result<Self> {
        match buffere {
            BufferE::F32(buffer) => {
                let sr = sample_rect.get_f32()?;
                View::from_buffer(buffer, *sr, screen_rect)
            }
            BufferE::I32(buffer) => {
                let sr = sample_rect.get_i32()?;
                View::from_buffer(buffer, *sr, screen_rect)
            }
            BufferE::I16(buffer) => {
                let sr = sample_rect.get_i16()?;
                View::from_buffer(buffer, *sr, screen_rect)
            }
        }
    }

    pub fn from_buffer<T: Sample + std::ops::Sub<Output = T>>(
        buffer: &Buffer<T>,
        sample_rect: SampleRect<T>,
        screen_rect: Rect,
    ) -> Result<Self> {
        ensure!(screen_rect.width() > 0.0, "screen_rect emtpy");
        ensure!(sample_rect.width() > 0.0, "sample_rect empty");

        let val_rng = sample_rect.val_rng.ok_or_else(|| anyhow!("val_rng is missing"))?;
        ensure!(!val_rng.is_empty(), "val_rng is empty");

        let samples_per_pixel = sample_rect.width() / screen_rect.width();

        // Get visible range of sample indices present in the buffer
        let start_ix = sample_rect.ix_rng.start.max(0.0).ceil() as usize;
        let end_ix = ((sample_rect.ix_rng.end + 1.0).max(0.0).floor() as usize).min(buffer.len());
        ensure!(end_ix >= start_ix, "end_ix < start_ix");
        let nr_samples = end_ix - start_ix;

        // Get screen position for sample, with floored x coordinate.
        let get_pos = |ix: usize, sample: T| -> Result<Pos> {
            let pos_x = sample_ix_to_screen_x(ix as f64, sample_rect.ix_rng, screen_rect);
            let pos_x = pos_x.floor();
            let pos_y = sample_value_to_screen_y(sample, val_rng, screen_rect).ok_or(anyhow!("sample_value_to_screen_y failed"))?;
            Ok(Pos::new(pos_x, pos_y))
        };

        let view_data = if samples_per_pixel < 2.0 {
            // We have less than 2 samples per pixel, we draw all the samples.
            let mut data = Vec::<Pos>::with_capacity(screen_rect.width() as usize);
            for (ix, sample) in buffer.iter().enumerate().skip(start_ix).take(nr_samples) {
                data.push(get_pos(ix, *sample)?);
            }
            ViewData::Single(data)
        } else {
            // We have 2 or more samples per pixel, we draw the min/max of the samples per
            // pixel column (x coordinate).
            let mut data = Vec::<MinMaxPos>::with_capacity(screen_rect.width() as usize);

            // Convert all samples to min/max positions even if outside of the screen rect.
            {
                let mut cur_min_max_pos;
                // Fill first min/max position
                {
                    let val = *buffer.get(start_ix).ok_or(anyhow!("buffer not in range"))?;
                    let pos = get_pos(start_ix, val)?;
                    cur_min_max_pos = MinMaxPos::from_pos(pos);
                }
                for (ix, sample) in buffer.iter().enumerate().skip(start_ix + 1).take(nr_samples - 1) {
                    let pos = get_pos(ix, *sample)?;
                    if cur_min_max_pos.min.x == pos.x {
                        cur_min_max_pos.include_y(pos.y);
                    } else {
                        cur_min_max_pos.make_at_least_one_high();
                        data.push(cur_min_max_pos);
                        cur_min_max_pos = MinMaxPos::from_pos(pos);
                    }
                }
            }

            // Clip the positions, taking care of extending where necessary to close the gaps.
            clip_view_data(&mut data, screen_rect);

            ViewData::MinMax(data)
        };

        Ok(Self {
            data: view_data,
            samples_per_pixel,
            sample_ix_start: sample_rect.ix_rng.start,
        })
    }
    pub fn from_level_data_e(level_data: &LevelDataERef<'_>, sample_rect: SampleRectE, screen_rect: Rect) -> Result<Self> {
        match level_data {
            LevelDataERef::F32(level_data) => Self::from_level_data(level_data, *sample_rect.get_f32()?, screen_rect),
            LevelDataERef::I32(level_data) => Self::from_level_data(level_data, *sample_rect.get_i32()?, screen_rect),
            LevelDataERef::I16(level_data) => Self::from_level_data(level_data, *sample_rect.get_i16()?, screen_rect),
        }
    }

    pub fn from_level_data<T: Sample + std::ops::Sub<Output = T>>(
        level_data: &LevelData<T>,
        sample_rect: SampleRect<T>,
        screen_rect: Rect,
    ) -> Result<Self> {
        ensure!(screen_rect.width() > 0.0, "screen_rect emtpy");
        ensure!(sample_rect.width() > 0.0, "sample_rect empty");
        let val_rng = sample_rect.val_rng.ok_or_else(|| anyhow!("val_rng is missing"))?;
        ensure!(!val_rng.is_empty(), "val_rng is empty");

        // target zoom level
        let samples_per_pixel = sample_rect.width() / screen_rect.width();
        ensure!(
            samples_per_pixel >= level_data.samples_per_pixel as f32,
            "samples_per_pixel too small"
        );

        // ratio target vs current (of level_data)
        let ratio = samples_per_pixel as f64 / level_data.samples_per_pixel;

        // Get visible range of min/max sample indices present in the level_data
        let start_ix = sample_rect.ix_rng.start.max(0.0).ceil();
        let end_ix = (sample_rect.ix_rng.end + 1.0).max(0.0).floor();
        let start_ix = start_ix / level_data.samples_per_pixel;
        let end_ix = end_ix / level_data.samples_per_pixel;
        let start_ix = start_ix as usize;
        let end_ix = end_ix as usize;
        let end_ix = end_ix.min(level_data.data.len());
        ensure!(end_ix >= start_ix, "end_ix < start_ix");
        let nr_samples = end_ix - start_ix;

        // Get screen positions for min/max sample, with floored x coordinate.
        let get_min_max_pos = |ix_in_level_data: usize, min_max_val: sample::ValRange<T>| -> Result<MinMaxPos> {
            let sample_ix = level_data.ix_to_sample_ix(ix_in_level_data);
            let pos_x = sample_ix_to_screen_x(sample_ix as f64, sample_rect.ix_rng, screen_rect);
            // NOTE: y screen coordinates go from top to bottom, so we need to invert the min/max values
            let pos_y_min =
                sample_value_to_screen_y(min_max_val.max, val_rng, screen_rect).ok_or(anyhow!("sample_value_to_screen_y failed"))?;
            let pos_y_max =
                sample_value_to_screen_y(min_max_val.min, val_rng, screen_rect).ok_or(anyhow!("sample_value_to_screen_y failed"))?;
            Ok(MinMaxPos {
                min: Pos::new(pos_x, pos_y_min),
                max: Pos::new(pos_x, pos_y_max),
            })
        };

        let view_data = {
            let mut data = Vec::<MinMaxPos>::with_capacity(nr_samples / ratio as usize);
            let mut cur_min_max_pos;
            {
                let val = *level_data.data.get(start_ix).ok_or(anyhow!("level_data not in range"))?;
                cur_min_max_pos = get_min_max_pos(start_ix, val)?;
            }
            for (ix_in_level_data, val) in level_data.data.iter().enumerate().skip(start_ix + 1).take(nr_samples - 1) {
                let min_max_pos = get_min_max_pos(ix_in_level_data, *val)?;
                if cur_min_max_pos.min.x == min_max_pos.min.x {
                    cur_min_max_pos.include_y(min_max_pos.max.y);
                } else {
                    cur_min_max_pos.make_at_least_one_high();
                    data.push(cur_min_max_pos);
                    cur_min_max_pos = min_max_pos;
                }
            }
            clip_view_data(&mut data, screen_rect);
            // tracing::trace!("Created ViewData from level_data, samples_per_pixel: {}, data.len(): {}", samples_per_pixel, data.len());
            ViewData::MinMax(data)
        };

        Ok(Self {
            data: view_data,
            samples_per_pixel,
            sample_ix_start: sample_rect.ix_rng.start,
        })
    }
}
pub fn clip_view_data(view_data: &mut [MinMaxPos], screen_rect: Rect) {
    if view_data.is_empty() {
        return;
    }
    {
        let first_element = &mut view_data[0];
        // if partially outside, clip to rect, but leave completely outside alone
        first_element.shrink_to_fit_if_partial(screen_rect.min.y, screen_rect.max.y);
    }
    for vd_ix in 0..view_data.len() - 1 {
        // This is a canonical way to get a sliding window of 2 mutable elements (says chatGPT)
        // TODO: maybe better to construct a new Vec<MinMaxPos>, the parts that are outside of the
        // screen_rect can be discarded. Might be faster?
        let (left, right) = view_data.split_at_mut(vd_ix + 1);
        let a = &mut left[vd_ix];
        let b = &mut right[0];

        // if partially outside, clip to rect, but leave completely outside alone
        b.shrink_to_fit_if_partial(screen_rect.min.y, screen_rect.max.y);

        // now a and b are completely inside or outside the screen rect
        let a_is_outside = a.is_outside(screen_rect.min.y, screen_rect.max.y);
        let b_is_outside = b.is_outside(screen_rect.min.y, screen_rect.max.y);

        if a_is_outside && b_is_outside {
            // if outside and at opposide sides of the screen_rect, we should draw a line
            let a_gt_b_lt = a.min.y > screen_rect.max.y && b.max.y < screen_rect.min.y;
            let a_lt_b_gt = a.max.y < screen_rect.min.y && b.min.y > screen_rect.max.y;
            if a_gt_b_lt || a_lt_b_gt {
                a.min.y = screen_rect.min.y;
                a.max.y = screen_rect.max.y;
            }
        } else if a_is_outside {
            // extend b to the edge
            if a.min.y > screen_rect.max.y {
                b.max.y = screen_rect.max.y;
            }
            if a.max.y < screen_rect.min.y {
                b.min.y = screen_rect.min.y;
            }
        } else if b_is_outside {
            // extend a to the edge
            if b.min.y > screen_rect.max.y {
                a.max.y = screen_rect.max.y;
            }
            if b.max.y < screen_rect.min.y {
                a.min.y = screen_rect.min.y;
            }
        } else {
            // if gap, extend both with half the distance of the gap
            if a.max.y < b.min.y {
                let half = (b.min.y - a.max.y) / 2.0;
                a.max.y += half;
                b.min.y -= half;
            }
            if b.max.y < a.min.y {
                let half = (a.min.y - b.max.y) / 2.0;
                b.max.y += half;
                a.min.y -= half;
            }
        }
    }
}

mod tests {
    use super::*;

    fn rect_001() -> Rect {
        Rect {
            min: Pos::new(10.0, 10.0),
            max: Pos::new(20.0, 20.0),
        }
    }

    #[test]
    fn test_clip_view_data_single_contained() {
        let screen_rect = rect_001();
        let mut view_data = vec![MinMaxPos {
            min: Pos::new(10.0, 13.0),
            max: Pos::new(10.0, 17.0),
        }];
        let exp_view_data = view_data.clone();
        clip_view_data(&mut view_data, screen_rect);
        assert_eq!(view_data, exp_view_data);
    }

    #[test]
    fn test_clip_view_data_single_not_contained_small() {
        let screen_rect = rect_001();
        let mut view_data = vec![MinMaxPos {
            min: Pos::new(11.0, 3.0),
            max: Pos::new(11.0, 7.0),
        }];
        let exp_view_data = view_data.clone();
        clip_view_data(&mut view_data, screen_rect);
        assert_eq!(view_data, exp_view_data);
    }

    #[test]
    fn test_clip_view_data_single_not_contained_large() {
        let screen_rect = rect_001();
        let mut view_data = vec![MinMaxPos {
            min: Pos::new(1.0, 23.0),
            max: Pos::new(1.0, 27.0),
        }];
        let exp_view_data = view_data.clone();
        clip_view_data(&mut view_data, screen_rect);
        assert_eq!(view_data, exp_view_data);
    }

    #[test]
    fn test_clip_view_data_single_half_contained() {
        let screen_rect = rect_001();
        let mut view_data_act = vec![MinMaxPos::new(11.0, 5.0, 15.0)];
        let view_data_exp = vec![MinMaxPos::new(11.0, 10.0, 15.0)];
        clip_view_data(&mut view_data_act, screen_rect);
        assert_eq!(view_data_act, view_data_exp);
    }

    #[test]
    fn test_clip_view_data_2_contained_and_overlapping() {
        let screen_rect = rect_001();
        let mut view_data_act = vec![
            MinMaxPos {
                min: Pos::new(10.0, 10.0),
                max: Pos::new(10.0, 15.0),
            },
            MinMaxPos {
                min: Pos::new(11.0, 12.0),
                max: Pos::new(11.0, 17.0),
            },
        ];
        let view_data_exp = view_data_act.clone();
        clip_view_data(&mut view_data_act, screen_rect);
        assert_eq!(view_data_act, view_data_exp);
    }

    #[test]
    fn test_clip_view_data_2_contained_not_overlapping() {
        let screen_rect = rect_001();
        {
            let mut view_data_act = vec![MinMaxPos::new(10.0, 10.0, 12.0), MinMaxPos::new(11.0, 14.0, 17.0)];
            let view_data_exp = vec![MinMaxPos::new(10.0, 10.0, 13.0), MinMaxPos::new(11.0, 13.0, 17.0)];
            clip_view_data(&mut view_data_act, screen_rect);
            assert_eq!(view_data_act, view_data_exp);
        }
        {
            let mut view_data_act = vec![MinMaxPos::new(10.0, 14.0, 17.0), MinMaxPos::new(11.0, 10.0, 12.0)];
            let view_data_exp = vec![MinMaxPos::new(10.0, 13.0, 17.0), MinMaxPos::new(11.0, 10.0, 13.0)];
            clip_view_data(&mut view_data_act, screen_rect);
            assert_eq!(view_data_act, view_data_exp);
        }
    }

    #[test]
    fn test_clip_view_data_1_contained_1_outside() {
        let screen_rect = rect_001();
        {
            let mut view_data_act = vec![MinMaxPos::new(10.0, 13.0, 17.0), MinMaxPos::new(10.0, 22.0, 25.0)];
            let view_data_exp = vec![MinMaxPos::new(10.0, 13.0, 20.0), MinMaxPos::new(10.0, 22.0, 25.0)];
            clip_view_data(&mut view_data_act, screen_rect);
            assert_eq!(view_data_act, view_data_exp);
        }
    }

    #[test]
    fn test_shrink_to_fit_if_not_outside() {
        let min_max_pos = MinMaxPos {
            min: Pos::new(0.0, 5.0),
            max: Pos::new(0.0, 10.0),
        };

        // outside
        let mut min_max_pos_act = min_max_pos.clone();
        min_max_pos_act.shrink_to_fit_if_partial(0.0, 5.0);
        assert_eq!(min_max_pos_act, min_max_pos);

        // outside
        let mut min_max_pos_act = min_max_pos.clone();
        min_max_pos_act.shrink_to_fit_if_partial(10.0, 15.0);
        assert_eq!(min_max_pos_act, min_max_pos);

        // completely inside
        let mut min_max_pos_act = min_max_pos.clone();
        min_max_pos_act.shrink_to_fit_if_partial(0.0, 15.0);
        assert_eq!(min_max_pos_act, min_max_pos);

        // shrinking min
        let mut min_max_pos_act = min_max_pos.clone();
        min_max_pos_act.shrink_to_fit_if_partial(7.0, 15.0);
        let min_max_pos_exp = MinMaxPos {
            min: Pos::new(0.0, 7.0),
            max: Pos::new(0.0, 10.0),
        };
        assert_eq!(min_max_pos_act, min_max_pos_exp);

        // shrinking max
        let mut min_max_pos_act = min_max_pos.clone();
        min_max_pos_act.shrink_to_fit_if_partial(0.0, 6.0);
        let min_max_pos_exp = MinMaxPos {
            min: Pos::new(0.0, 5.0),
            max: Pos::new(0.0, 6.0),
        };
        assert_eq!(min_max_pos_act, min_max_pos_exp);
    }
}
