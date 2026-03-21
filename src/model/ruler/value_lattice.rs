use crate::{
    audio::sample,
    model::ruler::{TickType, ValueDisplayScale, floor_to_multiple, sample_value_to_screen_y},
    rect,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValueTick {
    pub sample_value: f64,
    pub screen_y: f32,
    pub tick_type: TickType,
}

#[derive(Debug, Clone, Default)]
pub struct ValueLattice {
    pub ticks: Vec<ValueTick>,
    pub major_step: f64,
}

impl ValueLattice {
    pub fn compute_ticks(
        &mut self,
        val_range: sample::ValRange<f64>,
        screen_rect: rect::Rect,
        nr_pixels_per_tick: f32,
        display_scale: ValueDisplayScale,
    ) -> anyhow::Result<()> {
        let max_nr_ticks = screen_rect.height() / nr_pixels_per_tick;
        let range_len = val_range.len();
        if screen_rect.height() <= 0.0 || max_nr_ticks <= 0.0 || range_len == 0.0 {
            anyhow::bail!("value range or screen rect invalid, cannot draw value lattice");
        }

        self.ticks.clear();

        let visible_range = visible_tick_range(val_range, display_scale);
        let range_len = visible_range.len();
        if range_len == 0.0 {
            return Ok(());
        }

        // Use a "nice" decimal major step so the ruler stays readable while zooming.
        // Everything else derives from that single choice: mid ticks split it in half,
        // minor ticks split it into tenths, and label precision follows the same step.
        let min_major_step = range_len / max_nr_ticks as f64;
        let major_step = nice_step(min_major_step);
        let mid_step = major_step / 2.0;
        let minor_step = major_step / 10.0;
        self.major_step = major_step;

        // Snap the visible range to the minor grid first so iteration is stable and we do not
        // accumulate floating-point drift as the user pans around non-zero regions.
        let start_value = ceil_to_multiple_f64(visible_range.min, minor_step);
        let end_value = floor_to_multiple_f64(visible_range.max, minor_step);
        if start_value > end_value {
            return Ok(());
        }

        let mut step_ix = 0_u64;
        let max_steps = (((end_value - start_value) / minor_step).round() as u64).saturating_add(1);
        while step_ix <= max_steps {
            let value = quantize_to_step(start_value + step_ix as f64 * minor_step, minor_step);
            if value > visible_range.max + minor_step * 0.5 {
                break;
            }
            let Some(screen_y) =
                sample_value_to_screen_y(value, val_range, screen_rect, display_scale)
            else {
                step_ix += 1;
                continue;
            };
            if !screen_rect.contains_y(screen_y) {
                step_ix += 1;
                continue;
            }

            let tick_type = if is_multiple_of(value, major_step) {
                TickType::Big
            } else if is_multiple_of(value, mid_step) {
                TickType::Mid
            } else {
                TickType::Small
            };
            self.ticks.push(ValueTick {
                sample_value: value,
                screen_y,
                tick_type,
            });
            step_ix += 1;
        }

        Ok(())
    }
}

fn visible_tick_range(
    val_range: sample::ValRange<f64>,
    display_scale: ValueDisplayScale,
) -> sample::ValRange<f64> {
    if display_scale.skew_factor == 0.0 {
        return val_range;
    }

    sample::ValRange {
        min: val_range.min.max(-1.0),
        max: val_range.max.min(1.0),
    }
}

fn normalize_zero(value: f64) -> f64 {
    if value.abs() < 1e-9 { 0.0 } else { value }
}

fn quantize_to_step(value: f64, step: f64) -> f64 {
    if step <= 0.0 {
        return normalize_zero(value);
    }
    // Re-quantize every generated value back onto the chosen step so labels such as 0.1 do not
    // turn into 0.099999999 due to floating-point math.
    normalize_zero((value / step).round() * step)
}

fn is_multiple_of(value: f64, step: f64) -> bool {
    if step <= 0.0 {
        return false;
    }
    let quotient = value / step;
    (quotient - quotient.round()).abs() < 1e-6
}

fn nice_step(min_step: f64) -> f64 {
    if min_step <= 0.0 {
        return 1.0;
    }

    // Standard 1/2/5 progression gives predictable decimal labels across wide zoom ranges.
    let exponent = min_step.log10().floor();
    let base = 10.0_f64.powf(exponent);
    for factor in [1.0, 2.0, 5.0, 10.0] {
        let candidate = factor * base;
        if candidate >= min_step {
            return candidate;
        }
    }
    10.0 * base
}

fn ceil_to_multiple_f64(x: f64, step: f64) -> f64 {
    if step <= 0.0 {
        return x;
    }
    let scaled = (x / step).ceil();
    normalize_zero(scaled * step)
}

fn floor_to_multiple_f64(x: f64, step: f64) -> f64 {
    if step <= 0.0 {
        return x;
    }
    let scaled = floor_to_multiple((x / step).floor() as i64, 1) as f64;
    normalize_zero(scaled * step)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_scale_range_includes_zero_and_major_ticks() {
        let mut lattice = ValueLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 60.0, 220.0);
        lattice
            .compute_ticks(
                sample::ValRange {
                    min: -1.0,
                    max: 1.0,
                },
                screen_rect,
                50.0,
                ValueDisplayScale::default(),
            )
            .unwrap();

        let major_values: Vec<_> = lattice
            .ticks
            .iter()
            .filter(|tick| tick.tick_type == TickType::Big)
            .map(|tick| tick.sample_value)
            .collect();

        assert_eq!(major_values, vec![-1.0, -0.5, 0.0, 0.5, 1.0]);
        assert_eq!(lattice.major_step, 0.5);
        assert!(
            lattice
                .ticks
                .iter()
                .any(|tick| tick.sample_value == 0.0 && tick.tick_type == TickType::Big)
        );
    }

    #[test]
    fn zoomed_range_uses_decimal_subdivisions() {
        let mut lattice = ValueLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 60.0, 240.0);
        lattice
            .compute_ticks(
                sample::ValRange {
                    min: -0.2,
                    max: 0.2,
                },
                screen_rect,
                50.0,
                ValueDisplayScale::default(),
            )
            .unwrap();

        let major_values: Vec<_> = lattice
            .ticks
            .iter()
            .filter(|tick| tick.tick_type == TickType::Big)
            .map(|tick| tick.sample_value)
            .collect();

        assert_eq!(major_values, vec![-0.2, -0.1, 0.0, 0.1, 0.2]);
        assert_eq!(lattice.major_step, 0.1);
        assert!(
            lattice
                .ticks
                .iter()
                .all(|tick| tick.sample_value >= -0.2 && tick.sample_value <= 0.2)
        );
    }

    #[test]
    fn shifted_range_maps_ticks_inside_screen_rect() {
        let mut lattice = ValueLattice::default();
        let screen_rect = rect::Rect::new(10.0, 20.0, 70.0, 220.0);
        lattice
            .compute_ticks(
                sample::ValRange {
                    min: 0.25,
                    max: 0.75,
                },
                screen_rect,
                50.0,
                ValueDisplayScale::default(),
            )
            .unwrap();

        assert!(
            lattice
                .ticks
                .iter()
                .all(|tick| tick.sample_value >= 0.25 && tick.sample_value <= 0.75)
        );
        assert!(lattice.ticks.iter().all(
            |tick| tick.screen_y >= screen_rect.top() && tick.screen_y <= screen_rect.bottom()
        ));
    }

    #[test]
    fn skewed_ticks_remain_monotonic() {
        let mut lattice = ValueLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 60.0, 220.0);
        lattice
            .compute_ticks(
                sample::ValRange {
                    min: -1.0,
                    max: 1.0,
                },
                screen_rect,
                50.0,
                ValueDisplayScale { skew_factor: 1.0 },
            )
            .unwrap();

        assert!(
            lattice
                .ticks
                .windows(2)
                .all(|pair| pair[0].screen_y >= pair[1].screen_y)
        );
    }

    #[test]
    fn linear_ticks_can_extend_outside_full_scale() {
        let mut lattice = ValueLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 60.0, 220.0);
        lattice
            .compute_ticks(
                sample::ValRange { min: 1.1, max: 1.9 },
                screen_rect,
                50.0,
                ValueDisplayScale::default(),
            )
            .unwrap();

        assert!(!lattice.ticks.is_empty());
        assert!(lattice.ticks.iter().all(|tick| tick.sample_value > 1.0));
    }

    #[test]
    fn skewed_ticks_clip_to_full_scale() {
        let mut lattice = ValueLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 60.0, 220.0);
        lattice
            .compute_ticks(
                sample::ValRange { min: 0.5, max: 1.5 },
                screen_rect,
                50.0,
                ValueDisplayScale { skew_factor: 1.0 },
            )
            .unwrap();

        assert!(!lattice.ticks.is_empty());
        assert!(lattice.ticks.iter().all(|tick| tick.sample_value <= 1.0));
    }

    #[test]
    fn skewed_ticks_are_hidden_when_visible_range_is_outside_full_scale() {
        let mut lattice = ValueLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 60.0, 220.0);
        lattice
            .compute_ticks(
                sample::ValRange { min: 1.1, max: 1.9 },
                screen_rect,
                50.0,
                ValueDisplayScale { skew_factor: 1.0 },
            )
            .unwrap();

        assert!(lattice.ticks.is_empty());
    }
}
