use crate::rect::Rect;

/// Camera/View over the time axis.
///
/// Maps a window of **seconds** to a window of pixels along the X axis. Sample-rate
/// independent: per-track sample-index conversion happens at the boundary using each
/// buffer's own rate via [`time_to_sample_ix`] / [`sample_ix_to_time`].
///
/// The owner of the screen rect (the time ruler, or a per-track painter) passes it in
/// to the conversion methods — `TimeCamera` itself is pure position + zoom.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TimeCamera {
    /// Time (seconds) at the left edge of the screen rect.
    pub time_start: f64,

    /// Zoom: seconds covered by one pixel.
    seconds_per_pixel: f64,
}

impl TimeCamera {
    pub fn seconds_per_pixel(&self) -> f64 {
        self.seconds_per_pixel
    }

    pub fn set_seconds_per_pixel(&mut self, seconds_per_pixel: f64) {
        self.seconds_per_pixel = seconds_per_pixel;
    }

    /// Visible `[time_start, time_end)` for the given screen-rect pixel width.
    pub fn time_range(&self, pixel_width: f64) -> std::ops::Range<f64> {
        assert!(pixel_width >= 0.0, "pixel_width must be >= 0");
        let end = self.time_start + pixel_width * self.seconds_per_pixel;
        self.time_start..end
    }

    pub fn time_to_screen_x(&self, time: f64, screen_rect: Rect) -> f32 {
        let offset_s = time - self.time_start;
        let offset_px = offset_s / self.seconds_per_pixel;
        screen_rect.left() + offset_px as f32
    }

    pub fn screen_x_to_time(&self, screen_x: f32, screen_rect: Rect) -> f64 {
        let offset_px = (screen_x - screen_rect.left()) as f64;
        self.time_start + offset_px * self.seconds_per_pixel
    }
}

/// Seconds → fractional sample index, given a buffer's sample rate.
pub fn time_to_sample_ix(time: f64, sample_rate: u32) -> f64 {
    time * sample_rate as f64
}

/// Fractional sample index → seconds, given a buffer's sample rate.
pub fn sample_ix_to_time(sample_ix: f64, sample_rate: u32) -> f64 {
    sample_ix / sample_rate as f64
}
