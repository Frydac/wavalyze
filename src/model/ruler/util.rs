use crate::{audio::sample, rect::Rect};

use super::ValueDisplayScale;

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
    if val_range.is_empty() {
        return None;
    }

    let segment = build_value_segment_for_sample(sample_value, val_range, screen_rect);
    let frac = segment.sample_to_frac(sample_value, display_scale)?;
    Some(segment.frac_to_screen_y(frac))
}

pub fn screen_y_to_sample_value(
    screen_y: f32,
    val_range: sample::ValRange<f64>,
    screen_rect: Rect,
    display_scale: ValueDisplayScale,
) -> Option<f64> {
    if val_range.is_empty() {
        return None;
    }

    let segments = build_value_segments(val_range, screen_rect);
    let segment = segments
        .iter()
        .find(|segment| segment.contains_screen_y(screen_y))
        .or_else(|| segments.last())?;
    let frac = segment.screen_y_to_frac(screen_y)?;
    segment.frac_to_sample(frac, display_scale)
}

fn raw_sample_value_to_screen_y(
    sample_value: f64,
    val_range: sample::ValRange<f64>,
    screen_rect: Rect,
) -> f32 {
    let frac = (sample_value - val_range.min) / val_range.len();
    screen_rect.bottom() - frac as f32 * screen_rect.height()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegmentKind {
    Linear,
    SkewedPositive,
    SkewedNegative,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ValueSegment {
    sample_start: f64,
    sample_end: f64,
    screen_y_start: f32,
    screen_y_end: f32,
    kind: SegmentKind,
}

impl ValueSegment {
    fn contains_screen_y(self, screen_y: f32) -> bool {
        let min_y = self.screen_y_end.min(self.screen_y_start);
        let max_y = self.screen_y_end.max(self.screen_y_start);
        screen_y >= min_y && screen_y <= max_y
    }

    fn sample_to_frac(self, sample_value: f64, display_scale: ValueDisplayScale) -> Option<f64> {
        let sample_len = self.sample_end - self.sample_start;
        if sample_len == 0.0 {
            return None;
        }

        match self.kind {
            SegmentKind::Linear => Some((sample_value - self.sample_start) / sample_len),
            SegmentKind::SkewedPositive => {
                let offset = sample_value - self.sample_start;
                Some(display_scale.sample_to_display(offset / sample_len))
            }
            SegmentKind::SkewedNegative => {
                let offset = self.sample_end - sample_value;
                Some(1.0 - display_scale.sample_to_display(offset / sample_len))
            }
        }
    }

    fn frac_to_sample(self, frac: f64, display_scale: ValueDisplayScale) -> Option<f64> {
        let frac = frac.clamp(0.0, 1.0);
        match self.kind {
            SegmentKind::Linear => {
                Some(self.sample_start + frac * (self.sample_end - self.sample_start))
            }
            SegmentKind::SkewedPositive => Some(
                self.sample_start
                    + display_scale.display_to_sample(frac) * (self.sample_end - self.sample_start),
            ),
            SegmentKind::SkewedNegative => Some(
                self.sample_end
                    - display_scale.display_to_sample(1.0 - frac)
                        * (self.sample_end - self.sample_start),
            ),
        }
    }

    fn frac_to_screen_y(self, frac: f64) -> f32 {
        let frac = frac.clamp(0.0, 1.0) as f32;
        self.screen_y_start + frac * (self.screen_y_end - self.screen_y_start)
    }

    fn screen_y_to_frac(self, screen_y: f32) -> Option<f64> {
        let screen_len = self.screen_y_end - self.screen_y_start;
        if screen_len == 0.0 {
            return None;
        }
        Some(((screen_y - self.screen_y_start) / screen_len) as f64)
    }
}

fn build_value_segments(val_range: sample::ValRange<f64>, screen_rect: Rect) -> Vec<ValueSegment> {
    let start_anchor = val_range.min.floor() as i64;
    let end_anchor = val_range.max.ceil() as i64;

    (start_anchor..end_anchor)
        .map(|anchor| {
            let sample_start = anchor as f64;
            let sample_end = sample_start + 1.0;

            let kind = if sample_end <= 0.0 {
                SegmentKind::SkewedNegative
            } else if sample_start >= 0.0 {
                SegmentKind::SkewedPositive
            } else {
                SegmentKind::Linear
            };

            ValueSegment {
                sample_start,
                sample_end,
                screen_y_start: raw_sample_value_to_screen_y(sample_start, val_range, screen_rect),
                screen_y_end: raw_sample_value_to_screen_y(sample_end, val_range, screen_rect),
                kind,
            }
        })
        .collect()
}

fn build_value_segment_for_sample(
    sample_value: f64,
    val_range: sample::ValRange<f64>,
    screen_rect: Rect,
) -> ValueSegment {
    let anchor = sample_value.floor() as i64;
    let sample_start = anchor as f64;
    let sample_end = sample_start + 1.0;

    let kind = if sample_end <= 0.0 {
        SegmentKind::SkewedNegative
    } else if sample_start >= 0.0 {
        SegmentKind::SkewedPositive
    } else {
        SegmentKind::Linear
    };

    ValueSegment {
        sample_start,
        sample_end,
        screen_y_start: raw_sample_value_to_screen_y(sample_start, val_range, screen_rect),
        screen_y_end: raw_sample_value_to_screen_y(sample_end, val_range, screen_rect),
        kind,
    }
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

    #[test]
    fn skew_keeps_full_scale_anchors_fixed_when_range_is_panned() {
        let rect = Rect::new(0.0, 0.0, 100.0, 200.0);
        let range = sample::ValRange {
            min: -0.5f64,
            max: 1.5,
        };

        for anchor in [-1.0, 0.0, 1.0] {
            let y_linear =
                sample_value_to_screen_y(anchor, range, rect, ValueDisplayScale::default())
                    .unwrap();
            let y_skewed = sample_value_to_screen_y(
                anchor,
                range,
                rect,
                ValueDisplayScale { skew_factor: 1.0 },
            )
            .unwrap();

            assert!((y_linear - y_skewed).abs() < 0.001);
        }
    }

    #[test]
    fn skew_strength_does_not_drop_when_zero_leaves_view() {
        let rect = Rect::new(0.0, 0.0, 100.0, 200.0);
        let with_zero = sample::ValRange {
            min: -0.2f64,
            max: 1.8,
        };
        let without_zero = sample::ValRange {
            min: 0.2f64,
            max: 2.2,
        };
        let scale = ValueDisplayScale { skew_factor: 5.0 };
        let probe = 0.5;

        let linear_with_zero =
            sample_value_to_screen_y(probe, with_zero, rect, ValueDisplayScale::default()).unwrap();
        let skew_with_zero = sample_value_to_screen_y(probe, with_zero, rect, scale).unwrap();
        let linear_without_zero =
            sample_value_to_screen_y(probe, without_zero, rect, ValueDisplayScale::default())
                .unwrap();
        let skew_without_zero = sample_value_to_screen_y(probe, without_zero, rect, scale).unwrap();

        let offset_with_zero = skew_with_zero - linear_with_zero;
        let offset_without_zero = skew_without_zero - linear_without_zero;

        assert!((offset_with_zero - offset_without_zero).abs() < 0.001);
    }

    #[test]
    fn zero_maps_below_view_when_visible_range_is_above_zero() {
        let rect = Rect::new(0.0, 0.0, 100.0, 200.0);
        let range = sample::ValRange {
            min: 0.2f64,
            max: 2.2,
        };
        let scale = ValueDisplayScale { skew_factor: 5.0 };

        let y_zero = sample_value_to_screen_y(0.0, range, rect, scale).unwrap();
        let y_two = sample_value_to_screen_y(2.0, range, rect, scale).unwrap();

        assert!(y_zero > rect.bottom());
        assert!((y_zero - y_two).abs() > 0.001);
    }

    #[test]
    fn zero_maps_above_view_when_visible_range_is_below_zero() {
        let rect = Rect::new(0.0, 0.0, 100.0, 200.0);
        let range = sample::ValRange {
            min: -2.2f64,
            max: -0.2,
        };
        let scale = ValueDisplayScale { skew_factor: 5.0 };

        let y_zero = sample_value_to_screen_y(0.0, range, rect, scale).unwrap();
        let y_neg_two = sample_value_to_screen_y(-2.0, range, rect, scale).unwrap();

        assert!(y_zero < rect.top());
        assert!((y_zero - y_neg_two).abs() > 0.001);
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
