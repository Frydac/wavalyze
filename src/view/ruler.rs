use crate::model::{self, Action, hover_info::HoverInfoE};
use anyhow::Result;
use thousands::Separable;

use crate::view::util::zoom_delta_to_scroll_delta;

mod hover;
mod overview;
mod selection;
mod ticks;

pub use ticks::NR_PIXELS_PER_TICK;

const TIME_RULER_HEIGHT: f32 = 50.0;
pub(crate) const HEIGHT: f32 = overview::HEIGHT + TIME_RULER_HEIGHT;

fn block_coordinates(ruler_sample: i64, block_size: u64) -> (i128, i128) {
    let ruler_sample = i128::from(ruler_sample);
    let block_size = i128::from(block_size.max(1));
    (
        ruler_sample.div_euclid(block_size),
        ruler_sample.rem_euclid(block_size),
    )
}

fn format_sample_block_label(sample_ix: i64, block_size: u64) -> String {
    let (block_ix, in_block_offset) = block_coordinates(sample_ix, block_size);
    format!(
        "s: {}\nb: {block_ix} + {in_block_offset}",
        sample_ix.separate_with_commas()
    )
}

pub fn ui(ui: &mut egui::Ui, model: &mut model::Model, content_rect: egui::Rect) -> Result<()> {
    // The ruler block is split vertically: an overview strip above the interactive time ruler.
    // Overview edge-resizing depends on both rectangles sharing the waveform coordinate space.
    let overview_rect = egui::Rect::from_min_size(
        content_rect.min,
        egui::vec2(
            content_rect.width(),
            overview::HEIGHT.min(content_rect.height()),
        ),
    );
    let ruler_rect = egui::Rect::from_min_max(
        egui::pos2(content_rect.left(), overview_rect.bottom()),
        content_rect.right_bottom(),
    );

    // Overview interactions push the same navigation actions as the ruler/waveform views.
    overview::ui(ui, model, overview_rect, ruler_rect);

    let response = ui.allocate_rect(ruler_rect, egui::Sense::click_and_drag());
    let mut ui_ruler = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(ruler_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
    ui_ruler.painter().rect(
        ruler_rect,
        3.0,
        egui::Color32::TRANSPARENT,
        stroke,
        egui::epaint::StrokeKind::Inside,
    );
    ui_ruler.set_min_size(ruler_rect.size());

    // Update the screen rect of the ruler
    model.tracks.ruler.set_screen_rect(ruler_rect.into());

    // Do interactions
    handle_drag_interaction(&mut ui_ruler, &response, &mut model.actions);
    handle_scroll_interaction(
        &mut ui_ruler,
        &mut model.actions,
        &model.user_config.navigation,
    );

    // Draw stuff
    //
    // We get the hover text rect so we can avoid it when drawing ix lattice labels
    let hover_tick_label_rect = hover::ui_hover_interaction_and_tick(&mut ui_ruler, model)?;
    let mut existing_tick_label_rects = hover_tick_label_rect.into_iter().collect::<Vec<_>>();
    //
    // Draw sample index selection
    let selection_tick_label_rects = selection::ui_selection_interaction_and_tics(
        &mut ui_ruler,
        model,
        &mut existing_tick_label_rects,
    )?;
    existing_tick_label_rects.extend(selection_tick_label_rects);
    //
    if let Some(ix_range) = model.tracks.ix_range() {
        ticks::ui_ix_lattice(
            &mut ui_ruler,
            &mut model.tracks.ruler,
            ix_range,
            &mut existing_tick_label_rects,
        );
    }

    Ok(())
}

pub fn handle_drag_interaction(
    ui: &mut egui::Ui,
    response: &egui::Response,
    actions: &mut Vec<Action>,
) {
    if response.dragged() {
        let delta = ui.input(|i| i.pointer.delta());
        actions.push(model::action::Action::PanX {
            nr_pixels: -delta.x,
        });
    }
}

pub fn handle_scroll_interaction(
    ui: &mut egui::Ui,
    actions: &mut Vec<Action>,
    navigation: &model::config::NavigationConfig,
) {
    let rect = ui.min_rect();
    let pos_in_rect = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|&pos| rect.contains(pos));
    if let Some(pos) = pos_in_rect {
        let scroll_zoom_speed = ui.ctx().options(|o| o.input_options.scroll_zoom_speed);
        ui.ctx().input(|i| {
            if i.modifiers.shift && !i.modifiers.ctrl {
                let scroll = i.smooth_scroll_delta;
                if scroll.x != 0.0 {
                    actions.push(model::action::Action::PanX {
                        nr_pixels: scroll.x * navigation.pan_x_mult(),
                    });
                }
            } else if i.modifiers.ctrl && !i.modifiers.shift {
                let zoom_scroll_delta =
                    zoom_delta_to_scroll_delta(i.zoom_delta(), scroll_zoom_speed);
                if zoom_scroll_delta != 0.0 {
                    actions.push(model::action::Action::ZoomX {
                        nr_pixels: zoom_scroll_delta * navigation.zoom_x_mult(),
                        center_x: pos.x,
                    });
                }
            }
        });
    }
}

////////////////////////////////////////////////////////////////////////////////
// InfoPanel
////////////////////////////////////////////////////////////////////////////////

pub fn ui_ruler_info_panel(ui: &mut egui::Ui, tracks: &model::tracks::Tracks) {
    let ruler = &tracks.ruler;
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.heading("Ruler Info");
            ui.separator();

            let mut grid = crate::view::grid::KeyValueGrid::new(12345);
            {
                let rect = ruler.screen_rect();
                grid.row(
                    "screen rect:",
                    format!(
                        "[{:.1}, {:.1}, {:.1}, {:.1}]",
                        rect.min.x, rect.min.y, rect.max.x, rect.max.y
                    ),
                );
            }
            grid.row(
                "seconds per pixel:",
                format!("{:.6}", tracks.time_camera.seconds_per_pixel()),
            );
            if let Some(spp) = tracks.samples_per_pixel() {
                grid.row("samples per pixel:", format!("{spp:.3}"));
            }
            if let Some(ix_range) = tracks.ix_range() {
                let ix_range_start = format!("{:.1}", ix_range.start).separate_with_commas();
                let ix_range_end = format!("{:.1}", ix_range.end).separate_with_commas();
                grid.row("ix range:", format!("[{ix_range_start}, {ix_range_end}]"));
            }
            grid.show(ui);
        });
    });
}

pub fn ui_hover_info_panel2(ui: &mut egui::Ui, hover_info: &HoverInfoE) {
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.heading("Hover Info");
            ui.separator();
            match hover_info {
                HoverInfoE::NotHovered => {
                    ui.label("No hover info");
                }
                HoverInfoE::IsHovered(hover_info) => {
                    let id: u64 = ui.id().with("hover_info_panel2").value();
                    let mut grid = crate::view::grid::KeyValueGrid::new(id);
                    grid.row(
                        "pos x:",
                        format!("{:.1}", hover_info.screen_pos.x).separate_with_commas(),
                    );
                    grid.row(
                        "sample ix:",
                        (hover_info.sample_ix.round() as i64).separate_with_commas(),
                    );
                    grid.show(ui);
                }
            }
        });
    });
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
