use crate::{
    audio::sample::view::ViewData,
    model::{
        Action, Model,
        hover_info::{HoverInfo, HoverInfoE},
        ruler::sample_value_to_screen_y_e,
        track2::TrackId,
    },
    view::{util::rpc, value_ruler2},
};
use anyhow::Result;
const HEADER_HEIGHT: f32 = 20.0;

pub fn ui(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId) -> Result<()> {
    let min_height: f32 = model.user_config.track.min_height;
    let width = ui.available_width().max(0.0);
    let width_info = model.user_config.tracks_width_info.min(width);
    let height = model
        .tracks2
        .get_track_height(track_id)
        .unwrap_or(min_height);
    let height = height.max(0.0);

    // reserves space in the parent ui, moves the parent cursor
    let (track_rect, _) =
        ui.allocate_exact_size(egui::Vec2::new(width, height), egui::Sense::hover());

    // crate::view::util::debug_rect_text(ui, track_rect, egui::Color32::RED, "track_rect");

    // create a new child ui with its own cursor of the given size
    let mut track_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(track_rect)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );

    track_ui.horizontal(|ui| {
        ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

        // Draw track info left of waveform
        let info_size = [width_info.max(0.0), height.max(0.0)].into();
        ui.allocate_ui(info_size, |ui| {
            ui.set_max_size(info_size);
            ui.set_min_size(info_size);
            let info_rect = ui.min_rect();

            // sample value ruler
            let ruler_width = 100.0;
            let ruler_height = (height - HEADER_HEIGHT).max(0.0);
            let ruler_min = info_rect.max - egui::vec2(ruler_width, ruler_height);
            let ruler_size = egui::vec2(ruler_width, ruler_height);
            let ruler_rect = egui::Rect::from_min_size(ruler_min, ruler_size);
            let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
            ui.painter()
                .rect(ui.min_rect(), 0.0, egui::Color32::TRANSPARENT, stroke);
            if let Some(track) = model.tracks2.get_track(track_id) {
                value_ruler2::ui(ui, track, track_id, ruler_rect, &mut model.actions);
            }
        });

        // Draw track waveform + header
        ui.vertical(|ui| {
            ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);

            let size = ui.available_size();
            let size = egui::vec2(size.x.max(0.0), size.y.max(0.0));
            ui.set_max_size(size);
            ui.set_min_size(size);
            let rect_wf_group = ui.min_rect();

            ui.allocate_ui([size.x.max(0.0), HEADER_HEIGHT].into(), |ui| {
                let size = ui.available_size();
                let size = egui::vec2(size.x.max(0.0), size.y.max(0.0));
                ui.set_max_size(size);
                ui.set_min_size(size);
                let _ = ui_header(ui, model, track_id);
            });

            let waveform_height = (size.y - HEADER_HEIGHT).max(0.0);
            ui.allocate_ui([size.x.max(0.0), waveform_height].into(), |ui| {
                let size = ui.available_size();
                let size = egui::vec2(size.x.max(0.0), size.y.max(0.0));
                ui.set_max_size(size);
                ui.set_min_size(size);
                let waveform_rect = ui.min_rect();
                let _ = ui_waveform_canvas(ui, model, track_id, waveform_rect);
            });
        });
    });

    // -- Resize handle across full track width --
    {
        const RESIZE_HANDLE_HEIGHT: f32 = 3.0;
        let min_height: f32 = model.user_config.track.min_height + HEADER_HEIGHT;
        let resize_handle_rect = egui::Rect::from_min_size(
            egui::pos2(
                track_rect.left(),
                track_rect.bottom() - RESIZE_HANDLE_HEIGHT,
            ),
            egui::vec2(track_rect.width(), RESIZE_HANDLE_HEIGHT),
        );
        let resize_id = track_ui.id().with(track_id);
        let response = resize_handle(&mut track_ui, resize_id, resize_handle_rect);
        if response.dragged() {
            let modifiers = track_ui.input(|i| i.modifiers);
            let track = model
                .tracks2
                .get_track_mut(track_id)
                .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;

            if !modifiers.ctrl && !modifiers.shift && !modifiers.alt {
                track.height = (track.height + response.drag_delta().y).max(min_height);
                dbg!(track.height);
            } else if modifiers.shift {
                let new_height = (track.height + response.drag_delta().y).max(min_height);
                dbg!(new_height);
                model.tracks2.set_tracks_height(new_height);
            }
        }
    }

    Ok(())
}

pub fn ui_header(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId) -> Result<()> {
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
                    let path = file
                        .path
                        .as_ref()
                        .and_then(|p| p.to_str())
                        .unwrap_or("unknown");
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
        });
    Ok(())
}

pub fn ui_hover(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId) {
    // Draw hover info
    match &model.tracks2.hover_info {
        HoverInfoE::NotHovered => {}
        HoverInfoE::IsHovered(hover_info) => {
            // Draw vertical line always when hover info is present
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

            // Draw horizontal line only mouse is over the track
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

    // Do hover interaction
    {
        let rect = ui.min_rect();
        let hover_response = ui
            .interact(rect, egui::Id::new(track_id), egui::Sense::hover())
            .on_hover_cursor(egui::CursorIcon::None);

        if let Some(pos) = ui.ctx().pointer_hover_pos()
            && rect.contains(pos)
        {
            model
                .actions
                .push(Action::SetHoverInfo(HoverInfoE::IsHovered(HoverInfo {
                    screen_pos: pos.into(),
                    sample_ix: model
                        .tracks2
                        .ruler
                        .screen_x_to_sample_ix(pos.x)
                        .unwrap_or(0.0),
                })));
            ui.ctx().input(|i| {
                if i.modifiers.shift && !i.modifiers.ctrl {
                    let scroll = i.raw_scroll_delta;
                    if scroll.x != 0.0 {
                        let zoom_x_factor = model.user_config.zoom_x_scroll_factor;
                        model.actions.push(Action::ShiftX {
                            nr_pixels: scroll.x,
                        });
                    }
                } else if i.modifiers.ctrl {
                    let scroll = i.raw_scroll_delta;
                    if scroll.y != 0.0 {
                        let zoom_x_factor = model.user_config.zoom_x_scroll_factor;
                        model.actions.push(Action::ZoomX {
                            nr_pixels: scroll.y * zoom_x_factor,
                            center_x: pos.x,
                        });
                    }
                }
            });
        }
    }
}

// Wrap the waveform in a (manually implemented) resizable frame
// TODO: see if we can extrac like a resizable canvas or something
pub fn ui_waveform_canvas(
    ui: &mut egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    rect: egui::Rect,
) -> Result<()> {
    let size = ui.available_size();
    ui.set_max_size(size);
    ui.set_min_size(size);

    // -- Waveform canvas --
    let bg_color = ui.visuals().extreme_bg_color;
    let stroke = ui.visuals().window_stroke();
    // let stroke = egui::Stroke::NONE;
    ui.painter().rect(rect, 0.0, bg_color, stroke);
    let _ = ui_waveform(ui, model, track_id, rect);
    ui_hover(ui, model, track_id);

    // Ok(resp.response)
    Ok(())
}

fn resize_handle(ui: &mut egui::Ui, id: egui::Id, rect: egui::Rect) -> egui::Response {
    let response = ui.interact(rect, id, egui::Sense::drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    response
}

fn ui_waveform(
    ui: &mut egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    rect: egui::Rect,
) -> Result<()> {
    // TODO: middle line also dependson sample_rect
    // ui_middle_line(ui);

    let sample_ix_range = {
        let time_line = model
            .tracks2
            .ruler
            .time_line
            .as_ref()
            .ok_or(anyhow::anyhow!("No time line"))?;
        time_line.get_ix_range(ui.min_rect().width() as f64)
    };
    let track = model
        .tracks2
        .get_track_mut(track_id)
        .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;

    track.set_ix_range(sample_ix_range, &model.audio)?;
    track.set_screen_rect(rect.into());
    track.update_sample_view(&mut model.audio)?;
    let sample_view = track.get_sample_view()?;

    let color = egui::Color32::LIGHT_RED;
    let line_color = color.linear_multiply(0.7);
    let screen_rect = track
        .screen_rect
        .ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
    let sample_rect = track
        .single
        .item
        .sample_rect()
        .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;

    match sample_view.data {
        ViewData::Single(ref positions) => {
            if sample_view.samples_per_pixel < 0.25 {
                positions.iter().for_each(|pos| {
                    let pos = rpc(ui, pos.into());
                    if !screen_rect.contains(pos.into()) {
                        return;
                    }
                    let circle_size = 2.0;
                    let circle_color = color;
                    ui.painter().circle_filled(pos, circle_size, circle_color);
                    if let Some(val_rng) = sample_rect.val_rng()
                        && let Some(y_mid) = sample_value_to_screen_y_e(0.0, val_rng, screen_rect)
                    {
                        let pos_mid = rpc(ui, egui::pos2(pos.x, y_mid));
                        if screen_rect.contains(pos_mid.into()) {
                            ui.painter()
                                .line_segment([pos_mid, pos], egui::Stroke::new(1.0, line_color));
                        }
                    }
                });
            } else {
                let positions = positions
                    .iter()
                    .map(|pos| rpc(ui, pos.into()))
                    .filter(|pos| screen_rect.contains((*pos).into()))
                    .collect();
                ui.painter()
                    .line(positions, egui::Stroke::new(1.0, line_color));
            }
        }
        ViewData::MinMax(ref mix_max_positions) => {
            mix_max_positions.iter().for_each(|pos| {
                let min = rpc(ui, (&pos.min).into());
                let max = rpc(ui, (&pos.max).into());
                if !screen_rect.contains(min.into()) && !screen_rect.contains(max.into()) {
                    return;
                }
                let color = egui::Color32::LIGHT_RED;
                ui.painter()
                    .line_segment([min, max], egui::Stroke::new(1.0, color));
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
