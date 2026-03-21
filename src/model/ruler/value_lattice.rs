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
    /// Long tick spacing used by the ruler and waveform grid.
    pub major_step: f64,
    /// Optional midpoint cadence between adjacent major ticks.
    pub mid_step: Option<f64>,
    /// Finest tick spacing currently shown on the ruler.
    pub minor_step: f64,
    /// Values that should receive text labels on the ruler.
    pub label_step: f64,
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

        let min_label_step = range_len / max_nr_ticks as f64;
        // The cadence is the single source of truth for how dense the ruler should look at the
        // current zoom level. Views consume the emitted tick types and label spacing directly
        // instead of reconstructing their own hierarchy.
        let cadence = lattice_steps(min_label_step);
        self.major_step = cadence.major_step;
        self.mid_step = cadence.mid_step;
        self.minor_step = cadence.minor_step;
        self.label_step = cadence.label_step;

        // Snap the visible range to the active minor grid first so iteration is stable and we do
        // not accumulate floating-point drift as the user pans around non-zero regions.
        let start_value = ceil_to_multiple_f64(visible_range.min, cadence.minor_step);
        let end_value = floor_to_multiple_f64(visible_range.max, cadence.minor_step);
        if start_value > end_value {
            return Ok(());
        }

        let mut step_ix = 0_u64;
        let max_steps =
            (((end_value - start_value) / cadence.minor_step).round() as u64).saturating_add(1);
        while step_ix <= max_steps {
            let value = quantize_to_step(
                start_value + step_ix as f64 * cadence.minor_step,
                cadence.minor_step,
            );
            if value > visible_range.max + cadence.minor_step * 0.5 {
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

            self.ticks.push(ValueTick {
                sample_value: value,
                screen_y,
                tick_type: classify_tick_type(value, cadence),
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

fn classify_tick_type(value: f64, cadence: ValueTickCadence) -> TickType {
    if is_multiple_of(value, cadence.major_step) {
        TickType::Big
    } else if cadence
        .mid_step
        .is_some_and(|mid_step| is_multiple_of(value, mid_step))
    {
        TickType::Mid
    } else {
        TickType::Small
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ValueTickCadence {
    major_step: f64,
    mid_step: Option<f64>,
    minor_step: f64,
    label_step: f64,
}

fn lattice_steps(min_label_step: f64) -> ValueTickCadence {
    let label_step = nice_step(min_label_step);
    let (major_step, mid_step, minor_step) = if is_power_of_ten_step(label_step) && label_step < 1.0
    {
        // Decimal label states such as `0.1, 0.2, ...` read best with `0.05` mid ticks and
        // `0.01` small ticks instead of awkward `0.02` spacing.
        (label_step, Some(label_step / 2.0), label_step / 10.0)
    } else if is_five_step(label_step) && label_step < 0.1 {
        // When labels themselves land on `...0.05`, keep them as midpoint labels under the
        // surrounding `0.1` majors.
        (label_step * 2.0, Some(label_step), label_step / 5.0)
    } else {
        // Coarser states such as `0.5` or `1.0` do not need a dedicated midpoint hierarchy.
        (label_step, None, label_step / 5.0)
    };
    ValueTickCadence {
        major_step,
        mid_step,
        minor_step,
        label_step,
    }
}

fn is_five_step(step: f64) -> bool {
    if step <= 0.0 {
        return false;
    }
    let exponent = step.log10().floor();
    let base = 10.0_f64.powf(exponent);
    ((step / base) - 5.0).abs() < 1e-6
}

fn is_power_of_ten_step(step: f64) -> bool {
    if step <= 0.0 {
        return false;
    }
    let exponent = step.log10().floor();
    let base = 10.0_f64.powf(exponent);
    ((step / base) - 1.0).abs() < 1e-6
}

fn nice_step(min_step: f64) -> f64 {
    if min_step <= 0.0 {
        return 1.0;
    }

    // A 1/5/10 progression avoids the intermediate 2x states that make the value ruler
    // jump through extra major-tick layouts while resizing track height.
    let exponent = min_step.log10().floor();
    let base = 10.0_f64.powf(exponent);
    for factor in [1.0, 5.0, 10.0] {
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
        assert_eq!(lattice.mid_step, None);
        assert_eq!(lattice.minor_step, 0.1);
        assert_eq!(lattice.label_step, 0.5);
        assert!(
            lattice
                .ticks
                .iter()
                .any(|tick| tick.sample_value == 0.0 && tick.tick_type == TickType::Big)
        );
        let between_neg_one_and_neg_half = lattice
            .ticks
            .iter()
            .filter(|tick| {
                tick.tick_type == TickType::Small
                    && tick.sample_value > -1.0
                    && tick.sample_value < -0.5
            })
            .count();
        assert_eq!(between_neg_one_and_neg_half, 4);
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
        assert_eq!(lattice.mid_step, Some(0.05));
        assert_eq!(lattice.minor_step, 0.01);
        assert_eq!(lattice.label_step, 0.1);
        assert!(
            lattice
                .ticks
                .iter()
                .all(|tick| tick.sample_value >= -0.2 && tick.sample_value <= 0.2)
        );
        assert!(
            lattice
                .ticks
                .iter()
                .any(|tick| tick.sample_value == -0.19 && tick.tick_type == TickType::Small)
        );
        assert!(
            lattice
                .ticks
                .iter()
                .any(|tick| tick.sample_value == -0.15 && tick.tick_type == TickType::Mid)
        );
        let between_neg_point_two_and_neg_point_one = lattice
            .ticks
            .iter()
            .filter(|tick| {
                tick.tick_type == TickType::Small
                    && tick.sample_value > -0.2
                    && tick.sample_value < -0.1
            })
            .count();
        assert_eq!(between_neg_point_two_and_neg_point_one, 8);
    }

    #[test]
    fn lattice_steps_keep_minor_spacing_in_one_place() {
        assert_eq!(
            lattice_steps(1.0),
            ValueTickCadence {
                major_step: 1.0,
                mid_step: None,
                minor_step: 0.2,
                label_step: 1.0,
            }
        );
        assert_eq!(
            lattice_steps(0.5),
            ValueTickCadence {
                major_step: 0.5,
                mid_step: None,
                minor_step: 0.1,
                label_step: 0.5,
            }
        );
        assert_eq!(
            lattice_steps(0.1),
            ValueTickCadence {
                major_step: 0.1,
                mid_step: Some(0.05),
                minor_step: 0.01,
                label_step: 0.1,
            }
        );
        assert_eq!(
            lattice_steps(0.05),
            ValueTickCadence {
                major_step: 0.1,
                mid_step: Some(0.05),
                minor_step: 0.01,
                label_step: 0.05,
            }
        );
        assert_eq!(
            lattice_steps(0.01),
            ValueTickCadence {
                major_step: 0.01,
                mid_step: Some(0.005),
                minor_step: 0.001,
                label_step: 0.01,
            }
        );
    }

    #[test]
    fn nice_step_skips_two_x_progression() {
        assert_eq!(nice_step(1.0), 1.0);
        assert_eq!(nice_step(0.6), 1.0);
        assert_eq!(nice_step(0.5), 0.5);
        assert_eq!(nice_step(0.2), 0.5);
        assert_eq!(nice_step(0.11), 0.5);
        assert_eq!(nice_step(0.1), 0.1);
        assert_eq!(nice_step(0.06), 0.1);
        assert_eq!(nice_step(0.05), 0.05);
        assert_eq!(nice_step(0.02), 0.05);
        assert_eq!(nice_step(0.01), 0.01);
    }

    #[test]
    fn five_hundredth_labels_use_mid_ticks() {
        let mut lattice = ValueLattice::default();
        let screen_rect = rect::Rect::new(0.0, 0.0, 60.0, 240.0);
        lattice
            .compute_ticks(
                sample::ValRange {
                    min: -0.12,
                    max: 0.12,
                },
                screen_rect,
                50.0,
                ValueDisplayScale::default(),
            )
            .unwrap();

        assert_eq!(lattice.major_step, 0.1);
        assert_eq!(lattice.mid_step, Some(0.05));
        assert_eq!(lattice.minor_step, 0.01);
        assert_eq!(lattice.label_step, 0.05);
        assert!(
            lattice
                .ticks
                .iter()
                .any(|tick| tick.sample_value == 0.0 && tick.tick_type == TickType::Big)
        );
        assert!(
            lattice
                .ticks
                .iter()
                .any(|tick| tick.sample_value == 0.05 && tick.tick_type == TickType::Mid)
        );
        assert!(
            lattice
                .ticks
                .iter()
                .any(|tick| tick.sample_value == 0.1 && tick.tick_type == TickType::Big)
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
