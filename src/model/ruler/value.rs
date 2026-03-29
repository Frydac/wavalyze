use crate::audio::sample;
use crate::rect::Rect;

use super::ValueDisplayScale;

/// Convert a vertical pixel delta to a sample-value delta for the given range.
pub fn pixels_to_value_delta(
    delta_pixels: f32,
    val_range: sample::ValRange<f64>,
    screen_rect: Rect,
    _display_scale: ValueDisplayScale,
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

pub fn pan_val_range_with_scale(
    val_range: sample::ValRange<f64>,
    delta_pixels: f32,
    screen_rect: Rect,
    display_scale: ValueDisplayScale,
) -> sample::ValRange<f64> {
    let delta = pixels_to_value_delta(delta_pixels, val_range, screen_rect, display_scale);
    pan_val_range(val_range, delta)
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
    _display_scale: ValueDisplayScale,
) -> sample::ValRange<f64> {
    if delta_pixels == 0.0 || !screen_rect.contains_y(center_y) {
        return val_range;
    }

    let delta = pixels_to_value_delta(
        delta_pixels,
        val_range,
        screen_rect,
        ValueDisplayScale::default(),
    );
    let range_len = val_range_len(val_range);
    if delta < 0.0 && delta.abs() >= range_len {
        return val_range;
    }

    let center_frac =
        ((screen_rect.bottom() - center_y) as f64 / screen_rect.height() as f64).clamp(0.0, 1.0);
    let zoomed = zoom_val_range(val_range, delta, center_frac);
    if zoomed.is_empty() {
        return val_range;
    }

    zoomed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panning_is_independent_of_skew_factor() {
        let rect = Rect::new(0.0, 0.0, 100.0, 200.0);
        let range = sample::ValRange {
            min: -0.5f64,
            max: 1.5,
        };

        let linear = pan_val_range_with_scale(range, 40.0, rect, ValueDisplayScale::default());
        let skewed =
            pan_val_range_with_scale(range, 40.0, rect, ValueDisplayScale { skew_factor: 1.0 });

        assert_eq!(linear, skewed);
    }

    #[test]
    fn zooming_is_independent_of_skew_factor() {
        let rect = Rect::new(0.0, 0.0, 100.0, 200.0);
        let range = sample::ValRange {
            min: -0.5f64,
            max: 1.5,
        };

        let linear =
            zoom_val_range_with_scale(range, -30.0, 80.0, rect, ValueDisplayScale::default());
        let skewed = zoom_val_range_with_scale(
            range,
            -30.0,
            80.0,
            rect,
            ValueDisplayScale { skew_factor: 1.0 },
        );

        assert_eq!(linear, skewed);
    }
}
