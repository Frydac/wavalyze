use crate::{
    audio::sample,
    model::{
        ruler::{ix_lattice::IxLattice, sample_ix_to_screen_x, screen_x_to_sample_ix},
        timeline::Timeline2,
    },
    rect,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HoverInfo {
    pub sample_ix: i64,
    pub screen_x: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Time {
    /// When the mouse is over the ruler or any of the tracks, this represents the X pos
    pub hover_info: Option<HoverInfo>,

    // NOTE: empty rect means we don't have a screen rect yet
    screen_rect: rect::Rect,

    /// We (usually) set the time_line info when we have content to render
    time_line: Option<Timeline2>,

    /// The sample index ticks/lattice to draw for current screen rect/time_line
    ix_lattice: IxLattice,
}

impl Time {
    // NOTE: we don't adjust the zoom level or ix_start intentionally
    pub fn set_screen_rect(&mut self, screen_rect: rect::Rect) {
        self.screen_rect = screen_rect;
    }

    pub fn set_samples_per_pixel(&mut self, samples_per_pixel: f64) {
        let timeline = self.time_line.get_or_insert_with(Default::default);
        timeline.samples_per_pixel = samples_per_pixel;
    }

    // TODO: recalculates every time, maybe not so bad as we update the screen rect every time
    // (though it doesn't actually change)
    pub fn ix_lattice(&mut self) -> Option<&IxLattice> {
        let time_line = self.time_line.as_ref()?;
        let ix_range = self.ix_range()?;
        self.ix_lattice.compute_ticks(ix_range, self.screen_rect, crate::view::ruler2::NR_PIXELS_PER_TICK).ok()?;
        Some(&self.ix_lattice)
    }

    /// Check if fully initialized to something usable
    pub fn valid(&self) -> bool {
        self.screen_rect.width() > 0.0 && self.time_line.is_some() && self.time_line.as_ref().unwrap().samples_per_pixel > 0.0
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
        // if !ix_range.contains(sample_ix) {
        //     tracing::trace!("sample_ix {} not in ix_range {:?}", sample_ix, ix_range);
        //     return None;
        // }
        // let sample_ix_offset = sample_ix - ix_range.start;
        // let sample_ix_frac = sample_ix_offset / ix_range.len();
        // let screen_x = self.screen_rect.left() + sample_ix_frac as f32 * self.screen_rect.width();
        // Some(screen_x)
        sample_ix_to_screen_x(sample_ix, ix_range, self.screen_rect)
    }

    pub fn screen_x_to_sample_ix(&self, screen_x: f32) -> Option<f64> {
        let Some(ix_range) = self.ix_range() else { return None };
        screen_x_to_sample_ix(screen_x, ix_range, self.screen_rect)
        // if !self.screen_rect.contains_x(screen_x) {
        //     tracing::trace!("screen_x {} not in screen_rect {:?}", screen_x, self.screen_rect);
        //     return None;
        // }
        // let ix_range = time_line.get_ix_range(self.screen_rect.width() as f64);
        // let screen_x_offset = screen_x - self.screen_rect.left();
        // let sample_ix_frac = screen_x_offset / self.screen_rect.width();
        // let sample_ix = ix_range.start + sample_ix_frac as f64 * ix_range.len();
        // Some(sample_ix)
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
