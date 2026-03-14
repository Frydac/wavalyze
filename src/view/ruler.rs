use crate::model::{
    self, Action,
    hover_info::HoverInfoE,
    ruler::{self},
};
use anyhow::Result;
use thousands::Separable;

mod hover;
mod selection;
mod ticks;

pub use ticks::NR_PIXELS_PER_TICK;

pub fn ui(ui: &mut egui::Ui, model: &mut model::Model) -> Result<()> {
    let container_rect = ui.min_rect();
    let info_width = model
        .user_config
        .tracks_width_info
        .min(container_rect.width());
    let mut info_rect = container_rect;
    info_rect.set_width(info_width);
    let ruler_rect = egui::Rect::from_min_size(
        [info_rect.max.x, container_rect.min.y].into(),
        [container_rect.width() - info_width, container_rect.height()].into(),
    );
    // println!("ruler_rect first: {}", ruler_rect);

    // debug_rect_text(ui, rect.shrink(1.0), egui::Color32::LIGHT_GREEN, "ruler container");
    // debug_rect_text(ui, info_rect.shrink(1.0), egui::Color32::LIGHT_GRAY, "ruler info");
    // debug_rect_text(ui, ruler_rect.shrink(1.0), egui::Color32::LIGHT_BLUE, "ruler");

    let response = ui.allocate_rect(ruler_rect, egui::Sense::click_and_drag());
    let mut ui_ruler = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(ruler_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
    ui_ruler
        .painter()
        .rect(ruler_rect, 3.0, egui::Color32::TRANSPARENT, stroke);
    ui_ruler.set_min_size(ruler_rect.size());

    // Update the screen rect of the ruler
    model.tracks.ruler.set_screen_rect(ruler_rect.into());

    // Do interactions
    handle_drag_interaction(&mut ui_ruler, &response, &mut model.actions);
    handle_scroll_interaction(
        &mut ui_ruler,
        &mut model.actions,
        model.user_config.zoom_x_scroll_factor,
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
    ticks::ui_ix_lattice(
        &mut ui_ruler,
        &mut model.tracks.ruler,
        &mut existing_tick_label_rects,
    );

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

pub fn handle_scroll_interaction(ui: &mut egui::Ui, actions: &mut Vec<Action>, zoom_x_factor: f32) {
    let rect = ui.min_rect();
    let pos_in_rect = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|&pos| rect.contains(pos));
    if let Some(pos) = pos_in_rect {
        ui.ctx().input(|i| {
            if i.modifiers.shift && !i.modifiers.ctrl {
                let scroll = i.raw_scroll_delta;
                if scroll.x != 0.0 {
                    actions.push(model::action::Action::PanX {
                        nr_pixels: scroll.x,
                    });
                }
            } else if i.modifiers.ctrl && !i.modifiers.shift {
                let scroll = i.raw_scroll_delta;
                if scroll.y != 0.0 {
                    actions.push(model::action::Action::ZoomX {
                        nr_pixels: scroll.y * zoom_x_factor,
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

pub fn ui_ruler_info_panel(ui: &mut egui::Ui, ruler: &ruler::Time) {
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
            if let Some(time_line) = ruler.time_line.as_ref() {
                grid.row(
                    "samples per pixel:",
                    format!("{:.3}", time_line.samples_per_pixel()),
                );
                let ix_range = time_line.get_ix_range(ruler.screen_rect().width() as f64);
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
