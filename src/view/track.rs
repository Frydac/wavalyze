use crate::{
    audio,
    audio::sample::view::ViewData,
    model::{
        Action, Model,
        hover_info::{HoverInfo, HoverInfoE},
        ruler::sample_value_to_screen_y_e,
        track::TrackId,
    },
    rect::Rect,
    view::{util::rpc, value_ruler2},
};
use anyhow::Result;
const HEADER_HEIGHT: f32 = 22.0;

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
                let hover_info = model.tracks2.hover_info;
                let mut value_ruler_ctx = value_ruler2::ValueRulerContext {
                    actions: &mut model.actions,
                    hover_info: &hover_info,
                    audio: &model.audio,
                    zoom_y_factor: model.user_config.zoom_x_scroll_factor,
                };
                value_ruler2::ui(ui, track, track_id, ruler_rect, &mut value_ruler_ctx);
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
                egui::vec2(rect.width().max(0.0), HEADER_HEIGHT),
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
                    (text_size.y + padding.y * 2.0).min(HEADER_HEIGHT),
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
}

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
    handle_pan_drag(ui, model, track_id, rect);
    ui_waveform(ui, model, track_id, rect)?;
    ui_hover(ui, model, track_id);

    Ok(())
}

fn handle_pan_drag(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId, rect: egui::Rect) {
    let response = ui.interact(
        rect,
        ui.id().with(("pan_drag", track_id)),
        egui::Sense::drag(),
    );
    if response.dragged_by(egui::PointerButton::Secondary) {
        let (delta, modifiers) = ui.input(|i| (i.pointer.delta(), i.modifiers));
        if modifiers.ctrl {
            model.actions.push(Action::PanX {
                nr_pixels: -delta.x,
            });
            model.actions.push(Action::PanY {
                track_id,
                nr_pixels: delta.y,
            });
        } else if modifiers.shift {
            model.actions.push(Action::PanY {
                track_id,
                nr_pixels: delta.y,
            });
        } else {
            model.actions.push(Action::PanX {
                nr_pixels: -delta.x,
            });
        }
    }
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
    draw_value_grid(ui, sample_rect, screen_rect);

    match sample_view.data {
        ViewData::Single(ref positions) => {
            if sample_view.samples_per_pixel < 0.25 {
                positions.iter().for_each(|pos| {
                    // get mid pos, fails only if one of the ranges is invalid, then we don't draw
                    // anyway
                    let Some(val_rng) = sample_rect.val_rng() else {
                        return;
                    };
                    let Some(y_mid) = sample_value_to_screen_y_e(0.0, val_rng, screen_rect) else {
                        return;
                    };
                    let pos_mid = crate::Pos { x: pos.x, y: y_mid };

                    if pos.y < screen_rect.top() && pos_mid.y < screen_rect.top()
                        || pos.y > screen_rect.bottom() && pos_mid.y > screen_rect.bottom()
                    {
                        // both points are outside the screen rect on the same side, we don't need
                        // to draw anything
                        return;
                    }

                    // Here we know we'll have to draw at least a line

                    // prepare pos_mid
                    let pos_mid = screen_rect.clip_pos(pos_mid);
                    let pos_mid = rpc(ui, pos_mid.into());

                    let mut pos = *pos;

                    // We need a circle if the point itself is inside the screen rect.
                    if screen_rect.contains(pos) {
                        // draw circle
                        let circle_size = if sample_view.samples_per_pixel < 1.0 / 16.0 {
                            // a bit bigger when more zoomed in
                            3.0
                        } else {
                            2.0
                        };
                        let circle_color = color;
                        ui.painter()
                            .circle_filled(pos.into(), circle_size, circle_color);
                    } else {
                        // if the point is outside the screen rect, we still draw a line, so we
                        // clip it and don't draw a circle at this pos
                        pos = screen_rect.clip_pos(pos);
                        pos = rpc(ui, pos.into()).into();
                    };

                    // draw line
                    ui.painter()
                        .line_segment([pos_mid, pos.into()], egui::Stroke::new(1.0, line_color));
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

fn draw_value_grid(ui: &mut egui::Ui, sample_rect: audio::SampleRectE, screen_rect: Rect) {
    let Some(val_rng) = sample_rect.val_rng() else {
        return;
    };

    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    let faint = egui::Stroke::new(stroke.width, stroke.color.linear_multiply(0.35));
    let mid = egui::Stroke::new(stroke.width, stroke.color.linear_multiply(0.6));

    for (value, is_mid) in [
        (-1.0_f32, false),
        (-0.5_f32, false),
        (0.0_f32, true),
        (0.5_f32, false),
        (1.0_f32, false),
    ] {
        let Some(y) = sample_value_to_screen_y_e(value, val_rng, screen_rect) else {
            continue;
        };
        if y < screen_rect.top() || y > screen_rect.bottom() {
            continue;
        }
        let left = rpc(ui, egui::pos2(screen_rect.left(), y));
        let right = rpc(ui, egui::pos2(screen_rect.right(), y));
        ui.painter()
            .line_segment([left, right], if is_mid { mid } else { faint });
    }
}
