use crate::audio::sample;
use crate::rect::Rect;

/// Convert a vertical pixel delta to a sample-value delta for the given range.
pub fn pixels_to_value_delta(
    delta_pixels: f32,
    val_range: sample::ValRangeE,
    screen_rect: Rect,
) -> f64 {
    if screen_rect.height() == 0.0 || val_range.is_empty() {
        return 0.0;
    }
    let range = val_range_len(val_range);
    (delta_pixels as f64 / screen_rect.height() as f64) * range
}

/// Return the numeric length of the value range as f64.
pub fn val_range_len(val_range: sample::ValRangeE) -> f64 {
    match val_range {
        sample::ValRangeE::PCM16(range) => range.len(),
        sample::ValRangeE::PCM24(range) => range.len(),
        sample::ValRangeE::PCM32(range) => range.len(),
        sample::ValRangeE::F32(range) => range.len(),
    }
}

/// Shift a value range by a floating-point delta.
pub fn pan_val_range(val_range: sample::ValRangeE, delta: f64) -> sample::ValRangeE {
    match val_range {
        sample::ValRangeE::PCM16(mut range) => {
            let shift = delta.round() as i16;
            range.min = range.min.saturating_add(shift);
            range.max = range.max.saturating_add(shift);
            sample::ValRangeE::PCM16(range)
        }
        sample::ValRangeE::PCM24(mut range) => {
            let shift = delta.round() as i32;
            range.min = range.min.saturating_add(shift);
            range.max = range.max.saturating_add(shift);
            sample::ValRangeE::PCM24(range)
        }
        sample::ValRangeE::PCM32(mut range) => {
            let shift = delta.round() as i32;
            range.min = range.min.saturating_add(shift);
            range.max = range.max.saturating_add(shift);
            sample::ValRangeE::PCM32(range)
        }
        sample::ValRangeE::F32(mut range) => {
            let shift = delta as f32;
            range.min += shift;
            range.max += shift;
            sample::ValRangeE::F32(range)
        }
    }
}

/// Zoom a value range by a delta, around a normalized center (0.0..=1.0).
pub fn zoom_val_range(
    val_range: sample::ValRangeE,
    delta: f64,
    center_frac: f64,
) -> sample::ValRangeE {
    let center_frac = center_frac.clamp(0.0, 1.0);
    let delta_min = delta * center_frac;
    let delta_max = delta * (1.0 - center_frac);
    match val_range {
        sample::ValRangeE::PCM16(mut range) => {
            let shift_min = delta_min.round() as i16;
            let shift_max = delta_max.round() as i16;
            range.min = range.min.saturating_sub(shift_min);
            range.max = range.max.saturating_add(shift_max);
            if range.min > range.max {
                range.min = range.max;
            }
            sample::ValRangeE::PCM16(range)
        }
        sample::ValRangeE::PCM24(mut range) => {
            let shift_min = delta_min.round() as i32;
            let shift_max = delta_max.round() as i32;
            range.min = range.min.saturating_sub(shift_min);
            range.max = range.max.saturating_add(shift_max);
            if range.min > range.max {
                range.min = range.max;
            }
            sample::ValRangeE::PCM24(range)
        }
        sample::ValRangeE::PCM32(mut range) => {
            let shift_min = delta_min.round() as i32;
            let shift_max = delta_max.round() as i32;
            range.min = range.min.saturating_sub(shift_min);
            range.max = range.max.saturating_add(shift_max);
            if range.min > range.max {
                range.min = range.max;
            }
            sample::ValRangeE::PCM32(range)
        }
        sample::ValRangeE::F32(mut range) => {
            let shift_min = delta_min as f32;
            let shift_max = delta_max as f32;
            range.min -= shift_min;
            range.max += shift_max;
            if range.min > range.max {
                range.min = range.max;
            }
            sample::ValRangeE::F32(range)
        }
    }
}
