use crate::{audio::sample, rect::Rect};

use super::{ValueDisplayScale, display_range};

// assumes both ranges are valid
// This one is visually less stable, adjacent sample ix bin grouping shifts depending on
// parameters.
pub fn sample_ix_to_screen_x_2(
    sample_ix: f64,
    sample_ix_range: sample::FracIxRange,
    screen_rect: Rect,
) -> f32 {
    let sample_ix_offset = sample_ix - sample_ix_range.start;
    let sample_ix_frac = sample_ix_offset / sample_ix_range.len();
    screen_rect.left().floor() + sample_ix_frac as f32 * screen_rect.width().floor()
}

// This one is visually more stable that normal lerp.
// We take the sample ix 'global' bin and then shift that in place, so adjacent pixels
// always result in the same grouping per bin, with normal lerp these shift from bin to bin
pub fn sample_ix_to_screen_x(
    sample_ix: f64,
    sample_ix_range: sample::FracIxRange,
    screen_rect: Rect,
) -> f32 {
    let spp = sample_ix_range.len() / screen_rect.width() as f64;

    if spp > 2.0 {
        // global sample ix bin: stable grouping of adjacent sample indices
        let sample_ix_bin = (sample_ix / spp).floor();
        let sample_ix_range_start_offset_bin = (sample_ix_range.start / spp).floor();

        let res = sample_ix_bin as f32 - sample_ix_range_start_offset_bin as f32;
        res + screen_rect.left()
    } else {
        sample_ix_to_screen_x_2(sample_ix, sample_ix_range, screen_rect)
    }
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
    display_scale: ValueDisplayScale,
) -> Option<f32> {
    let display_range = display_range::sample_to_display_range(val_range, display_scale);
    let min = display_range.min;
    let max = display_range.max;
    let value = display_scale.sample_to_display(sample_value);
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
    display_scale: ValueDisplayScale,
) -> Option<f64> {
    let display_range = display_range::sample_to_display_range(val_range, display_scale);
    let min = display_range.min;
    let max = display_range.max;
    let range_len = max - min;
    if range_len == 0.0 {
        return None;
    }

    // Normalize screen Y into [0, 1], inverted
    let frac = (screen_rect.bottom() - screen_y) as f64 / screen_rect.height() as f64;

    Some(display_scale.display_to_sample(min + frac * range_len))
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
    use crate::model::ruler::ValueDisplayScale;

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
        let display_scale = ValueDisplayScale::default();

        let y_top = sample_value_to_screen_y(1.0, range, rect, display_scale).unwrap();
        let y_bottom = sample_value_to_screen_y(-1.0, range, rect, display_scale).unwrap();

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
        let display_scale = ValueDisplayScale::default();

        let y = sample_value_to_screen_y(0.0, range, rect, display_scale).unwrap();

        assert!((y - 50.0).abs() < 1.0);
    }

    #[test]
    fn screen_to_sample_round_trip_is_reasonable() {
        let rect = Rect::new(0.0, 0.0, 100.0, 200.0);
        let range = sample::ValRange {
            min: -1.0f64,
            max: 1.0,
        };
        let display_scale = ValueDisplayScale::default();

        let original_y = 42.0;
        let sample = screen_y_to_sample_value(original_y, range, rect, display_scale).unwrap();
        let y_back = sample_value_to_screen_y(sample, range, rect, display_scale).unwrap();

        assert!((original_y - y_back).abs() < 0.5);
    }

    #[test]
    fn skewed_round_trip_is_reasonable() {
        let rect = Rect::new(0.0, 0.0, 100.0, 200.0);
        let range = sample::ValRange {
            min: -1.0f64,
            max: 1.0,
        };
        let display_scale = ValueDisplayScale { skew_factor: 1.0 };

        let original_y = 25.0;
        let sample = screen_y_to_sample_value(original_y, range, rect, display_scale).unwrap();
        let y_back = sample_value_to_screen_y(sample, range, rect, display_scale).unwrap();

        assert!((original_y - y_back).abs() < 0.5);
    }

    #[allow(dead_code)]
    fn linspace(start: f64, end: f64, steps: usize) -> impl Iterator<Item = f64> {
        (0..=steps).map(move |i| start + (end - start) * (i as f64 / steps as f64))
    }

    // #[test]
    // fn test_emile() {
    //     println!("test_emile");
    //     let mut sample_ix_range = crate::audio::sample::FracIxRange {
    //         start: 0.0,
    //         end: 10.0,
    //     };
    //     let screen_rect = Rect::new(0.0, 0.0, 4.0, 4.0);
    //     let samples_per_pixel = sample_ix_range.len() / screen_rect.width() as f64;
    //     dbg!(samples_per_pixel);
    //     let ix_rng = 10..12;

    //     for sample_ix in ix_rng.clone() {
    //         let screen_x = sample_ix_to_screen_x(sample_ix as f64, sample_ix_range, screen_rect);
    //         println!(
    //             "{:8.2} -> {:8.2} -> {:8.2}",
    //             sample_ix,
    //             screen_x,
    //             screen_x.floor()
    //         );
    //     }

    //     println!();

    //     for shift in linspace(0.0, 1.0, 5) {
    //     // let shift = 0.5;
    //     // {
    //     dbg!(shift);
    //         sample_ix_range.shift(shift);
    //         let samples_per_pixel = sample_ix_range.len() / screen_rect.width() as f64;
    //         dbg!(samples_per_pixel);
    //         for sample_ix in ix_rng.clone() {
    //             let screen_x =
    //                 sample_ix_to_screen_x(sample_ix as f64, sample_ix_range, screen_rect);
    //             println!(
    //                 "{:8.2} -> {:8.2} -> {:8.2}",
    //                 sample_ix,
    //                 screen_x,
    //                 screen_x.floor()
    //             );
    //         }
    //     }
    // }
}
