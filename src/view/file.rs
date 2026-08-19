use crate::{
    model::{Action, FileVisibilityState, Model},
    view::{grid::KeyValueGrid, util::add_row_label},
    wav,
    wav::file::FileId,
};

#[derive(Debug, Clone)]
struct FileRow {
    file_id: FileId,
    title: String,
    hover_text: String,
    visibility: FileVisibilityState,
    sample_ix_offset: crate::audio::sample::Ix,
    metadata: Vec<(&'static str, String)>,
    channels: Vec<ChannelRow>,
}

#[derive(Debug, Clone)]
struct ChannelRow {
    buffer_id: Option<crate::audio::BufferId>,
    label: String,
    visible: bool,
    missing_track: bool,
}

pub fn ui(ui: &mut egui::Ui, model: &mut Model) {
    ui.add_space(5.0);
    ui.heading("Files");
    ui.add_space(5.0);

    if model.files.is_empty() {
        ui.label("No files loaded");
        return;
    }

    egui::ScrollArea::both().show(ui, |ui| {
        // PERF: we could cache this datastructure, but I expect we won't have that many files open
        // at the same time.
        let rows: Vec<_> = model
            .files_order
            .iter()
            .filter_map(|&file_id| model.files.get(file_id).map(|file| (file_id, file)))
            .map(|(file_id, file)| FileRow {
                file_id,
                title: file_title(file),
                hover_text: format!("{file}"),
                visibility: model
                    .file_visibility_state_for(file_id)
                    .unwrap_or(FileVisibilityState::NoneVisible),
                sample_ix_offset: file.sample_ix_offset,
                metadata: metadata_rows(file),
                channels: channel_rows(model, file),
            })
            .collect();

        for row in rows {
            ui.push_id(("file_tree", row.file_id), |ui| {
                let mut root_checked = row.visibility == FileVisibilityState::AllVisible;
                let id = ui.make_persistent_id(("file_header", row.file_id));
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
                        model.set_file_visible_for(row.file_id, make_visible);
                    }
                    add_row_label(ui, &row.title)
                        .on_hover_text(row.hover_text)
                        .context_menu(|ui| {
                            if ui.button("Close file").clicked() {
                                model.actions.push(Action::CloseFile {
                                    file_id: row.file_id,
                                });
                                ui.close_menu();
                            }
                        });
                })
                .body(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("offset:");
                        let mut sample_ix_offset = row.sample_ix_offset;
                        if ui
                            .add(egui::DragValue::new(&mut sample_ix_offset).speed(1.0))
                            .changed()
                        {
                            model.actions.push(Action::SetFileSampleIxOffset {
                                file_id: row.file_id,
                                sample_ix_offset,
                            });
                        }
                    });

                    egui::CollapsingHeader::new("Metadata")
                        .id_salt("metadata")
                        .default_open(false)
                        .show(ui, |ui| {
                            let id: u64 = ui.id().with("file_metadata_grid").value();
                            let mut grid = KeyValueGrid::new(id).key_col_width(80.0);
                            for (key, value) in &row.metadata {
                                grid.row(*key, value.clone());
                            }
                            grid.show(ui);
                        });

                    egui::CollapsingHeader::new("Channels")
                        .id_salt("channels")
                        .default_open(true)
                        .show(ui, |ui| ui_channels(ui, model, &row.channels));
                });
            });
            ui.add_space(2.0);
        }
    });
}

fn ui_channels(ui: &mut egui::Ui, model: &mut Model, channels: &[ChannelRow]) {
    for channel in channels {
        ui.horizontal(|ui| {
            let mut checked = channel.visible;
            let response = ui.add_enabled(
                !channel.missing_track,
                egui::Checkbox::without_text(&mut checked),
            );
            if response.changed()
                && let Some(buffer_id) = channel.buffer_id
            {
                model.set_channel_visible(buffer_id, checked);
            }
            let channel_label = if channel.buffer_id.is_none() {
                format!("{} (not loaded)", channel.label)
            } else if channel.missing_track {
                format!("{} (closed)", channel.label)
            } else {
                channel.label.clone()
            };
            let label_response = if channel.buffer_id.is_none() {
                ui.add_enabled_ui(false, |ui| add_row_label(ui, channel_label))
                    .inner
            } else {
                add_row_label(ui, channel_label)
            };
            if let Some(buffer_id) = channel.buffer_id {
                label_response.context_menu(|ui| {
                    let button_label = if channel.missing_track {
                        "Load track"
                    } else {
                        "Remove track"
                    };
                    if ui.button(button_label).clicked() {
                        let result = if channel.missing_track {
                            model.restore_channel_track(buffer_id)
                        } else {
                            Ok(model.remove_channel_track(buffer_id))
                        };
                        if let Err(err) = result {
                            tracing::error!(
                                "Failed to toggle track for buffer {:?}: {err}",
                                buffer_id
                            );
                        }
                        ui.close_menu();
                    }
                });
            }
        });
    }
}

fn metadata_rows(file: &wav::file::File) -> Vec<(&'static str, String)> {
    vec![
        (
            "path",
            file.path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "unknown".to_string()),
        ),
        (
            "channels",
            format!(
                "{} / {} loaded",
                file.channels.len(),
                file.total_nr_channels
            ),
        ),
        ("sample type", format!("{:?}", file.sample_type)),
        ("bit depth", file.bit_depth.to_string()),
        ("sample rate", format!("{} Hz", file.sample_rate)),
        ("samples", file.nr_samples.to_string()),
        (
            "duration",
            if file.sample_rate == 0 {
                "unknown".to_string()
            } else {
                format!("{:.3} s", file.nr_samples as f64 / file.sample_rate as f64)
            },
        ),
        (
            "layout",
            file.layout
                .as_ref()
                .map(|layout| format!("{layout:?}"))
                .unwrap_or_else(|| "unknown".to_string()),
        ),
    ]
}

fn channel_rows(model: &Model, file: &wav::file::File) -> Vec<ChannelRow> {
    (0..file.total_nr_channels)
        .map(|ch_ix| {
            let channel = file.channels.get(&ch_ix);
            let track = channel.and_then(|channel| {
                model
                    .find_track_id_for_buffer(channel.buffer_id)
                    .and_then(|track_id| model.tracks.get_track(track_id))
            });
            ChannelRow {
                buffer_id: channel.map(|channel| channel.buffer_id),
                label: channel
                    .map(channel_label)
                    .unwrap_or_else(|| format!("ch {ch_ix}")),
                visible: track.is_some_and(|track| track.visible),
                missing_track: track.is_none(),
            }
        })
        .collect()
}

fn file_title(file: &wav::file::File) -> String {
    file.path
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or_else(|| "Demo".to_string())
}

fn channel_label(channel: &wav::file::Channel) -> String {
    match channel.channel_id {
        Some(channel_id) => format!("ch {} - {}", channel.ch_ix, channel_id.long_name()),
        None => format!("ch {}", channel.ch_ix),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{self, buffer::BufferE};
    use std::{collections::BTreeMap, sync::Arc};

    #[test]
    fn channel_rows_include_channels_excluded_from_load() {
        let mut model = Model::new();
        let buffer_id =
            model
                .audio
                .buffers
                .insert(Arc::new(BufferE::F32(audio::buffer::Buffer::with_size(
                    48_000, 32, 16,
                ))));
        let file = wav::file::File {
            channels: BTreeMap::from([
                (
                    0,
                    wav::file::Channel {
                        ch_ix: 0,
                        buffer_id,
                        channel_id: None,
                    },
                ),
                (
                    2,
                    wav::file::Channel {
                        ch_ix: 2,
                        buffer_id,
                        channel_id: None,
                    },
                ),
            ]),
            total_nr_channels: 4,
            sample_type: audio::SampleType::Float,
            bit_depth: 32,
            sample_rate: 48_000,
            layout: None,
            path: None,
            nr_samples: 16,
            sample_ix_offset: 0,
        };

        let rows = channel_rows(&model, &file);
        let metadata = metadata_rows(&file);

        assert_eq!(
            metadata
                .iter()
                .find(|(key, _)| *key == "channels")
                .unwrap()
                .1,
            "2 / 4 loaded"
        );
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].label, "ch 0");
        assert!(rows[0].buffer_id.is_some());
        assert_eq!(rows[1].label, "ch 1");
        assert!(rows[1].buffer_id.is_none());
        assert!(rows[1].missing_track);
        assert_eq!(rows[2].label, "ch 2");
        assert!(rows[2].buffer_id.is_some());
        assert_eq!(rows[3].label, "ch 3");
        assert!(rows[3].buffer_id.is_none());
    }
}
