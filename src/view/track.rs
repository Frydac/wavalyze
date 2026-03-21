use crate::{
    model::{
        Action, Model,
        track::{self, TrackId},
    },
    view::value_ruler2,
};
use anyhow::Result;

#[path = "track/hover.rs"]
mod hover;
#[path = "track/selection.rs"]
mod selection;
#[path = "track/waveform.rs"]
mod waveform;

pub fn ui(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId) -> Result<()> {
    let min_height = track::min_total_height(&model.user_config.track);
    let width = ui.available_width().max(0.0);
    let width_info = model.user_config.tracks_width_info.min(width);
    let height = model
        .tracks
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
            let ruler_height = (height - track::HEADER_HEIGHT).max(0.0);
            let ruler_min = info_rect.max - egui::vec2(ruler_width, ruler_height);
            let ruler_size = egui::vec2(ruler_width, ruler_height);
            let ruler_rect = egui::Rect::from_min_size(ruler_min, ruler_size);
            let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
            ui.painter()
                .rect(ui.min_rect(), 0.0, egui::Color32::TRANSPARENT, stroke);
            if let Some(track) = model.tracks.get_track(track_id) {
                let hover_info = model.tracks.hover_info;
                let mut value_ruler_ctx = value_ruler2::ValueRulerContext {
                    actions: &mut model.actions,
                    hover_info: &hover_info,
                    audio: &model.audio,
                    zoom_y_factor: model.user_config.zoom_x_scroll_factor,
                };
                let value_ruler_config = value_ruler2::ValueRulerConfig {
                    show_hover_tick: false,
                };
                value_ruler2::ui(
                    ui,
                    track,
                    track_id,
                    ruler_rect,
                    value_ruler_config,
                    &mut value_ruler_ctx,
                );
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

            ui.allocate_ui([size.x.max(0.0), track::HEADER_HEIGHT].into(), |ui| {
                let size = ui.available_size();
                let size = egui::vec2(size.x.max(0.0), size.y.max(0.0));
                ui.set_max_size(size);
                ui.set_min_size(size);
                let _ = ui_header(ui, model, track_id);
            });

            let waveform_height = (size.y - track::HEADER_HEIGHT).max(0.0);
            ui.allocate_ui([size.x.max(0.0), waveform_height].into(), |ui| {
                let size = ui.available_size();
                let size = egui::vec2(size.x.max(0.0), size.y.max(0.0));
                ui.set_max_size(size);
                ui.set_min_size(size);
                let waveform_rect = ui.min_rect();
                let _ = waveform::ui_waveform_canvas(ui, model, track_id, waveform_rect);
            });
        });
    });

    // -- Resize handle across full track width --
    {
        const RESIZE_HANDLE_HEIGHT: f32 = 3.0;
        let min_height = track::min_total_height(&model.user_config.track);
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
                .tracks
                .get_track_mut(track_id)
                .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;

            if !modifiers.ctrl && !modifiers.shift && !modifiers.alt {
                track.height = (track.height + response.drag_delta().y).max(min_height);
            } else if modifiers.shift {
                let new_height = (track.height + response.drag_delta().y).max(min_height);
                model.tracks.set_tracks_height(new_height);
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
            let mut text = String::from("track header");
            let mut hover_text = None;
            let mut path_text = None;
            let mut channel_text = None;

            // TODO: store hover text in model.. *smh*
            if let Some((file, channel)) = model.get_file_channel_for_track(track_id) {
                let path = file
                    .path
                    .as_ref()
                    .and_then(|p| p.to_str())
                    .unwrap_or("unknown");
                path_text = Some(path.to_string());
                channel_text = Some(format!(" - ch {}", channel.ch_ix));
                text = format!("{} - ch {}", path, channel.ch_ix);
                hover_text = Some(format!("{file}"));
            }

            let rect = ui.max_rect();
            let rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(rect.width().max(0.0), track::HEADER_HEIGHT),
            );
            ui.set_clip_rect(rect);

            let font_id = ui
                .style()
                .text_styles
                .get(&egui::TextStyle::Body)
                .cloned()
                .unwrap_or_else(|| egui::FontId::proportional(14.0));
            let color = ui.style().visuals.text_color();
            let padding = ui.spacing().button_padding;
            let item_spacing = ui.spacing().item_spacing.x.max(2.0);

            let button_size = |label: &str| {
                let text_size = ui
                    .fonts(|fonts| fonts.layout_no_wrap(label.to_string(), font_id.clone(), color))
                    .size();
                egui::vec2(
                    text_size.x + padding.x * 2.0,
                    (text_size.y + padding.y * 2.0).min(track::HEADER_HEIGHT),
                )
            };

            let mut right = rect.right();
            let button_x_size = button_size("x");
            let button_x_rect = egui::Rect::from_min_size(
                egui::pos2(
                    right - button_x_size.x,
                    rect.center().y - button_x_size.y / 2.0,
                ),
                button_x_size,
            );
            right = button_x_rect.left() - item_spacing;

            let button_center_size = button_size("center y");
            let button_center_rect = egui::Rect::from_min_size(
                egui::pos2(
                    right - button_center_size.x,
                    rect.center().y - button_center_size.y / 2.0,
                ),
                button_center_size,
            );
            right = button_center_rect.left() - item_spacing;

            if ui.put(button_x_rect, egui::Button::new("x")).clicked() {
                model.actions.push(Action::RemoveTrack(track_id));
            }
            if ui
                .put(button_center_rect, egui::Button::new("center y"))
                .clicked()
            {
                model.actions.push(Action::RecenterY { track_id });
            }

            let label_rect =
                egui::Rect::from_min_max(rect.left_top(), egui::pos2(right, rect.bottom()));
            let max_width = label_rect.width().max(0.0);
            let display_text = if let (Some(path), Some(suffix)) = (path_text, channel_text) {
                truncate_path_keep_basename_to_width(ui, &path, &suffix, max_width)
            } else {
                text
            };
            let galley = ui.fonts(|fonts| fonts.layout_no_wrap(display_text, font_id, color));
            let text_pos = egui::pos2(
                label_rect.left() + 2.0,
                rect.center().y - galley.size().y / 2.0,
            );
            ui.painter().galley(text_pos, galley, color);
            if let Some(hover_text) = hover_text {
                ui.interact(
                    label_rect,
                    ui.id().with("header_label"),
                    egui::Sense::hover(),
                )
                .on_hover_text(hover_text);
            }
        });
    Ok(())
}

fn truncate_path_keep_basename_to_width(
    ui: &egui::Ui,
    path: &str,
    suffix: &str,
    max_width: f32,
) -> String {
    let font_id = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .cloned()
        .unwrap_or_else(|| egui::FontId::proportional(14.0));
    let color = ui.style().visuals.text_color();

    let measure = |text: &str| -> f32 {
        ui.fonts(|fonts| {
            fonts
                .layout_no_wrap(text.to_string(), font_id.clone(), color)
                .size()
                .x
        })
    };

    let full = format!("{path}{suffix}");
    if measure(&full) <= max_width {
        return full;
    }

    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let base_with_suffix = format!("{base}{suffix}");
    if measure(&base_with_suffix) > max_width {
        return truncate_basename_to_width(&measure, base, suffix, max_width);
    }

    let parent_len = path.len().saturating_sub(base.len());
    let parent = &path[..parent_len];
    truncate_parent_to_width(&measure, parent, base, suffix, max_width)
}

fn truncate_basename_to_width(
    measure: &dyn Fn(&str) -> f32,
    base: &str,
    suffix: &str,
    max_width: f32,
) -> String {
    let ellipsis = "...";
    let base_chars: Vec<char> = base.chars().collect();
    let mut lo = 0usize;
    let mut hi = base_chars.len();

    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let tail: String = base_chars[base_chars.len() - mid..].iter().collect();
        let candidate = format!("{ellipsis}{tail}{suffix}");
        if measure(&candidate) <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    if lo == 0 {
        return format!("{ellipsis}{suffix}");
    }

    let tail: String = base_chars[base_chars.len() - lo..].iter().collect();
    format!("{ellipsis}{tail}{suffix}")
}

fn truncate_parent_to_width(
    measure: &dyn Fn(&str) -> f32,
    parent: &str,
    base: &str,
    suffix: &str,
    max_width: f32,
) -> String {
    let ellipsis = "...";
    let parent_chars: Vec<char> = parent.chars().collect();
    let mut lo = 0usize;
    let mut hi = parent_chars.len();

    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let tail: String = parent_chars[parent_chars.len() - mid..].iter().collect();
        let candidate = format!("{ellipsis}{tail}{base}{suffix}");
        if measure(&candidate) <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    let tail: String = parent_chars[parent_chars.len() - lo..].iter().collect();
    format!("{ellipsis}{tail}{base}{suffix}")
}

fn resize_handle(ui: &mut egui::Ui, id: egui::Id, rect: egui::Rect) -> egui::Response {
    let response = ui.interact(rect, id, egui::Sense::drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    response
}
