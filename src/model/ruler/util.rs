use crate::{audio::sample, rect::Rect};

// assumes both ranges are valid
pub fn sample_ix_to_screen_x(
    sample_ix: f64,
    sample_ix_range: sample::FracIxRange,
    screen_rect: Rect,
) -> f32 {
    let sample_ix_offset = sample_ix - sample_ix_range.start;
    let sample_ix_frac = sample_ix_offset / sample_ix_range.len();
    screen_rect.left() + sample_ix_frac as f32 * screen_rect.width()
}

// assumes both ranges are valid
pub fn screen_x_to_sample_ix(
    screen_x: f32,
    sample_ix_range: sample::FracIxRange,
    screen_rect: Rect,
) -> f64 {
    let screen_x_offset = screen_x - screen_rect.left();
    let sample_ix_frac = screen_x_offset / screen_rect.width();
    sample_ix_range.start + sample_ix_frac as f64 * sample_ix_range.len()
    // let sample_ix =
    // sample_ix
}

// PERF: remove range_len check, this can be in a per sample basis, while the range should be
// checked per buffer.
pub fn sample_value_to_screen_y(
    sample_value: f64,
    val_range: sample::ValRange<f64>,
    screen_rect: Rect,
) -> Option<f32> {
    let min = val_range.min;
    let max = val_range.max;
    let value = sample_value;
    let range_len = max - min;
    if range_len == 0.0 {
        return None;
    }

    // Normalize sample into [0, 1]
    let frac = (value - min) / range_len;

    // Invert Y axis: max value at top, min at bottom
    Some(screen_rect.bottom() - frac as f32 * screen_rect.height())
}

pub fn screen_y_to_sample_value(
    screen_y: f32,
    val_range: sample::ValRange<f64>,
    screen_rect: Rect,
) -> Option<f64> {
    let min = val_range.min;
    let max = val_range.max;
    let range_len = max - min;
    if range_len == 0.0 {
        return None;
    }

    // Normalize screen Y into [0, 1], inverted
    let frac = (screen_rect.bottom() - screen_y) as f64 / screen_rect.height() as f64;

    Some(min + frac * range_len)
}

// smallest multiple of m that is >= x
// e.g. -120, 50 -> -100
pub fn ceil_to_multiple(x: i64, m: i64) -> i64 {
    if x % m == 0 {
        x
    } else {
        x + (m - x.rem_euclid(m))
    }
}

// largest multiple of m that is <= x
pub fn floor_to_multiple(x: i64, m: i64) -> i64 {
    x - x.rem_euclid(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ceil_to_multiple() {
        assert_eq!(ceil_to_multiple(-120, 50), -100);
        assert_eq!(ceil_to_multiple(-100, 50), -100);
        assert_eq!(ceil_to_multiple(-50, 50), -50);
        assert_eq!(ceil_to_multiple(0, 50), 0);
        assert_eq!(ceil_to_multiple(50, 50), 50);
        assert_eq!(ceil_to_multiple(99, 50), 100);
        assert_eq!(ceil_to_multiple(100, 50), 100);
    }

    #[test]
    fn test_floor_to_multiple() {
        assert_eq!(floor_to_multiple(-120, 50), -150);
        assert_eq!(floor_to_multiple(-100, 50), -100);
        assert_eq!(floor_to_multiple(-50, 50), -50);
        assert_eq!(floor_to_multiple(0, 50), 0);
        assert_eq!(floor_to_multiple(50, 50), 50);
        assert_eq!(floor_to_multiple(99, 50), 50);
        assert_eq!(floor_to_multiple(100, 50), 100);
    }

    #[test]
    fn sample_extremes_map_to_screen_extremes() {
        let rect = Rect::new(0.0, 10.0, 100.0, 110.0);
        let range = sample::ValRange {
            min: -1.0f64,
            max: 1.0,
        };

        let y_top = sample_value_to_screen_y(1.0, range, rect).unwrap();
        let y_bottom = sample_value_to_screen_y(-1.0, range, rect).unwrap();

        assert!((y_top - rect.top()).abs() < 0.001);
        assert!((y_bottom - rect.bottom()).abs() < 0.001);
    }

    #[test]
    fn sample_midpoint_maps_to_screen_center() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let range = sample::ValRange {
            min: -1.0,
            max: 1.0,
        };

        let y = sample_value_to_screen_y(0.0, range, rect).unwrap();

        assert!((y - 50.0).abs() < 1.0);
    }

    #[test]
    fn screen_to_sample_round_trip_is_reasonable() {
        let rect = Rect::new(0.0, 0.0, 100.0, 200.0);
        let range = sample::ValRange {
            min: -1.0f64,
            max: 1.0,
        };

        let original_y = 42.0;
        let sample = screen_y_to_sample_value(original_y, range, rect).unwrap();
        let y_back = sample_value_to_screen_y(sample, range, rect).unwrap();

        assert!((original_y - y_back).abs() < 0.5);
    }
}
