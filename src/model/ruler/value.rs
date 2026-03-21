use crate::audio::sample;
use crate::rect::Rect;

use super::{ValueDisplayScale, display_range, screen_y_to_sample_value};

/// Convert a vertical pixel delta to a sample-value delta for the given range.
pub fn pixels_to_value_delta(
    delta_pixels: f32,
    val_range: sample::ValRange<f64>,
    screen_rect: Rect,
    display_scale: ValueDisplayScale,
) -> f64 {
    let display_range = display_range::sample_to_display_range(val_range, display_scale);
    if screen_rect.height() == 0.0 || display_range.is_empty() {
        return 0.0;
    }
    let range = val_range_len(display_range);
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

pub fn pan_val_range_with_scale(
    val_range: sample::ValRange<f64>,
    delta_pixels: f32,
    screen_rect: Rect,
    display_scale: ValueDisplayScale,
) -> sample::ValRange<f64> {
    let display_delta = pixels_to_value_delta(delta_pixels, val_range, screen_rect, display_scale);
    let display_range = display_range::sample_to_display_range(val_range, display_scale);
    let shifted = pan_val_range(display_range, display_delta);
    display_range::display_to_sample_range(shifted, display_scale)
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

pub fn zoom_val_range_with_scale(
    val_range: sample::ValRange<f64>,
    delta_pixels: f32,
    center_y: f32,
    screen_rect: Rect,
    display_scale: ValueDisplayScale,
) -> sample::ValRange<f64> {
    if delta_pixels == 0.0 || !screen_rect.contains_y(center_y) {
        return val_range;
    }

    let display_range = display_range::sample_to_display_range(val_range, display_scale);
    let delta_display = pixels_to_value_delta(delta_pixels, val_range, screen_rect, display_scale);
    let range_len = val_range_len(display_range);
    if delta_display < 0.0 && delta_display.abs() >= range_len {
        return val_range;
    }

    let center_sample =
        match screen_y_to_sample_value(center_y, val_range, screen_rect, display_scale) {
            Some(value) => value,
            None => return val_range,
        };
    let center_display = display_scale.sample_to_display(center_sample);
    let center_frac = (center_display - display_range.min) / range_len;
    let zoomed = zoom_val_range(display_range, delta_display, center_frac);
    if zoomed.is_empty() {
        return val_range;
    }

    display_range::display_to_sample_range(zoomed, display_scale)
}
