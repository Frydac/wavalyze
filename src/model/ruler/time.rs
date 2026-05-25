use crate::{
    audio::sample,
    model::{
        TimeCamera,
        ruler::{ix_lattice::IxLattice, sample_ix_to_screen_x, screen_x_to_sample_ix},
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

/// Time ruler rendering state. Owns its own screen rect and the lattice of tick positions,
/// but no longer the X-axis camera — that lives on `Tracks` as `time_camera` and is passed
/// in to the methods that need it.
#[derive(Debug, Clone, Default)]
pub struct Time {
    /// When the mouse is over the ruler or any of the tracks, this represents the X pos
    pub hover_info: Option<HoverInfo>,

    // NOTE: empty rect means we don't have a screen rect yet
    screen_rect: rect::Rect,

    /// The sample index ticks/lattice to draw for current screen rect + camera.
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

    /// Recompute and return the tick lattice for `ix_range` projected on the ruler's screen
    /// rect. `ix_range` comes from the caller (`Tracks::ix_range()`), since the camera lives
    /// outside the ruler.
    pub fn ix_lattice_for(&mut self, ix_range: sample::FracIxRange) -> Option<&IxLattice> {
        self.ix_lattice
            .compute_ticks(
                ix_range,
                self.screen_rect,
                crate::view::ruler::NR_PIXELS_PER_TICK,
            )
            .ok()?;
        Some(&self.ix_lattice)
    }

    /// True once the ruler has a non-empty screen rect *and* the supplied camera has a
    /// meaningful zoom level. Callers that want to gate ruler rendering use this.
    pub fn valid_with(&self, camera: &TimeCamera) -> bool {
        self.screen_rect.width() > 0.0 && camera.seconds_per_pixel() > 0.0
    }

    /// Translate a single sample-ix to a screen-x using the explicit camera + sample rate.
    /// Picks the stable bin-based mapping when spp > 2 so adjacent indices group identically
    /// across frames; otherwise falls back to plain lerp.
    pub fn sample_ix_to_screen_x_with(&self, sample_ix: f64, ix_range: sample::FracIxRange) -> f32 {
        sample_ix_to_screen_x(sample_ix, ix_range, self.screen_rect)
    }

    pub fn screen_x_to_sample_ix_with(&self, screen_x: f32, ix_range: sample::FracIxRange) -> f64 {
        screen_x_to_sample_ix(screen_x, ix_range, self.screen_rect)
    }
}
