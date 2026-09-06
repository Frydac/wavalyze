use crate::model::{
    self, Action,
    hover_info::{HoverInfo, HoverInfoE},
};
use anyhow::Result;
use thousands::Separable;

use super::ticks::{self, TickLabel, TriangleType};

pub(crate) fn ui_hover_interaction_and_tick(
    ui: &mut egui::Ui,
    model: &mut model::Model,
) -> Result<Option<egui::Rect>> {
    let rect = ui.min_rect();
    if let Some(pos_in_rect) = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|&pos| rect.contains(pos))
    {
        let sample_ix = model
            .tracks
            .screen_x_to_sample_ix(pos_in_rect.x)
            .unwrap_or(0.0);
        let sample_pos_x = model
            .tracks
            .sample_ix_to_screen_x(sample_ix.round())
            .map(|x| x.floor() as f64);
        let hover_info = HoverInfoE::IsHovered(HoverInfo {
            screen_pos: pos_in_rect.into(),
            sample_ix,
            sample_pos_x,
            track_id: None,
        });
        model.actions.push(Action::SetHoverInfo(hover_info));
    }

    let mut hover_text_rect = None;
    if let HoverInfoE::IsHovered(hover_info) = &model.tracks.hover_info {
        let accent = model.user_config.active_theme_colors(ui.visuals()).accent;
        hover_text_rect = ui_hover_tick_label(ui, model, hover_info);
        ui_hover_tick_line_triangle(ui, hover_info, accent);
    }
    Ok(hover_text_rect)
}

fn ui_hover_tick_label(
    ui: &mut egui::Ui,
    model: &model::Model,
    hover_info: &HoverInfo,
) -> Option<egui::Rect> {
    let sample_ix = hover_info.sample_ix.round() as i64;
    let (block_ix, in_block_offset) = block_coordinates(sample_ix, model.block_size);
    ticks::ui_tick_label(
        ui,
        hover_info.screen_pos.x,
        TickLabel::Text(format!(
            "s: {}\nb: {block_ix} + {in_block_offset}",
            sample_ix.separate_with_commas()
        )),
        None,
        true,
    )
}

fn block_coordinates(ruler_sample: i64, block_size: u64) -> (i128, i128) {
    let ruler_sample = i128::from(ruler_sample);
    let block_size = i128::from(block_size.max(1));
    (
        ruler_sample.div_euclid(block_size),
        ruler_sample.rem_euclid(block_size),
    )
}

fn ui_hover_tick_line_triangle(ui: &mut egui::Ui, hover_info: &HoverInfo, color: egui::Color32) {
    let screen_x = hover_info.screen_pos.x;
    let rect_x_range = ui.min_rect().left()..ui.min_rect().right();
    if !rect_x_range.contains(&screen_x) {
        tracing::trace!("screen_x {} not in rect {:?}", screen_x, rect_x_range);
        return;
    }
    ticks::ui_tick_line(ui, screen_x, ticks::TICK_HEIGHT_LONG - 2.0, Some(color));
    ticks::ui_triangle(ui, screen_x, TriangleType::Full, color);
}

#[cfg(test)]
mod tests {
    use super::block_coordinates;

    #[test]
    fn ruler_origin_is_block_zero_sample_zero() {
        // Track placement already subtracts latency: source sample 512 with offset 512 is here.
        assert_eq!(block_coordinates(0, 1024), (0, 0));
    }

    #[test]
    fn sample_before_origin_uses_euclidean_block_coordinates() {
        assert_eq!(block_coordinates(-1, 1024), (-1, 1023));
    }

    #[test]
    fn block_coordinates_advance_from_ruler_origin() {
        assert_eq!(block_coordinates(2048, 1024), (2, 0));
        assert_eq!(block_coordinates(2049, 1024), (2, 1));
    }
}
