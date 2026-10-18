//! The Files sidebar: file metadata, channel visibility, and pending disk changes.
//!
//! The file tree manages the loaded workspace, while the native-only changed-files component
//! previews affected tracks and queues individual or batch reloads. Detection, dependency
//! discovery, and audio replacement stay in the model so drawing does not perform disk work.

use crate::{
    model::{Action, FileVisibilityState, Model},
    view::{grid::KeyValueGrid, util::add_row_label},
    wav,
    wav::file::FileId,
};

const PATH_METADATA_KEY: &str = "path";

pub(super) fn take_external_drop_target(ctx: &egui::Context) -> Option<FileId> {
    ctx.data_mut(|data| data.remove_temp::<FileId>(egui::Id::new("files_external_drop_target")))
}

/// File-tree presentation snapshot, allowing rendering to enqueue actions without holding file borrows.
#[derive(Debug, Clone)]
struct FileRow {
    file_id: FileId,
    title: String,
    color: egui::Color32,
    hover_text: String,
    visibility: FileVisibilityState,
    sample_ix_offset: crate::audio::sample::Ix,
    metadata: Vec<(&'static str, String)>,
    channels: Vec<ChannelRow>,
    interpretation: wav::channel_interpretation::ChannelInterpretation,
    channel_mask: Option<u32>,
    total_nr_channels: usize,
}

/// Distinguish visible, hidden, closed, and never-loaded channels in the file tree.
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

    #[cfg(not(target_arch = "wasm32"))]
    changed_files_ui(ui, model);

    if model.files.is_empty() {
        ui.label("No files loaded");
        return;
    }

    egui::ScrollArea::both().show(ui, |ui| {
        // PERF: we could cache this datastructure, but I expect we won't have that many files open
        // at the same time.
        let rows: Vec<_> = model
            .files.order()
            .iter()
            .filter_map(|&file_id| model.files.get(file_id).map(|file| (file_id, file)))
            .map(|(file_id, file)| FileRow {
                file_id,
                title: file_title(file),
                color: model.files.color(file_id).unwrap_or_default(),
                hover_text: format!("{file}"),
                visibility: model
                    .file_visibility_state_for(file_id)
                    .unwrap_or(FileVisibilityState::NoneVisible),
                sample_ix_offset: file.sample_ix_offset,
                metadata: metadata_rows(file),
                channels: channel_rows(model, file),
                interpretation: file.channel_interpretation.clone(),
                channel_mask: file.channel_mask,
                total_nr_channels: file.total_nr_channels,
            })
            .collect();

        for row in rows {
            ui.push_id(("file_tree", row.file_id), |ui| {
                let mut root_checked = row.visibility == FileVisibilityState::AllVisible;
                let id = ui.make_persistent_id(("file_header", row.file_id));
                ui.add_space(4.0);
                let (_, header, _) = egui::collapsing_header::CollapsingState::load_with_default_open(
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
                    let mut color = row.color;
                    if egui::color_picker::color_edit_button_srgba(ui, &mut color, egui::color_picker::Alpha::Opaque)
                        .on_hover_text("File color — click to change")
                        .changed()
                    {
                        model.actions.push(Action::SetFileColor { file_id: row.file_id, color });
                    }
                    let drag = ui.dnd_drag_source(ui.id().with("file_drag"), row.file_id, |ui| {
                        add_row_label(ui, &row.title)
                    });
                    drag.inner
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
                    ui.add_space(4.0);
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
                        if ui
                            .small_button("≠")
                            .on_hover_text("Align first non-zero sample to timeline sample zero")
                            .clicked()
                        {
                            model.actions.push(Action::DetectFileOffset {
                                file_id: row.file_id,
                                mode: crate::model::jobs::OffsetDetectionMode::FirstNonZero,
                            });
                        }
                        if ui
                            .small_button("0")
                            .on_hover_text(
                                "Align last leading zero before first non-zero to timeline sample zero",
                            )
                            .clicked()
                        {
                            model.actions.push(Action::DetectFileOffset {
                                file_id: row.file_id,
                                mode: crate::model::jobs::OffsetDetectionMode::LastLeadingZero,
                            });
                        }
                    });

                    let mut interpretation = row.interpretation.clone();
                    let mut interpretation_changed = super::channel_interpretation::order_controls(ui, &mut interpretation, row.channel_mask, row.total_nr_channels);

                    egui::CollapsingHeader::new("Metadata")
                        .id_salt("metadata")
                        .default_open(false)
                        .show(ui, |ui| {
                            let id: u64 = ui.id().with("file_metadata_grid").value();
                            let mut grid = KeyValueGrid::new(id).key_col_width(80.0);
                            for (key, value) in &row.metadata {
                                if *key == PATH_METADATA_KEY {
                                    grid.row_with_hover(*key, value.clone(), value.clone());
                                } else {
                                    grid.row(*key, value.clone());
                                }
                            }
                            grid.show_with(ui, |ui| {
                                ui.label("active layout");
                                interpretation_changed |= super::channel_interpretation::layout_selector(ui, &mut interpretation, row.channel_mask, row.total_nr_channels);
                                ui.end_row();
                            });
                        });

                    if interpretation_changed {
                        model.actions.push(Action::SetFileChannelInterpretation { file_id: row.file_id, interpretation });
                    }

                    egui::CollapsingHeader::new("Channels")
                        .id_salt("channels")
                        .default_open(true)
                        .show(ui, |ui| ui_channels(ui, model, &row.channels));
                });
                file_drop_target(ui, model, row.file_id, header.response.rect);
            });
            ui.add_space(4.0);
        }
    });
}

fn file_drop_target(ui: &mut egui::Ui, model: &mut Model, file_id: FileId, header: egui::Rect) {
    let rect = egui::Rect::from_min_max(
        egui::pos2(ui.max_rect().left(), header.top() - 4.0),
        egui::pos2(ui.max_rect().right(), header.bottom() + 4.0),
    )
    .intersect(ui.clip_rect());
    if !rect.is_positive() {
        return;
    }
    let response = ui.interact(rect, ui.id().with("file_drop"), egui::Sense::hover());
    let external = ui
        .input(|input| !input.raw.hovered_files.is_empty() || !input.raw.dropped_files.is_empty());
    // OS file drags may stop normal pointer-hover events. Always expose the targets,
    // and use available coordinates rather than widget hover state to emphasize one.
    let external_on = external
        && ui
            .input(|input| input.pointer.hover_pos())
            .is_some_and(|pos| {
                rect.contains(pos) && ui.ctx().layer_id_at(pos) == Some(ui.layer_id())
            });
    if external_on {
        ui.ctx().data_mut(|data| {
            data.insert_temp(egui::Id::new("files_external_drop_target"), file_id)
        });
    }
    let internal_on = response
        .dnd_hover_payload::<FileId>()
        .is_some_and(|source| *source != file_id);
    if external || internal_on {
        let active = external_on || internal_on;
        let fill = ui
            .visuals()
            .selection
            .bg_fill
            .linear_multiply(if active { 0.3 } else { 0.12 });
        let stroke = egui::Stroke::new(
            if active { 2.0 } else { 1.0 },
            ui.visuals()
                .selection
                .stroke
                .color
                .gamma_multiply(if active { 1.0 } else { 0.6 }),
        );
        ui.painter().rect(
            rect.shrink(1.0),
            2.0,
            fill,
            stroke,
            egui::epaint::StrokeKind::Inside,
        );
    }
    if let Some(source) = response.dnd_release_payload::<FileId>()
        && *source != file_id
    {
        model.actions.push(Action::DiffFiles {
            file_a: *source,
            file_b: file_id,
        });
    }
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
    let resolved = file.resolved_channel_interpretation();
    vec![
        (
            "channel order",
            super::channel_interpretation::source_label(resolved.source).into(),
        ),
        (
            "WAVEX mask",
            file.channel_mask
                .map(|mask| format!("0x{mask:08X}"))
                .unwrap_or_else(|| "absent".into()),
        ),
        (
            "layout source",
            if file.layout.is_none() {
                "unknown"
            } else if resolved.source == wav::channel_interpretation::InterpretationSource::Wavex {
                "WAVEX"
            } else if file.channel_interpretation.active_layout.is_some() {
                "selected"
            } else {
                "provided"
            }
            .into(),
        ),
        (
            PATH_METADATA_KEY,
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
    ]
}

fn channel_rows(model: &Model, file: &wav::file::File) -> Vec<ChannelRow> {
    let resolved = file.resolved_channel_interpretation();
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
                label: channel_label(
                    ch_ix,
                    channel
                        .map(|ch| ch.channel_id)
                        .unwrap_or(resolved.channels[ch_ix].id),
                    resolved.channels[ch_ix].padding,
                ),
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

fn channel_label(ch_ix: usize, id: Option<crate::audio::Id>, padding: bool) -> String {
    let role = id
        .map(|id| format!(" - {}", id.long_name()))
        .unwrap_or_default();
    let padding = if padding { " (padding)" } else { "" };
    format!("ch {ch_ix}{role}{padding}")
}

/// Keep notification controls separate from the existing file tree; both enqueue model actions.
#[cfg(not(target_arch = "wasm32"))]
fn changed_files_ui(ui: &mut egui::Ui, model: &mut Model) {
    if let Some(error) = model.files.monitoring_error() {
        ui.colored_label(ui.visuals().warn_fg_color, error);
    }
    let ids = model.files.changed_ids();
    if ids.is_empty() {
        return;
    }
    let busy = model.files.reload_running();
    let all_ready = ids.iter().all(|id| {
        model
            .files
            .change_for(*id)
            .is_some_and(|change| change.ready())
    });
    ui.group(|ui| {
        ui.strong(format!("Changed files ({})", ids.len()));
        if ui
            .add_enabled(!busy && all_ready, egui::Button::new("Update all"))
            .clicked()
        {
            model.actions.push(Action::ReloadFiles(ids.clone()));
        }
        if busy {
            ui.label("Updating files and diffs…");
        }
        for id in ids {
            let file = &model.files[id];
            let source = file.source.as_ref().unwrap();
            let path = crate::model::files::disk::absolute_path(&source.filepath);
            let change = model
                .files
                .change_for(id)
                .expect("changed file has a monitored source");
            let ready = change.ready();
            let error = change.error.clone();
            let title = file_title(file);
            let affected = model.affected_reload_tracks(&[id]);
            ui.push_id(("changed_file", id), |ui| {
                ui.horizontal(|ui| {
                    let update = ui.add_enabled(!busy && ready, egui::Button::new("Update"));
                    if update.clicked() {
                        model.actions.push(Action::ReloadFiles(vec![id]));
                    }
                    add_row_label(ui, title).on_hover_ui(|ui| {
                        affected_tracks_hover(ui, model, id, &path, &affected);
                    });
                });
                if !ready {
                    ui.small("Waiting for file changes to settle…");
                }
                if let Some(error) = error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                }
            });
        }
    });
    ui.add_space(5.0);
}

#[cfg(not(target_arch = "wasm32"))]
fn affected_tracks_hover(
    ui: &mut egui::Ui,
    model: &Model,
    file_id: FileId,
    path: &std::path::Path,
    affected: &[crate::model::track::TrackId],
) {
    ui.label(path.display().to_string());
    ui.label(format!(
        "File offset: {} samples",
        model.files[file_id].sample_ix_offset
    ));
    ui.label("Reload preserves existing file, track, and diff alignment offsets; automatic alignment is not rerun.");
    ui.separator();
    let describe = |buffer_id| {
        model
            .files
            .channel_for_buffer(buffer_id)
            .map(|(file, channel)| format!("{} ch {}", file_title(file), channel.ch_ix))
            .unwrap_or_else(|| format!("diff {buffer_id:?}"))
    };
    egui::Grid::new(("affected_tracks", file_id))
        .striped(true)
        .show(ui, |ui| {
            ui.strong("Track / channel");
            ui.strong("Visible");
            ui.strong("Offset (samples)");
            ui.end_row();
            for track_id in affected {
                let track = model.tracks.get_track(*track_id).unwrap();
                let (label, offsets) = if let Some(diff) = &track.diff {
                    (
                        format!(
                            "Diff: {} − {}",
                            describe(diff.buffer_id_a),
                            describe(diff.buffer_id_b)
                        ),
                        format!(
                            "A {}, B {}; display {}",
                            diff.sample_ix_offset_a,
                            diff.sample_ix_offset_b,
                            track.single.sample_ix_offset
                        ),
                    )
                } else {
                    (
                        describe(track.single.buffer_id),
                        track.single.sample_ix_offset.to_string(),
                    )
                };
                ui.label(label);
                ui.label(if track.visible { "Yes" } else { "No" });
                ui.label(offsets);
                ui.end_row();
            }
        });
    if affected.is_empty() {
        ui.label("No open tracks; stored channel audio will be updated.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{self, buffer::BufferE};
    use std::{collections::BTreeMap, sync::Arc};

    #[test]
    fn dragging_a_file_header_onto_another_opens_loaded_channel_pairing() {
        let mut model = Model::new();
        #[cfg(not(target_arch = "wasm32"))]
        model.files.changes.disable_watcher();
        let mut files = Vec::new();
        for name in ["source.wav", "target.wav"] {
            let config = wav::ReadConfigBytes::new(
                Some(name.into()),
                wav::channel_mask::tests::wav_bytes(None, 3, &[]),
            )
            .with_ch_ixs([2]);
            files.push(
                model
                    .add_loaded_file(
                        wav::read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap(),
                    )
                    .unwrap(),
            );
        }
        let original_order = model.tracks.tracks_order.clone();
        let ctx = egui::Context::default();
        let frame = |model: &mut Model, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| super::ui(ui, model));
                },
            )
        };
        let _ = frame(&mut model, vec![]);
        let output = frame(&mut model, vec![]);
        let label_center = |name: &str| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == name => {
                        Some(text.pos + text.galley.size() / 2.0)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing file header {name}"))
        };
        let source = label_center("source.wav");
        let target = label_center("target.wav");
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let _ = frame(
            &mut model,
            vec![egui::Event::PointerMoved(source), button(source, true)],
        );
        let _ = frame(&mut model, vec![egui::Event::PointerMoved(target)]);
        let output = frame(&mut model, vec![]);
        assert!(egui::DragAndDrop::payload::<FileId>(&ctx).is_some());
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.stroke.width == 2.0 && rect.stroke.color == ctx.style().visuals.selection.stroke.color)));
        let _ = frame(&mut model, vec![button(target, false)]);
        let action = model.actions.pop().expect("file drop should queue pairing");
        assert!(
            matches!(action, Action::DiffFiles { file_a, file_b } if file_a == files[0] && file_b == files[1])
        );
        action.process(&mut model).unwrap();
        let pending = model.pending_diff_pairing.as_ref().unwrap();
        assert_eq!(pending.ch_ixs_a, [2]);
        assert_eq!(pending.ch_ixs_b, [2]);
        assert!(pending.existing_files.is_some());
        assert_eq!(model.files.len(), 2);
        assert_eq!(model.tracks.tracks_order, original_order);
    }

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
        let mut file = wav::file::File {
            channel_interpretation: Default::default(),
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
            channel_mask: None,
            source: None,
            path: None,
            nr_samples: 16,
            sample_ix_offset: 0,
        };

        file.set_channel_interpretation(Default::default());
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
        assert_eq!(rows[0].label, "ch 0 - Left");
        assert!(rows[0].buffer_id.is_some());
        assert_eq!(rows[1].label, "ch 1 - Right");
        assert!(rows[1].buffer_id.is_none());
        assert!(rows[1].missing_track);
        assert_eq!(rows[2].label, "ch 2 - Center");
        assert!(rows[2].buffer_id.is_some());
        assert_eq!(rows[3].label, "ch 3 - LFE");
        assert!(rows[3].buffer_id.is_none());
    }
    #[test]
    fn interpretation_action_updates_sidebar_metadata_padding_and_track_titles() {
        use wav::channel_interpretation::{ChannelInterpretation, ChannelOrderPreset};
        let mut model = Model::new();
        #[cfg(not(target_arch = "wasm32"))]
        model.files.changes.disable_watcher();
        let config = wav::ReadConfigBytes::new(
            Some("mono.wav".into()),
            wav::channel_mask::tests::wav_bytes(None, 3, &[]),
        )
        .with_ch_ixs([0, 2]);
        let id = model
            .add_loaded_file(wav::read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap())
            .unwrap();
        let track = model
            .find_track_id_for_buffer(model.files[id].channels[&0].buffer_id)
            .unwrap();
        let metadata = metadata_rows(&model.files[id]);
        assert!(model.files[id].layout.is_none());
        assert!(metadata.contains(&("channel order", "Auro ID order (assumed)".into())));
        assert_eq!(
            super::super::track::track_label(&model, track),
            "0 L mono.wav"
        );
        Action::SetFileChannelInterpretation {
            file_id: id,
            interpretation: ChannelInterpretation {
                preset: ChannelOrderPreset::Automatic,
                active_layout: Some(audio::Layout::LAYOUT_1_0),
            },
        }
        .process(&mut model)
        .unwrap();
        let file = &model.files[id];
        let rows = channel_rows(&model, file);
        assert_eq!(rows[0].label, "ch 0 - Left (padding)");
        assert_eq!(rows[1].label, "ch 1 - Right (padding)");
        assert!(rows[1].buffer_id.is_none());
        assert_eq!(rows[2].label, "ch 2 - Center");
        let metadata = metadata_rows(file);
        assert_eq!(file.layout, Some(audio::Layout::LAYOUT_1_0));
        assert!(metadata.contains(&("layout source", "selected".into())));
        assert!(metadata.contains(&("WAVEX mask", "absent".into())));
        assert_eq!(
            super::super::track::track_label(&model, track),
            "0 L* mono.wav"
        );
        Action::SetFileChannelInterpretation {
            file_id: id,
            interpretation: ChannelInterpretation {
                preset: ChannelOrderPreset::Unassigned,
                active_layout: Some(audio::Layout::LAYOUT_1_0),
            },
        }
        .process(&mut model)
        .unwrap();
        assert_eq!(channel_rows(&model, &model.files[id])[0].label, "ch 0");
        assert_eq!(
            super::super::track::track_label(&model, track),
            "0 mono.wav"
        );
        assert!(model.files[id].layout.is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn notification_controls_enqueue_single_and_batch_actions() {
        let mut model = Model::new();
        model.files.changes.disable_watcher();
        let buffer = crate::model::test_support::add_buffer(&mut model);
        let mut file = crate::model::test_support::make_file(&[buffer]);
        let path =
            crate::model::files::disk::absolute_path(std::path::Path::new("target/ui-reload.wav"));
        file.path = Some(path.clone());
        file.source = Some(Box::new(wav::ReadConfig::new(&path)));
        let id = model.files.insert(file);
        let context = egui::Context::default();
        let frame = |model: &mut Model, events| {
            context.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(500.0, 500.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| changed_files_ui(ui, model));
                },
            )
        };
        fn labels(output: &egui::FullOutput) -> Vec<(String, egui::Pos2)> {
            fn collect(shape: &egui::Shape, labels: &mut Vec<(String, egui::Pos2)>) {
                match shape {
                    egui::Shape::Text(text) => {
                        labels.push((text.galley.text().to_owned(), text.pos))
                    }
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            collect(shape, labels);
                        }
                    }
                    _ => {}
                }
            }
            let mut labels = Vec::new();
            for shape in &output.shapes {
                collect(&shape.shape, &mut labels);
            }
            labels
        }
        assert!(
            !labels(&frame(&mut model, vec![]))
                .iter()
                .any(|(label, _)| label.starts_with("Changed files"))
        );
        let change = model.files.changes.paths.get_mut(&path).unwrap();
        change.changed = true;
        change.revision = 1;
        let _ = frame(&mut model, vec![]);
        for button in ["Update", "Update all"] {
            let output = frame(&mut model, vec![]);
            let position = labels(&output)
                .into_iter()
                .find(|(label, _)| label == button)
                .unwrap()
                .1
                + egui::vec2(2.0, 2.0);
            let _ = frame(&mut model, vec![egui::Event::PointerMoved(position)]);
            let _ = frame(
                &mut model,
                vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                }],
            );
            let _ = frame(
                &mut model,
                vec![egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                }],
            );
            assert!(
                matches!(model.actions.pop(), Some(Action::ReloadFiles(ids)) if ids == vec![id])
            );
        }
        model.files.reload.active_job = Some(1);
        assert!(
            labels(&frame(&mut model, vec![]))
                .iter()
                .any(|(label, _)| label == "Updating files and diffs…")
        );
        let hover = context.run(Default::default(), |ctx| {
            egui::CentralPanel::default()
                .show(ctx, |ui| affected_tracks_hover(ui, &model, id, &path, &[]));
        });
        assert!(
            labels(&hover)
                .iter()
                .any(|(label, _)| label.contains("automatic alignment is not rerun"))
        );
    }
}
