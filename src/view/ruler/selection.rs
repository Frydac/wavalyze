use crate::model::{self, selection_info::SelectionInfoE};
use anyhow::Result;
use thousands::Separable;

use super::ticks::{self, TickLabel, TriangleType};

pub fn ui_selection_interaction_and_tics(
    ui: &mut egui::Ui,
    model: &mut model::Model,
    existing_rects: &mut Vec<egui::Rect>,
) -> Result<Vec<egui::Rect>> {
    // Bail out early when there is no active selection to annotate in the ruler.
    let selection_ix_range =
        if let SelectionInfoE::IsSelected(selection_info) = &model.tracks.selection_info {
            selection_info.ix_rng
        } else {
            return Ok(Vec::new());
        };

    let mut result = Vec::new();

    // For each visible selection edge, draw its tick/triangle immediately and keep the
    // corresponding label payload around for the placement pass below.
    let rect = ui.min_rect();
    let left_ix = selection_ix_range.start;
    let left_label = model
        .tracks
        .sample_ix_to_screen_x(left_ix as f64)
        .filter(|&left_x| left_x > rect.min.x && left_x < rect.max.x)
        .map(|left_x| {
            ticks::ui_tick_line(ui, left_x, ticks::TICK_HEIGHT_LONG, None);
            ticks::ui_triangle(ui, left_x, TriangleType::Left);
            (left_x, TickLabel::Text(left_ix.separate_with_commas()))
        });

    let right_ix = selection_ix_range.end - 1;
    let right_label = model
        .tracks
        .sample_ix_to_screen_x(right_ix as f64)
        .filter(|&right_x| right_x > rect.min.x && right_x < rect.max.x)
        .map(|right_x| {
            ticks::ui_tick_line(ui, right_x, ticks::TICK_HEIGHT_LONG, None);
            ticks::ui_triangle(ui, right_x - 1.0, TriangleType::Right);
            (right_x, TickLabel::Text(right_ix.separate_with_commas()))
        });

    // If both labels are visible, try the paired placement logic first so they can resolve
    // against each other. Otherwise fall back to the ordinary single-label placement path.
    match (left_label, right_label) {
        (Some((left_x, left_text)), Some((right_x, right_text))) => {
            if let Some(pair_rects) = ticks::ui_selection_tick_label_pair(
                ui,
                (left_x, left_text),
                (right_x, right_text),
                existing_rects.as_slice(),
            ) {
                result.extend(pair_rects);
            } else if let Some(rect) = ticks::ui_tick_label(
                ui,
                left_x,
                TickLabel::SampleIx(left_ix),
                Some(existing_rects.as_slice()),
                true,
            ) {
                result.push(rect);
            }
        }
        (Some((left_x, _)), None) => {
            if let Some(rect) = ticks::ui_tick_label(
                ui,
                left_x,
                TickLabel::SampleIx(left_ix),
                Some(existing_rects.as_slice()),
                true,
            ) {
                result.push(rect);
            }
        }
        (None, Some((right_x, _))) => {
            if let Some(rect) = ticks::ui_tick_label(
                ui,
                right_x,
                TickLabel::SampleIx(right_ix),
                Some(existing_rects.as_slice()),
                true,
            ) {
                result.push(rect);
            }
        }
        (None, None) => {}
    }

    Ok(result)
}
