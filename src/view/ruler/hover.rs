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
            .ruler
            .screen_x_to_sample_ix(pos_in_rect.x)
            .unwrap_or(0.0);
        let sample_pos_x = model
            .tracks
            .ruler
            .sample_ix_to_screen_x(sample_ix.round())
            .map(|x| x.floor() as f64);
        let hover_info = HoverInfoE::IsHovered(HoverInfo {
            screen_pos: pos_in_rect.into(),
            sample_ix,
            sample_pos_x,
        });
        model.actions.push(Action::SetHoverInfo(hover_info));
    }

    let mut hover_text_rect = None;
    if let HoverInfoE::IsHovered(hover_info) = &model.tracks.hover_info {
        hover_text_rect = ui_hover_tick_label(ui, hover_info);
        ui_hover_tick_line_triangle(ui, hover_info);
    }
    Ok(hover_text_rect)
}

fn ui_hover_tick_label(ui: &mut egui::Ui, hover_info: &HoverInfo) -> Option<egui::Rect> {
    let sample_ix = hover_info.sample_ix.round() as i64;
    ticks::ui_tick_label(
        ui,
        hover_info.screen_pos.x,
        TickLabel::Text(sample_ix.separate_with_commas()),
        None,
        true,
    )
}

fn ui_hover_tick_line_triangle(ui: &mut egui::Ui, hover_info: &HoverInfo) {
    let screen_x = hover_info.screen_pos.x;
    let rect_x_range = ui.min_rect().left()..ui.min_rect().right();
    if !rect_x_range.contains(&screen_x) {
        tracing::trace!("screen_x {} not in rect {:?}", screen_x, rect_x_range);
        return;
    }
    let color = egui::Color32::LIGHT_BLUE;
    ticks::ui_tick_line(ui, screen_x, ticks::TICK_HEIGHT_LONG - 2.0, Some(color));
    ticks::ui_triangle(ui, screen_x, TriangleType::Full);
}
