use crate::{
    audio::sample,
    model::{
        ruler::{ix_lattice::IxLattice, sample_ix_to_screen_x, screen_x_to_sample_ix},
        PixelCoord, SampleIxZoom,
    },
    rect,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoverInfo {
    pub sample_ix: i64,
    pub screen_x: f32,
}

pub enum HoverInfoE {
    HoverInfo(HoverInfo),
    None,
}

#[derive(Debug, Clone, Default)]
pub struct Time {
    /// When the mouse is over the ruler or any of the tracks, this represents the X pos
    pub hover_info: Option<HoverInfo>,

    // NOTE: empty rect means we don't have a screen rect yet
    screen_rect: rect::Rect,

    /// We (usually) set the time_line info when we have content to render
    pub time_line: Option<SampleIxZoom>,

    /// The sample index ticks/lattice to draw for current screen rect/time_line
    pub ix_lattice: IxLattice,
    // TODO: don't recalculate ix_lattice every time, only when needed
    // make API a bit cleaner
}

impl Time {
    // NOTE: we don't adjust the zoom level or ix_start intentionally
    pub fn set_screen_rect(&mut self, screen_rect: rect::Rect) {
        self.screen_rect = screen_rect;
    }

    pub fn screen_rect(&self) -> &rect::Rect {
        &self.screen_rect
    }

    pub fn set_samples_per_pixel(&mut self, samples_per_pixel: f64) {
        let timeline = self.time_line.get_or_insert_with(Default::default);
        timeline.set_samples_per_pixel(samples_per_pixel);
    }

    // TODO: recalculates every time, maybe not so bad as we update the screen rect every time
    // (though it doesn't actually change)
    pub fn ix_lattice(&mut self) -> Option<&IxLattice> {
        let time_line = self.time_line.as_ref()?;
        let ix_range = self.ix_range()?;
        self.ix_lattice
            .compute_ticks(ix_range, self.screen_rect, crate::view::ruler2::NR_PIXELS_PER_TICK)
            .ok()?;
        Some(&self.ix_lattice)
    }

    /// Check if fully initialized to something usable
    pub fn valid(&self) -> bool {
        self.screen_rect.width() > 0.0 && self.time_line.is_some() && self.time_line.as_ref().unwrap().samples_per_pixel() > 0.0
    }

    /// The current sample index range
    pub fn ix_range(&self) -> Option<sample::FracIxRange> {
        self.time_line.as_ref().map(|tl| tl.get_ix_range(self.screen_rect.width() as f64))
    }

    pub fn zoom_to_ix_range(&mut self, ix_range: sample::FracIxRange) {
        if self.screen_rect.width() == 0.0 {
            return;
        }
        self.set_samples_per_pixel(ix_range.len() / self.screen_rect.width() as f64);
        self.time_line.as_mut().unwrap().ix_start = ix_range.start;
    }

    pub fn sample_ix_to_screen_x(&self, sample_ix: f64) -> Option<f32> {
        let time_line = self.time_line.as_ref().or_else(|| {
            tracing::trace!("No time line");
            None
        })?;
        let ix_range = time_line.get_ix_range(self.screen_rect.width() as f64);
        Some(sample_ix_to_screen_x(sample_ix, ix_range, self.screen_rect))
    }

    pub fn screen_x_to_sample_ix(&self, screen_x: f32) -> Option<f64> {
        let ix_range = self.ix_range()?;
        Some(screen_x_to_sample_ix(screen_x, ix_range, self.screen_rect))
    }

    // Returns all the absolute sample indices that are visible in the given screen x pixel column
    // NOTE: because of floating point inaccuracies, this is not 100% accurate, but it doesn't
    // matter for our use case
    pub fn screen_x_to_sample_ix_range(&self, screen_x: f32) -> Option<sample::IxRange> {
        let screen_x = screen_x.floor();
        let start_ix = self.screen_x_to_sample_ix(screen_x)?;
        let end_ix = self.screen_x_to_sample_ix(screen_x + 1.0)?;

        todo!()
    }
}

// TODO: handle unwraps
impl Time {
    pub fn shift_x(&mut self, delta_pixels: PixelCoord) {
        if let Some(time_line) = self.time_line.as_mut() {
            let delta_sample_ixs = delta_pixels * time_line.samples_per_pixel() as f32;
            time_line.ix_start += delta_sample_ixs as f64;
            if let Some(mut hover_info) = self.hover_info {
                hover_info.sample_ix = self.screen_x_to_sample_ix(hover_info.screen_x).unwrap().floor() as i64;
                self.hover_info = Some(hover_info);
            }
        }
    }

    pub fn zoom_x(&mut self, nr_pixels: f32, center_x: f32) {
        if !self.screen_rect.contains_x(center_x) {
            return;
        }
        if self.time_line.is_none() {
            return;
        }
        let center_x_normalized = center_x - self.screen_rect.min.x;
        let frac_min = center_x_normalized / self.screen_rect.width();
        let new_min_x = self.screen_rect.min.x - frac_min * nr_pixels;
        let new_max_x = self.screen_rect.max.x + (1.0 - frac_min) * nr_pixels;
        let Some(new_min_ix) = self.screen_x_to_sample_ix(new_min_x) else {
            return;
        };
        let Some(new_max_ix) = self.screen_x_to_sample_ix(new_max_x) else {
            return;
        };
        let Some(time_line) = self.time_line.as_mut() else { return };
        time_line.ix_start = new_min_ix;
        self.set_samples_per_pixel((new_max_ix - new_min_ix) / self.screen_rect.width() as f64);
    }
}
