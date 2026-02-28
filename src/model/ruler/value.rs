use crate::audio::sample;
use crate::rect::Rect;

/// Convert a vertical pixel delta to a sample-value delta for the given range.
pub fn pixels_to_value_delta(
    delta_pixels: f32,
    val_range: sample::ValRange<f64>,
    screen_rect: Rect,
) -> f64 {
    if screen_rect.height() == 0.0 || val_range.is_empty() {
        return 0.0;
    }
    let range = val_range_len(val_range);
    (delta_pixels as f64 / screen_rect.height() as f64) * range
}

/// Return the numeric length of the value range as f64.
pub fn val_range_len(val_range: sample::ValRange<f64>) -> f64 {
    val_range.len()
}

/// Shift a value range by a floating-point delta.
pub fn pan_val_range(mut val_range: sample::ValRange<f64>, delta: f64) -> sample::ValRange<f64> {
    val_range.min += delta;
    val_range.max += delta;
    val_range
}

/// Zoom a value range by a delta, around a normalized center (0.0..=1.0).
pub fn zoom_val_range(
    mut val_range: sample::ValRange<f64>,
    delta: f64,
    center_frac: f64,
) -> sample::ValRange<f64> {
    let center_frac = center_frac.clamp(0.0, 1.0);
    let delta_min = delta * center_frac;
    let delta_max = delta * (1.0 - center_frac);
    val_range.min -= delta_min;
    val_range.max += delta_max;
    if val_range.min > val_range.max {
        val_range.min = val_range.max;
    }
    val_range
}
