use crate::{audio::sample::FracIxRange, model::Model, view::util::rpc};

fn grid_step(block_size: u64, samples_per_pixel: f64, min_spacing: f32) -> Option<i128> {
    if !samples_per_pixel.is_finite() || samples_per_pixel <= 0.0 {
        return None;
    }
    let spacing = if min_spacing.is_finite() {
        min_spacing.clamp(1.0, 1000.0)
    } else {
        25.0
    };
    let mut step = i128::from(block_size.max(1));
    while step as f64 / samples_per_pixel < f64::from(spacing) {
        step = step.checked_mul(10)?;
    }
    Some(step)
}

fn boundaries(range: FracIxRange, step: i128) -> Vec<i64> {
    if !range.start.is_finite() || !range.end.is_finite() || range.end <= range.start || step <= 0 {
        return Vec::new();
    }
    let start = range.start.ceil().max(i64::MIN as f64) as i128;
    let end = (range.end.floor() as i128).min(i128::from(i64::MAX));
    let Some(mut sample) = start.div_euclid(step).checked_mul(step) else {
        return Vec::new();
    };
    if sample < start {
        let Some(next) = sample.checked_add(step) else {
            return Vec::new();
        };
        sample = next;
    }
    let mut result = Vec::new();
    while sample <= end {
        if sample != 0
            && let Ok(sample) = i64::try_from(sample)
        {
            result.push(sample);
        }
        let Some(next) = sample.checked_add(step) else {
            break;
        };
        sample = next;
    }
    result
}

pub(super) fn draw(ui: &egui::Ui, model: &Model, rect: egui::Rect) {
    let config = &model.user_config;
    if !config.blocks.enabled
        || !config.blocks.show_grid
        || !rect.is_finite()
        || !rect.is_positive()
    {
        return;
    }
    let (Some(range), Some(spp)) = (model.tracks.ix_range(), model.tracks.samples_per_pixel())
    else {
        return;
    };
    let Some(step) = grid_step(model.block_size, spp, config.blocks.grid_min_spacing_px) else {
        return;
    };
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let stroke = egui::Stroke::new(1.0, ui.visuals().text_color().gamma_multiply(0.14));
    for sample in boundaries(range, step) {
        if let Some(x) = model.tracks.sample_ix_to_screen_x(sample as f64) {
            painter.line_segment(
                [
                    rpc(ui, egui::pos2(x, rect.top())),
                    rpc(ui, egui::pos2(x, rect.bottom())),
                ],
                stroke,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_uses_decades_at_thresholds() {
        assert_eq!(grid_step(1000, 39.99, 25.0), Some(1000));
        assert_eq!(grid_step(1000, 40.0, 25.0), Some(1000));
        assert_eq!(grid_step(1000, 40.01, 25.0), Some(10_000));
        assert_eq!(grid_step(1000, 400.0, 25.0), Some(10_000));
        assert_eq!(grid_step(1000, 400.01, 25.0), Some(100_000));
        assert_eq!(grid_step(1024, 51.2, 25.0), Some(10_240));
        assert_eq!(grid_step(1024, 51.2, 20.0), Some(1024));
    }

    #[test]
    fn grid_is_zero_anchored_and_skips_existing_zero_line() {
        let range = FracIxRange {
            start: -2050.0,
            end: 2050.0,
        };
        assert_eq!(boundaries(range, 1024), vec![-2048, -1024, 1024, 2048]);
        let panned = FracIxRange {
            start: 1024.1,
            end: 3072.0,
        };
        assert_eq!(boundaries(panned, 1024), vec![2048, 3072]);
    }

    #[test]
    fn invalid_spacing_and_geometry_are_handled() {
        assert_eq!(grid_step(1000, 40.0, f32::NAN), Some(1000));
        assert_eq!(grid_step(1000, 40.0, f32::INFINITY), Some(1000));
        assert_eq!(grid_step(1024, 0.0, 25.0), None);
        assert_eq!(grid_step(1024, f64::NAN, 25.0), None);
        assert!(
            boundaries(
                FracIxRange {
                    start: 0.0,
                    end: f64::INFINITY
                },
                1024
            )
            .is_empty()
        );
        assert!(
            boundaries(
                FracIxRange {
                    start: 10.0,
                    end: 0.0
                },
                1024
            )
            .is_empty()
        );
        assert!(grid_step(u64::MAX, f64::MAX, 25.0).is_none());
    }
}
