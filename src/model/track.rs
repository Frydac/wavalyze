use std::cell::RefCell;
use std::rc::Rc;

use crate::audio;
use crate::model;
use crate::pos;
use crate::rect;
use crate::util;
use anyhow::ensure;
use anyhow::Result;
use itertools::Itertools;

use audio::SampleIx;

// TODO: I think I need
// pub type SharedBuffer = Rc<RefCell<Buffer<f32>>>;
// for the track, see anthropic https://claude.ai/chat/bf8ee596-80c3-4c69-a720-275319476438
//
// * we want a constant zoom level, independent of the window size, at least in the horizontal
//   direction (x).
//   * probably express it in samples per pixel
//     * when less than 1 -> draw lines and dots for samples (like in Audacity)
//     * when [1, 4) samples per pixel -> draw lines for all samples
//     * when [1x4, 2x4] samples per pixel ->
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Track {
    id: u64,

    // Contains all the 'original' audio samples
    pub buffer: Rc<RefCell<audio::Buffer<f32>>>,
    // channel ix in the buffer associated with this track
    channel_ix: usize,

    // TODO: we want some kind of metadata struct
    pub name: String,

    /// The rectangle of samples indices that are currently visible,
    /// indices into self.buffer.
    pub sample_rect: audio::SampleRect,

    // x range is pixel width starting at 0.0
    // y range is sample_rect sample range coordinates I think
    // NOTE: not really needed, we can just use screen_rect and 'normalize' it, they should be very similar
    pub view_rect: rect::Rect,

    /// The pixel rectangle with absolute screen coordinates that should display self.sample_rect of
    /// samples
    pub screen_rect: rect::Rect,

    /// Zoom level in x direction
    ///
    /// * Doesn't change when updating the screen_rect to keep the zoom stable
    pub samples_per_pixel: Option<f32>,

    /// Contains all the samples as pixel positions relative to top_left (0,0), currently to be
    /// rendered by the track::View
    /// The final transformation to absolute screen coordinates is done in the view::Track
    view_buffer: model::ViewBuffer,

    hover_info: Option<HoverInfo>,

    hover_info2: Option<HoverInfo2>,
    // when there are more pixels than samples, we want to offset the sample rect
    // this is the distance in pixels from pixel 0 of the view rect to the pixel where the first
    // sample should be drawn
    // pixel_offset: f32,
}

#[derive(Debug, Clone)]
pub struct HoverInfo {
    /// absolute (mouse) screen position in pixel coordinates
    pub screen_pos: pos::Pos,
    // TODO: store time range and sample buffer
    pub samples: Vec<(i32, f32)>, // sample ixs and values for the samples that are rendedred on
    pub sample_type: audio::sample::SampleType,
    pub bit_depth: u32,
    // the given screen_pos, or is closest to the screen_pos
    pub contains_pointer: bool,
    // Key value pairs of what is displayed
    pub list_data: Vec<(String, String)>,
}

impl HoverInfo {
    pub fn new(
        screen_pos: pos::Pos,
        contains_pointer: bool,
        samples: Vec<(i32, f32)>,
        sample_type: audio::sample::SampleType,
        bit_depth: u32,
    ) -> HoverInfo {
        HoverInfo {
            screen_pos,
            samples,
            contains_pointer,
            list_data: vec![],
            sample_type,
            bit_depth,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HoverInfo2 {
    pub sample_ix: SampleIx,
    // When mouse is over this track, we also have an y coordinate
    pub view_buffer_y: Option<f32>,
}

impl Track {
    // TODO: find better way to name/use/abstract this 'zoom' functionality

    // Assumes zero based in and output ranges
    // good for size/width or normalized values
    pub fn sample_ix_to_view_x(&self, sample_ix: SampleIx) -> f32 {
        let samples_per_pixel = self.samples_per_pixel.expect("zoom level not set");
        sample_ix as f32 / samples_per_pixel
    }
    pub fn view_x_to_sample_ix(&self, view_x: f32) -> SampleIx {
        let samples_per_pixel = self.samples_per_pixel.expect("zoom level not set");
        (view_x * samples_per_pixel) as SampleIx
    }

    pub fn sample_ix_to_screen_x(&self, sample_ix: SampleIx) -> Option<f32> {
        if !self.sample_rect.ix_rng.contains(sample_ix) {
            return None;
        }
        self.samples_per_pixel?;
        let sample_ix_offset = sample_ix - self.sample_rect.ix_rng.start();
        let view_x_offset = self.sample_ix_to_view_x(sample_ix_offset);
        let screen_x = view_x_offset + self.screen_rect.min.x;
        Some(screen_x)
    }

    pub fn screen_x_to_sample_ix(&self, screen_x: f32) -> Option<SampleIx> {
        // if !self.screen_rect.contains_x(screen_x) {
        // return None;
        // }
        let view_x_offset = screen_x - self.screen_rect.min.x;
        None
    }

    // TODO: make convert with actual sample index (with current sample rect view) and screen coordinates
    // probaly need an sample offset too, as we want to be able to move tracks around relatively to
    // each other
}

impl Track {
    pub fn new(buffer: Rc<RefCell<audio::Buffer<f32>>>, channel_ix: usize, name: &str) -> Result<Self> {
        ensure!(
            channel_ix < buffer.borrow().nr_channels(),
            "channel out of range, nr_channels: {}, channel ix: {}",
            buffer.borrow().nr_channels(),
            channel_ix
        );
        let mut track = Self {
            id: util::unique_id(),
            buffer,
            channel_ix,
            name: name.to_string(),
            view_rect: rect::Rect::new(0.0, 1.5, 0.0, -1.5),
            view_buffer: model::ViewBuffer::SingleSamples(vec![]),

            screen_rect: rect::Rect::default(),
            samples_per_pixel: None,
            // samples_per_pixel: 1.5,
            hover_info: None,
            hover_info2: None,

            sample_rect: audio::SampleRect::default(),
            // pixel_offset: 0.0,
        };

        // We don't have the screen_rect yet from the GUI, so we can't yet calculate the
        // view_buffer yet
        track.sample_rect = audio::SampleRect::from_buffer(&track.buffer.borrow());
        // dbg!(track.sample_rect);

        Ok(track)
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn hover(&mut self, sample_ix: SampleIx, view_buffer_y: Option<f32>) -> Result<()> {
        self.hover_info2 = Some(HoverInfo2 { sample_ix, view_buffer_y });
        Ok(())
    }
    pub fn unhover(&mut self) {
        self.hover_info = None;
    }

    // Assumes view_rect is correctly set, maybe just pass to function?
    // @param screen_pos Absolute mouse position within the screen_rect
    // TODO: review/redo, not accurate, probably needs more data in some way
    pub fn update_hover_info(&mut self, screen_pos: pos::Pos) {
        if !self.screen_rect.contains_x(screen_pos.x) {
            return;
        }

        let Some(samples_per_pixel) = self.samples_per_pixel else {
            return;
        };
        // relative position
        let pixel_x0 = (screen_pos.x - self.screen_rect.min.x) as i32;
        // sample(s) near/on the pixel
        // dbg!(pixel_x0);
        // dbg!(samples_per_pixel);
        let mut sample_ix_range = sample_x_range(pixel_x0, samples_per_pixel);
        // dbg!(&sample_ix_range);
        // dbg!(self.sample_rect);
        sample_ix_range.start += self.sample_rect.ix_rng.start();
        sample_ix_range.end += self.sample_rect.ix_rng.start();
        // dbg!(&sample_ix_range);

        let mut hover_info = HoverInfo {
            screen_pos,
            samples: vec![],
            contains_pointer: self.screen_rect.contains(screen_pos),
            list_data: vec![],
            sample_type: self.buffer.borrow().sample_type,
            bit_depth: self.buffer.borrow().bit_depth,
        };
        let buffer = self.buffer.borrow();
        let channel = &buffer[self.channel_ix];
        let start = sample_ix_range.start.ceil() as usize;
        let end = sample_ix_range.end.ceil() as usize;
        for sample_ix in start..end {
            let sample_val = channel.at(sample_ix.try_into().unwrap()).unwrap_or(&0.0);
            hover_info.samples.push((sample_ix.try_into().unwrap(), *sample_val))
        }
        self.hover_info = Some(hover_info);
    }

    pub fn hover_info(&self) -> Option<&HoverInfo> {
        self.hover_info.as_ref()
    }

    pub fn view_buffer(&self) -> &model::ViewBuffer {
        &self.view_buffer
    }

    pub fn view_rect(&self) -> &rect::Rect {
        &self.view_rect
    }

    pub fn channel(&self) -> usize {
        self.channel_ix
    }

    pub fn audio_channel(&self) -> std::cell::Ref<'_, audio::Channel<f32>> {
        // Borrow the buffer immutably
        let buffer_ref = self.buffer.borrow();

        // Return a reference to the specific channel
        std::cell::Ref::map(buffer_ref, |buffer| &buffer[self.channel_ix])
    }

    pub fn samples_per_pixel(&self) -> Option<f32> {
        self.samples_per_pixel
    }
    pub fn set_samples_per_pixel(&mut self, samples_per_pixel: f32) {
        self.samples_per_pixel = Some(samples_per_pixel);
    }
}

fn distance_to_next_integer(x: audio::SampleIx) -> audio::SampleIx {
    if x.fract() == 0.0 {
        0.0
    } else if x > 0.0 {
        1.0 - x.fract()
    } else {
        x.fract().abs()
    }
}

/// functions to do with position/transformation of samples
impl Track {
    pub fn set_screen_rect(&mut self, screen_rect: rect::Rect) -> Result<()> {
        // Only update_view_buffer if we have to
        if self.screen_rect != screen_rect {
            self.screen_rect = screen_rect;

            if self.samples_per_pixel.is_some() {
                // sample rect end should be adjusted so it corresponds tot the screen rect and current
                // zoom level
                // TODO: maybe better in update_view_buffer, the if test above is breaking the abstraction
                let screen_rect_width = screen_rect.width();
                let sample_rect_width = self.view_x_to_sample_ix(screen_rect_width);
                self.sample_rect.resize_ix_rng(sample_rect_width);
            }
            self.update_view_buffer()?;
        }
        Ok(())
    }

    pub fn set_sample_rect(&mut self, sample_rect: audio::SampleRect) -> Result<()> {
        // Only update_view_buffer if we have to
        if self.sample_rect != sample_rect {
            self.sample_rect = sample_rect;
            self.update_view_buffer()?;
        }
        Ok(())
    }

    /// The zoom level in samples per pixel so that the whole buffer is visible
    pub fn get_zoom_for_full_buffer(&self) -> f32 {
        let nr_samples = self.sample_rect.ix_rng.len() as usize;
        let nr_pixels_x = self.screen_rect.width();
        if nr_pixels_x == 0.0 {
            return 0.0;
        }
        nr_samples as f32 / nr_pixels_x
    }

    /// Recalculate the view buffer, based on sample_rect, offset and screen_rect
    pub fn update_view_buffer(&mut self) -> Result<()> {
        // We need this information, if not available, don't 'draw' anything, and that's Ok()
        if self.sample_rect.ix_rng.positive_len() == 0 || self.screen_rect.width() == 0.0 {
            self.view_buffer.clear();
            return Ok(());
        }

        // Zoom level
        let samples_per_pixel = self.samples_per_pixel.get_or_insert(self.get_zoom_for_full_buffer());
        ensure!(
            *samples_per_pixel != 0.0,
            "We should have non-zero screen_rect width due to guard clause"
        );

        let nr_samples = self.sample_rect.ix_rng.positive_len();
        assert!(nr_samples > 0, "We should have non-zero sample_rect width due to guard clause");
        let screen_pixel_width = self.screen_rect.width();
        // self.samples_per_pixel = nr_samples as f32 / screen_pixel_width;

        let audio_buffer = self.buffer.borrow();

        // We want to calculate positions for sample_ixs in the sample_rect. However, the
        // sample_rect might have negative ix (it is before the audio). In that case we just start
        // in the beginning, i.e. no skip.
        let skip_nr_samples = self.sample_rect.ix_rng.start().max(0.0).ceil() as usize;
        // dbg!(self.sample_rect.ix_rng.start());
        // dbg!(skip_nr_samples);
        let mut fractional_offset = 0.0;
        if *samples_per_pixel <= 0.5 {
            // 0.3 -> 0.7
            // -0.3 -> 0.3
            let distance_to_next_integer = distance_to_next_integer(self.sample_rect.ix_rng.start());
            fractional_offset = distance_to_next_integer / *samples_per_pixel as audio::SampleIx;
        }
        // NOTE: 'integer' offset
        let mut first_sample_offset = 0.0;
        if self.sample_rect.ix_rng.start() < 0.0 {
            first_sample_offset = self.sample_rect.ix_rng.start().ceil().abs() as f32 / *samples_per_pixel;
        }
        let pixels_per_sample = 1.0 / *samples_per_pixel;
        // dbg!(*samples_per_pixel);
        // dbg!(pixels_per_sample);
        // dbg!(self.sample_rect.ix_rng.start());
        // dbg!(first_sample_offset);

        // TODO: probably want to store this to easily revert when e.g. 'clicking'
        let total_offset = first_sample_offset + fractional_offset as f32;

        // Iter over x adjusted positions for all samples that fall into the nr_pixels to fill
        let sample_pos_iter = audio_buffer[self.channel_ix]
            .samples()
            .skip(skip_nr_samples)
            .take(nr_samples)
            .enumerate()
            .map(|(sample_ix, sample)| {
                let pixel_x = (sample_ix as f32 / *samples_per_pixel).floor() + total_offset;
                let pixel_offset = 0.5; // to get them at the middle of pixel columns, vertical
                                        // lines then draw exactly on the pixel
                pos::Pos::new(pixel_x + pixel_offset, *sample)
            });

        // PERF: this recreates/reallocates the view buffer I guess. Probably better some other
        // data structure that persists?
        self.view_buffer = match *samples_per_pixel {
            sp if sp < 0.5 => {
                // we have at least 2 pixels width for each sample: draw each sample as a dot and a
                // line to the midline
                // Just collect the positions, the view will know who to draw them
                model::ViewBuffer::SingleSamples(sample_pos_iter.collect())
            }
            sp if sp >= 2.0 => {
                // we have at least two samples per pixel: draw a line for each pixel column
                // between the min and max y values of all the samples in that pixel
                model::ViewBuffer::LinePerPixelColumn(
                    sample_pos_iter
                        .chunk_by(|&pos| pos.x)
                        .into_iter()
                        .map(|(pixel_x, chunk)| {
                            let (min_sample, max_sample) = chunk.fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), pos| {
                                (min.min(pos.y), max.max(pos.y))
                            });
                            [pos::Pos::new(pixel_x, min_sample), pos::Pos::new(pixel_x, max_sample)]
                        })
                        .collect(),
                )
            }

            // In the other case we will draw a contiguous line through all the samples values.
            _ => model::ViewBuffer::OneLine(sample_pos_iter.collect()),
        };

        // view_rect:
        // y: sample range is sample_rect sample range
        // x: pixel width
        self.view_rect = rect::Rect::new(
            0.0,
            self.sample_rect.val_rng.max,
            self.screen_rect.width(),
            self.sample_rect.val_rng.min,
        );

        Ok(())
    }

    // move sample rect by dx_pixels
    pub fn shift_sample_rect_x(&mut self, dx_pixels: f32) -> Result<()> {
        if dx_pixels == 0.0 {
            return Ok(());
        }
        let Some(samples_per_pixel) = self.samples_per_pixel else {
            return Ok(());
        };
        let dx_samples = self.view_x_to_sample_ix(dx_pixels);
        self.sample_rect.shift_x(dx_samples);
        self.update_view_buffer()
    }

    /// * `center_screen_x` - The x position of the center of the zoom
    /// * `dx_samples` - The amount of pixels we add or remove. Positive -> zoom out, negative -> zoom in
    pub fn zoom_x(&mut self, center_screen_x: f32, dx_pixels: f32) -> Result<()> {
        // center normalized to x=[0,width]
        let center_view_x = center_screen_x - self.screen_rect.min.x;
        // center as sample ix
        let center_sample_ix = self.view_x_to_sample_ix(center_view_x) + self.sample_rect.ix_rng.start();

        // println!("\n\n\n\n");
        // dbg!(center_screen_x);
        // dbg!(center_view_x);
        // dbg!(center_sample_ix);

        // nr of samples to zoom
        let dx_samples = self.view_x_to_sample_ix(dx_pixels);
        // dbg!(dx_pixels);
        // dbg!(dx_samples);

        // update sample rect sample index range
        // dbg!(self.sample_rect);
        self.sample_rect.zoom_x(dx_samples, center_sample_ix);
        // dbg!(self.sample_rect);

        // recalculate samples per pixel based on new sample rect
        // dbg!(self.samples_per_pixel);
        self.samples_per_pixel = Some(self.sample_rect.ix_rng.len() as f32 / self.screen_rect.width());
        // dbg!(self.samples_per_pixel);

        self.update_view_buffer()
    }
}

/// Which sample index is closest to, or which sample indices are rendered at a screen pixel x
/// position (given 0 is the start pixel x, result is also relative to sample_ix 0)
pub fn sample_x_range(screen_x: i32, samples_per_pixel: f32) -> std::ops::Range<audio::SampleIx> {
    // dbg!(screen_x);
    let mut first_sample_ix = (screen_x as f32 * samples_per_pixel).ceil() as audio::SampleIx;
    // dbg!(first_sample_ix);
    // one past the last sample ix
    let mut last_sample_ix = ((screen_x as f32 + 1.0) * samples_per_pixel).floor() as audio::SampleIx;
    // dbg!(last_sample_ix);

    // due to float chenanigans, the start and end may be off by (at least?) one
    // e.g. samples_per_pixel = 2.4
    // sample_ix = 36
    // 36/2.4 = 14.99..
    // this results in screen_x 14
    // but: 15 * 2.4 = 36 (exactly)
    // this means that 36 would be returned for screen_x = 15
    while (first_sample_ix as f32 / samples_per_pixel).floor() >= screen_x as f32 {
        first_sample_ix -= 1.0;
    }
    while (first_sample_ix as f32 / samples_per_pixel).floor() < screen_x as f32 {
        first_sample_ix += 1.0;
    }
    while (last_sample_ix as f32 / samples_per_pixel).floor() <= screen_x as f32 {
        last_sample_ix += 1.0;
    }
    while (last_sample_ix as f32 / samples_per_pixel).floor() > screen_x as f32 {
        last_sample_ix -= 1.0;
    }

    // This means that there are more pixels than samples, we take the sample that is closest to
    // the pixel. NOTE that this prevents a bijection, as from sample to pixel we round down
    if first_sample_ix >= last_sample_ix {
        let closest = (screen_x as f32 * samples_per_pixel).round() as audio::SampleIx;
        return closest..closest + 1.0;
    }

    first_sample_ix..last_sample_ix + 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_full_scale_x_range() {
    //     let test_data = [
    //         (audio::SampleType::Float, 32, (-1.0, 1.0)),
    //         (audio::SampleType::Int, 16, (-32768.0, 32767.0)),
    //         (audio::SampleType::Int, 24, (-8388608.0, 8388607.0)),
    //     ];

    //     for (sample_type, bit_depth, (min, max)) in test_data {
    //         let (min2, max2) = super::full_scale_y_range(sample_type, bit_depth);
    //         assert_eq!(min, min2);
    //         assert_eq!(max, max2);
    //     }
    // }

    // #[test]
    // fn test_model_track() {
    //     println!("test_model_track");

    //     let mut ab = audio::BufferBuilder::new()
    //         .nr_channels(1)
    //         .nr_samples(10)
    //         .sample_rate(48000)
    //         .bit_depth(32)
    //         .sample_type(audio::SampleType::Float)
    //         .build::<f32>()
    //         .unwrap();

    //     for (ix, sample) in ab[0].iter_mut().enumerate() {
    //         *sample = ix as f32;
    //     }

    //     println!("{}: {}", "ab", ab);

    //     let mut track = crate::model::track::Track::new(Rc::new(RefCell::new(ab)), 0, "Test Track").unwrap();
    //     track.set_nr_pixels_to_fill_x(5);
    // }

    #[test]
    fn test_sample_x_range() {
        // NOTE: everyting below 2 for samples per pixel isn't an exact bijection because we round
        // down when going from sample to pixel and we just round when going from pixel to sample.
        // May want to make use the round too for sample to pixel calculation
        {
            let start = 0.2;
            let end = 20.0;
            let step = 2.0;
            let samples_per_pixel_iter = std::iter::successors(Some(start), move |&x| if x + step <= end { Some(x + step) } else { None });
            for samples_per_pixel in samples_per_pixel_iter {
                // println!("\n==> {}: {}", "samples_per_pixel", samples_per_pixel);
                // for sample_x in 0..1000000 {
                for sample_x in 0..1000000 {
                    let screen_x = (sample_x as f32 / samples_per_pixel).floor() as i32;
                    // println!();
                    // dbg!(sample_x);
                    // println!("{}: {}", "sample_x as f32 / samples_per_pixel", sample_x as f32 / samples_per_pixel);
                    // dbg!(screen_x);
                    let sample_ix_rng = sample_x_range(screen_x, samples_per_pixel);
                    // dbg!(&sample_ix_rng);
                    assert!(sample_ix_rng.contains(&(sample_x as f64)));
                }
            }
        }

        // Weird usecase where 2 numbers multiplied by 2.4 result in exactly 36
        {
            let samples_per_pixel = 2.4;
            //println!("{}: {:.50}", "samples_per_pixel", samples_per_pixel);
            let sample_ix = 36.0;
            let screen_x: f32 = sample_ix / samples_per_pixel;
            //println!("{:15}: {}", "sample_ix", sample_ix);
            let screen_x_floor = screen_x.floor();
            //println!("{:15}: {}", "screen_x_floor", screen_x_floor);

            let sample_ix_f = screen_x * samples_per_pixel;
            //println!("{:15}: {}", "screen_x", screen_x);
            //println!("{:15}: {:.50}", "sample_ix_f", sample_ix_f);

            let sample_ix_start = screen_x_floor * samples_per_pixel;
            let sample_ix_end = (screen_x_floor + 1.0) * samples_per_pixel;
        }
    }
}
