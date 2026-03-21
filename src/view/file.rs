use crate::{
    model::{FileVisibilityState, Model},
    wav,
};

#[derive(Debug, Clone)]
struct FileRow {
    file_ix: usize,
    title: String,
    hover_text: String,
    visibility: FileVisibilityState,
    channels: Vec<ChannelRow>,
}

#[derive(Debug, Clone)]
struct ChannelRow {
    buffer_id: crate::audio::BufferId,
    label: String,
    visible: bool,
    missing_track: bool,
}

pub fn ui(ui: &mut egui::Ui, model: &mut Model) {
    ui.add_space(5.0);
    ui.heading("Files");
    ui.add_space(5.0);

    if model.files2.is_empty() {
        ui.label("No files loaded");
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        let rows: Vec<_> = model
            .files2
            .iter()
            .enumerate()
            .map(|(file_ix, file)| FileRow {
                file_ix,
                title: file_title(file),
                hover_text: format!("{file}"),
                visibility: model
                    .file_visibility_state_at(file_ix)
                    .unwrap_or(FileVisibilityState::NoneVisible),
                channels: file
                    .channels
                    .values()
                    .map(|channel| {
                        let track = model
                            .find_track_id_for_buffer(channel.buffer_id)
                            .and_then(|track_id| model.tracks.get_track(track_id));
                        ChannelRow {
                            buffer_id: channel.buffer_id,
                            label: channel_label(channel),
                            visible: track.is_some_and(|track| track.visible),
                            missing_track: track.is_none(),
                        }
                    })
                    .collect(),
            })
            .collect();

        for row in rows {
            ui.push_id(("file_tree", row.file_ix), |ui| {
                let mut root_checked = row.visibility == FileVisibilityState::AllVisible;
                let id = ui.make_persistent_id(("file_header", row.file_ix));
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    id,
                    true,
                )
                .show_header(ui, |ui| {
                    let response = ui.add(
                        egui::Checkbox::without_text(&mut root_checked)
                            .indeterminate(row.visibility == FileVisibilityState::PartiallyVisible),
                    );
                    if response.changed() {
                        let make_visible = row.visibility != FileVisibilityState::AllVisible;
                        model.set_file_visible_at(row.file_ix, make_visible);
                    }
                    add_row_label(ui, &row.title).on_hover_text(row.hover_text);
                })
                .body(|ui| {
                    for channel in row.channels {
                        ui.horizontal(|ui| {
                            let mut checked = channel.visible;
                            let response = ui.add_enabled(
                                !channel.missing_track,
                                egui::Checkbox::without_text(&mut checked),
                            );
                            if response.changed() {
                                model.set_channel_visible(channel.buffer_id, checked);
                            }
                            let channel_label = if channel.missing_track {
                                format!("{} (closed)", channel.label)
                            } else {
                                channel.label
                            };
                            add_row_label(ui, channel_label).context_menu(|ui| {
                                let button_label = if channel.missing_track {
                                    "Load track"
                                } else {
                                    "Remove track"
                                };
                                if ui.button(button_label).clicked() {
                                    let result = if channel.missing_track {
                                        model.restore_channel_track(channel.buffer_id)
                                    } else {
                                        Ok(model.remove_channel_track(channel.buffer_id))
                                    };
                                    if let Err(err) = result {
                                        tracing::error!(
                                            "Failed to toggle track for buffer {:?}: {err}",
                                            channel.buffer_id
                                        );
                                    }
                                    ui.close_menu();
                                }
                            });
                        });
                    }
                });
            });
            ui.add_space(2.0);
        }
    });
}

fn file_title(file: &wav::file2::File) -> String {
    file.path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| "Demo".to_string())
}

fn channel_label(channel: &wav::file2::Channel) -> String {
    match channel.channel_id {
        Some(channel_id) => format!("ch {} - {}", channel.ch_ix, channel_id.long_name()),
        None => format!("ch {}", channel.ch_ix),
    }
}

fn add_row_label(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) -> egui::Response {
    let size = egui::vec2(ui.available_width().max(0.0), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Min), |ui| {
        ui.add(egui::Label::new(text).truncate())
    })
    .inner
}
