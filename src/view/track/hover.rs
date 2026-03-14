use crate::{
    model::{
        Action, Model,
        hover_info::{HoverInfo, HoverInfoE},
        track::TrackId,
    },
    view::util::rpc,
};

pub fn ui_hover(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId) {
    match &model.tracks.hover_info {
        HoverInfoE::NotHovered => {}
        HoverInfoE::IsHovered(hover_info) => {
            {
                let pos_y_min = ui.min_rect().top();
                let pos_y_max = ui.min_rect().bottom();
                let pos_x = hover_info.screen_pos.x;
                let pos_min = rpc(ui, egui::pos2(pos_x, pos_y_min));
                let pos_max = rpc(ui, egui::pos2(pos_x, pos_y_max));
                ui.painter().line_segment(
                    [pos_min, pos_max],
                    egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE),
                );
            }

            {
                let rect = ui.min_rect();
                if rect.contains((&hover_info.screen_pos).into()) {
                    let pos_x_min = rect.left();
                    let pos_x_max = rect.right();
                    let pos_y = hover_info.screen_pos.y;
                    let pos_min = rpc(ui, egui::pos2(pos_x_min, pos_y));
                    let pos_max = rpc(ui, egui::pos2(pos_x_max, pos_y));
                    ui.painter().line_segment(
                        [pos_min, pos_max],
                        egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE),
                    );
                }
            }
        }
    }

    let rect = ui.min_rect();
    let _hover_response = ui
        .interact(rect, egui::Id::new(track_id), egui::Sense::hover())
        .on_hover_cursor(egui::CursorIcon::None);

    if let Some(pos) = ui.ctx().pointer_hover_pos()
        && rect.contains(pos)
    {
        let sample_ix = model
            .tracks
            .ruler
            .screen_x_to_sample_ix(pos.x)
            .unwrap_or(0.0);
        let sample_pos_x = model
            .tracks
            .ruler
            .sample_ix_to_screen_x(sample_ix.round())
            .map(|x| x.floor() as f64);
        model
            .actions
            .push(Action::SetHoverInfo(HoverInfoE::IsHovered(HoverInfo {
                screen_pos: pos.into(),
                sample_ix,
                sample_pos_x,
            })));
        ui.ctx().input(|i| {
            let scroll = i.raw_scroll_delta;
            let scroll_y = if scroll.y != 0.0 { scroll.y } else { scroll.x };
            if i.modifiers.alt {
                if i.modifiers.shift && !i.modifiers.ctrl && scroll_y != 0.0 {
                    model.actions.push(Action::PanY {
                        track_id,
                        nr_pixels: scroll_y,
                    });
                } else if i.modifiers.ctrl && scroll_y != 0.0 {
                    let zoom_x_factor = model.user_config.zoom_x_scroll_factor;
                    model.actions.push(Action::ZoomY {
                        track_id,
                        nr_pixels: scroll_y * zoom_x_factor,
                        center_y: pos.y,
                    });
                }
            } else if i.modifiers.shift && !i.modifiers.ctrl {
                if scroll.x != 0.0 {
                    model.actions.push(Action::PanX {
                        nr_pixels: scroll.x,
                    });
                }
            } else if i.modifiers.ctrl && scroll.y != 0.0 {
                let zoom_x_factor = model.user_config.zoom_x_scroll_factor;
                model.actions.push(Action::ZoomX {
                    nr_pixels: scroll.y * zoom_x_factor,
                    center_x: pos.x,
                });
            }
        });
    }
}
