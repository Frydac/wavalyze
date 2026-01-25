use crate::{
    audio::sample::view::ViewData,
    model::{hover_info::HoverInfoE, ruler::sample_value_to_screen_y_e, track2::TrackId, Action, Model},
};
use anyhow::Result;

pub fn ui(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId) -> Result<()> {
    // ui.style_mut().spacing.item_spacing = egui::vec2(4.0, 0.0);
    let resp = ui_header(ui, model, track_id)?;
    // ui.painter().rect(resp.rect, 0.0, egui::Color32::RED, egui::Stroke::NONE);
    let resp = ui_waveform_canvas(ui, model, track_id, resp.rect.height())?;
    // ui.painter().rect(resp.rect, 0.0, egui::Color32::GREEN, egui::Stroke::NONE);

    Ok(())
}

pub fn ui_header(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId) -> Result<egui::Response> {
    let resp = egui::Frame::default()
        .stroke(ui.style().visuals.window_stroke())
        // .inner_margin(ui.style().spacing.window_margin / 6.0)
        // .outer_margin(ui.style().spacing.window_margin / 6.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // ui.allocate_space(egui::vec2(ui.available_width(), 0.0));

            ui.horizontal(|ui| {
                let mut text = String::from("track header");
                let mut hover_text = None;

                // TODO: store hover text in model.. *smh*
                if let Some((file, channel)) = model.get_file_channel_for_track(track_id) {
                    let path = file.path.as_ref().and_then(|p| p.to_str()).unwrap_or("unknown");
                    text = format!("{} - ch {}", path, channel.ch_ix);
                    hover_text = Some(format!("{file}"));
                }

                let label_resp = ui.label(text);
                if let Some(hover_text) = hover_text {
                    label_resp.on_hover_text(hover_text);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.style_mut().spacing.window_margin = egui::Margin::same(4.0);
                    if ui.button("x").clicked() {
                        model.actions.push(Action::RemoveTrack(track_id));
                    }
                });
            });

            // let rect = ui.max_rect();
            // ui.painter().line_segment(
            //     [rect.left_top(), rect.right_top()],
            //     ui.visuals().window_stroke(),
            // );
        });

    Ok(resp.response)
}

pub fn ui_hover(ui: &mut egui::Ui, model: &mut Model) {
    // let IsHovered(hover_info): crate::model::hover_info::HoverInfoE = model.tracks2.hover_info else { return };
    match &model.tracks2.hover_info.get() {
        HoverInfoE::NotHovered => {}
        HoverInfoE::IsHovered(hover_info) => {
            // ui_hover_info(ui, model, hover_info);
            let pos_y_min = ui.min_rect().top();
            let pos_y_max = ui.min_rect().bottom();
            let pos_x = hover_info.screen_pos.x;
            let pos_min = rpc(ui, egui::pos2(pos_x, pos_y_min));
            let pos_max = rpc(ui, egui::pos2(pos_x, pos_y_max));
            ui.painter().line_segment([pos_min, pos_max], egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
            // ui.painter().line_segment([[pos_x, pos_y_min].into(), [pos_x, pos_y_max].into()], egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
        }
    }

    // let height = ui.available_height();
    // let width = ui.available_width();

    // ui.painter().rect_filled(egui::Rect::from_min_size(egui::pos2(pos_x, pos_y_min), egui::vec2(width, height)), 0.0, egui::Color32::LIGHT_BLUE);
}

// Wrap the waveform in a (manually implemented) resizable frame
// TODO: see if we can extrac like a resizable canvas or something
pub fn ui_waveform_canvas(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId, header_height: f32) -> Result<egui::Response> {
    const RESIZE_HANDLE_HEIGHT: f32 = 3.0;
    let min_height: f32 = model.user_config.track.min_height;
    let max_height: f32 = ui.available_height().max(min_height); // TODO: find better value
    let width = ui.available_width();
    let height = model.tracks2.get_track_height(track_id).unwrap_or(min_height);
    let height = height - header_height;

    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());

    // -- Waveform canvas --
    let resp = ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            let width = ui.available_width();
            ui.set_min_size(egui::vec2(width, height));
            ui.set_max_size(egui::vec2(width, height));

            let _ = ui_waveform(ui, model, track_id);

            ui_hover(ui, model);
        });
    });

    let track = model
        .tracks2
        .get_track_mut(track_id)
        .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;

    // -- Resize handle --
    // make a rect at the border of the canvas that can be used for resizing height only
    let resize_handle_rect = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.bottom() - RESIZE_HANDLE_HEIGHT),
        egui::vec2(rect.width(), RESIZE_HANDLE_HEIGHT),
    );
    let response = ui.interact(resize_handle_rect, ui.id().with(track_id), egui::Sense::drag());
    if response.dragged() {
        let modifiers = ui.input(|i| i.modifiers);

        if !modifiers.ctrl && !modifiers.shift && !modifiers.alt {
            // resize current track

            // track.height = (track.height + response.drag_delta().y).clamp(min_height, max_height);
            track.height = (track.height + response.drag_delta().y).max(min_height);
        } else if modifiers.shift {
            // resize all tracks
            // TODO: use action for this?
            let new_height = track.height + response.drag_delta().y;
            // let new_height = (track.height + response.drag_delta().y).clamp(min_height, max_height);
            model.tracks2.set_tracks_height(new_height);
        }
        // track.height = (track.height + response.drag_delta().y).clamp(min_height, max_height);
    }
    // Visual + cursor
    // ui.painter().rect_filled(resize_handle_rect, 0.0, ui.visuals().widgets.inactive.bg_fill);
    if response.hovered() || response.dragged() {
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::ResizeVertical);
    }

    Ok(resp.response)
}

fn ui_waveform(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId) -> Result<()> {
    // TODO: middle line also dependson sample_rect
    ui_middle_line(ui);

    let sample_ix_range = {
        let time_line = model.tracks2.ruler.time_line.as_ref().ok_or(anyhow::anyhow!("No time line"))?;
        time_line.get_ix_range(ui.min_rect().width() as f64)
    };
    let track = model
        .tracks2
        .get_track_mut(track_id)
        .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;

    track.set_ix_range(sample_ix_range, &model.audio)?;
    track.set_screen_rect(ui.min_rect().into());
    track.update_sample_view(&mut model.audio)?;
    let sample_view = track.get_sample_view()?;

    let color = egui::Color32::LIGHT_RED;
    let line_color = color.linear_multiply(0.7);
    let screen_rect = track.screen_rect.ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
    let sample_rect = track.sample_rect.ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;

    match sample_view.data {
        ViewData::Single(ref positions) => {
            if sample_view.samples_per_pixel < 0.25 {
                positions.iter().for_each(|pos| {
                    let pos = rpc(ui, pos.into());
                    let circle_size = 2.0;
                    let circle_color = color;
                    ui.painter().circle_filled(pos, circle_size, circle_color);
                    if let Some(val_rng) = sample_rect.val_rng() {
                        if let Some(y_mid) = sample_value_to_screen_y_e(0.0, val_rng, screen_rect) {
                            let pos_mid = rpc(ui, egui::pos2(pos.x, y_mid));
                            ui.painter().line_segment([pos_mid, pos], egui::Stroke::new(1.0, line_color));
                        }
                    }
                });
            } else {
                let positions = positions.iter().map(|pos| rpc(ui, pos.into())).collect();
                ui.painter().line(positions, egui::Stroke::new(1.0, line_color));
            }
        }
        ViewData::MinMax(ref mix_max_positions) => {
            mix_max_positions.iter().for_each(|pos| {
                let min = rpc(ui, (&pos.min).into());
                let max = rpc(ui, (&pos.max).into());
                let color = egui::Color32::LIGHT_RED;
                ui.painter().line_segment([min, max], egui::Stroke::new(1.0, color));
            });
        }
    };

    Ok(())
}

fn ui_middle_line(ui: &mut egui::Ui) {
    let rect = ui.min_rect();
    let y = rect.center().y;
    let left = rpc(ui, egui::pos2(rect.left(), y));
    let right = rpc(ui, egui::pos2(rect.right(), y));
    ui.painter()
        .line_segment([left, right], egui::Stroke::new(1.0, egui::Color32::GRAY));

    // ui.painter().line_segment([min_x, max_x], ui.visuals().widgets.inactive.bg_stroke);
}

/// round to pixel center (TODO: move to somehwere more general)
pub fn rpc(ui: &egui::Ui, pos: egui::Pos2) -> egui::Pos2 {
    let pos = ui.painter().round_pos_to_pixel_center(pos);
    pos
}
