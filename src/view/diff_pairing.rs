use crate::{
    model::{self, Action},
    wav::channel_interpretation::ResolvedChannelInterpretation,
};

const OFFSET_HOVER_TEXT: &str = "Absolute sample offset for all channels in file.\nPositive value means we start from that positive value.";
const FIRST_NON_ZERO_HOVER_TEXT: &str = "Align first non-zero sample to timeline sample zero";
const LAST_LEADING_ZERO_HOVER_TEXT: &str =
    "Align last leading zero before first non-zero to timeline sample zero";

/// Channel-pairing matrix dialog for diffing two files: rows are file A channels, columns are
/// file B channels, one checkbox per cell. Shown while `model.pending_diff_pairing` is `Some`.
pub fn ui_modal(ctx: &egui::Context, model: &mut model::Model) {
    let Some(pending) = model.pending_diff_pairing.as_mut() else {
        return;
    };
    // Leave room for the row labels, offset and order controls, and window margins.
    let path_width = (ctx.content_rect().width() - 440.0).clamp(80.0, 480.0);
    let mut ok = false;
    let mut cancel = false;
    egui::Window::new("Pair channels for diff")
        .id(egui::Id::new("diff_pairing_window_v3"))
        .collapsible(false)
        .auto_sized()
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Avoid retaining an old window width through separators and horizontal rows.
            ui.set_max_width(path_width + 400.0);
            egui::Grid::new("diff_pairing_files")
                .num_columns(4)
                .show(ui, |ui| {
                    ui.label("");
                    ui.label("path");
                    ui.label("offset").on_hover_text(OFFSET_HOVER_TEXT);
                    ui.label("channel order");
                    ui.end_row();

                    ui.label("A");
                    path_label(ui, &pending.file_a.filepath, path_width);
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            pending.offset_detection_a.is_none(),
                            egui::DragValue::new(&mut pending.file_a.sample_ix_offset)
                                .speed(1.0)
                                .suffix(" samples"),
                        )
                        .on_hover_text(OFFSET_HOVER_TEXT);
                        let first_non_zero = model::jobs::OffsetDetectionMode::FirstNonZero;
                        if ui
                            .selectable_label(
                                pending.offset_detection_a == Some(first_non_zero),
                                "≠",
                            )
                            .on_hover_text(FIRST_NON_ZERO_HOVER_TEXT)
                            .clicked()
                        {
                            pending.toggle_offset_detection_a(first_non_zero);
                        }
                        let last_zero = model::jobs::OffsetDetectionMode::LastLeadingZero;
                        if ui
                            .selectable_label(pending.offset_detection_a == Some(last_zero), "0")
                            .on_hover_text(LAST_LEADING_ZERO_HOVER_TEXT)
                            .clicked()
                        {
                            pending.toggle_offset_detection_a(last_zero);
                        }
                    });
                    ui.push_id("interpretation_a", |ui| {
                        let interpretation = &mut pending.file_a.channel_interpretation;
                        let response = super::channel_interpretation::order_selector(
                            ui,
                            &mut interpretation.preset,
                        );
                        let resolved = interpretation.resolve_and_validate(
                            pending.metadata_a.channel_mask,
                            pending.metadata_a.total_nr_channels,
                        );
                        response.on_hover_text(super::channel_interpretation::source_label(
                            resolved.source,
                        ));
                    });
                    ui.end_row();

                    ui.label("B");
                    path_label(ui, &pending.file_b.filepath, path_width);
                    ui.horizontal(|ui| {
                        ui.add_enabled(
                            pending.offset_detection_b.is_none(),
                            egui::DragValue::new(&mut pending.file_b.sample_ix_offset)
                                .speed(1.0)
                                .suffix(" samples"),
                        )
                        .on_hover_text(OFFSET_HOVER_TEXT);
                        let first_non_zero = model::jobs::OffsetDetectionMode::FirstNonZero;
                        if ui
                            .selectable_label(
                                pending.offset_detection_b == Some(first_non_zero),
                                "≠",
                            )
                            .on_hover_text(FIRST_NON_ZERO_HOVER_TEXT)
                            .clicked()
                        {
                            pending.toggle_offset_detection_b(first_non_zero);
                        }
                        let last_zero = model::jobs::OffsetDetectionMode::LastLeadingZero;
                        if ui
                            .selectable_label(pending.offset_detection_b == Some(last_zero), "0")
                            .on_hover_text(LAST_LEADING_ZERO_HOVER_TEXT)
                            .clicked()
                        {
                            pending.toggle_offset_detection_b(last_zero);
                        }
                    });
                    ui.push_id("interpretation_b", |ui| {
                        let interpretation = &mut pending.file_b.channel_interpretation;
                        let response = super::channel_interpretation::order_selector(
                            ui,
                            &mut interpretation.preset,
                        );
                        let resolved = interpretation.resolve_and_validate(
                            pending.metadata_b.channel_mask,
                            pending.metadata_b.total_nr_channels,
                        );
                        response.on_hover_text(super::channel_interpretation::source_label(
                            resolved.source,
                        ));
                    });
                    ui.end_row();
                });
            if pending.existing_files.is_some() {
                ui.small("Comparing loaded audio; only loaded channels are available.");
            }
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Clear").clicked() {
                    pending.clear();
                }
                if ui.button("Reset").clicked() {
                    pending.set_default_pairing();
                }
            });
            ui.separator();
            let roles_a = pending.file_a.channel_interpretation.resolve(
                pending.metadata_a.channel_mask,
                pending.metadata_a.total_nr_channels,
            );
            let roles_b = pending.file_b.channel_interpretation.resolve(
                pending.metadata_b.channel_mask,
                pending.metadata_b.total_nr_channels,
            );
            egui::ScrollArea::horizontal()
                .id_salt("diff_pairing_matrix_scroll")
                .auto_shrink([true, true])
                .show(ui, |ui| {
                    pairing_matrix(ui, pending, &roles_a, &roles_b);
                });
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(pending.any_checked(), egui::Button::new("OK"))
                    .clicked()
                {
                    ok = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
    if ok {
        model.actions.push(Action::ConfirmDiffPairing);
    }
    if cancel {
        model.actions.push(Action::CancelDiffPairing);
    }
}

fn pairing_matrix(
    ui: &mut egui::Ui,
    pending: &mut model::diff_pairing::PendingDiffPairing,
    roles_a: &ResolvedChannelInterpretation,
    roles_b: &ResolvedChannelInterpretation,
) -> (Vec<egui::Rect>, Vec<egui::Rect>) {
    // Reserve background shapes before drawing labels, then fill the hovered
    // axes after drawing the cells. This highlights both labels in this frame.
    let mut row_highlights = Vec::new();
    let mut column_highlight = None;
    let mut rows = Vec::new();
    let mut columns = Vec::new();
    let mut hovered = None;
    egui::Grid::new("diff_pairing_matrix")
        .striped(true)
        .spacing(egui::Vec2::ZERO)
        .min_col_width(0.0)
        .show(ui, |ui| {
            ui.label("A\\B  ")
                .on_hover_text("Rows: file A channels. Columns: file B channels.");
            column_highlight = Some(ui.painter().add(egui::Shape::Noop));
            for ch_b in &pending.ch_ixs_b {
                columns.push(channel_heading(ui, "B", *ch_b, roles_b, true).rect);
            }
            ui.end_row();
            for (row, ch_a) in pending.ch_ixs_a.iter().enumerate() {
                row_highlights.push(ui.painter().add(egui::Shape::Noop));
                rows.push(channel_heading(ui, "A", *ch_a, roles_a, false).rect);
                for (column, checked) in pending.checked[row].iter_mut().enumerate() {
                    let response = ui.checkbox(checked, "");
                    if response.hovered() {
                        hovered = Some((row, column));
                    }
                    response.on_hover_ui(|ui| {
                        ui.label(format!(
                            "Compare {}\nwith {}",
                            comparison_channel("A", *ch_a, roles_a),
                            comparison_channel("B", pending.ch_ixs_b[column], roles_b),
                        ));
                    });
                }
                ui.end_row();
            }
        });
    if let Some((row, column)) = hovered {
        let color = ui.visuals().selection.bg_fill;
        ui.painter().set(
            row_highlights[row],
            egui::Shape::rect_filled(rows[row], 2.0, color),
        );
        ui.painter().set(
            column_highlight.expect("column highlight reserved while drawing grid"),
            egui::Shape::rect_filled(columns[column], 2.0, color),
        );
    }
    (rows, columns)
}

fn comparison_channel(
    input: &str,
    index: usize,
    interpretation: &ResolvedChannelInterpretation,
) -> String {
    let channel = interpretation.channels.get(index);
    let role = channel
        .and_then(|channel| channel.id)
        .map(|id| format!("{} — {}", id.short_name(), id.long_name()));
    let mut text = format!("File {input}, WAV channel {index}");
    if let Some(role) = role {
        text.push_str(&format!(" ({role})"));
    }
    if channel.is_some_and(|channel| channel.padding) {
        text.push_str(" — expected padding");
    }
    text
}

fn channel_heading(
    ui: &mut egui::Ui,
    input: &str,
    index: usize,
    interpretation: &ResolvedChannelInterpretation,
    stacked: bool,
) -> egui::Response {
    let channel = interpretation.channels.get(index);
    let id = channel.and_then(|channel| channel.id);
    let padding = channel.is_some_and(|channel| channel.padding);
    let short = id.map(|id| id.short_name()).unwrap_or("");
    let marker = if padding { "*" } else { "" };
    let separator = if stacked { "\n" } else { " " };
    let text = if id.is_some() {
        format!("{index}{separator}{short}{marker}")
    } else {
        index.to_string()
    };
    let role = id
        .as_ref()
        .map(|id| id.long_name())
        .unwrap_or("Role unassigned");
    let mut hover = format!(
        "File {input}, WAV channel {index}\n{role}\nOrder: {}",
        super::channel_interpretation::source_label(interpretation.source)
    );
    if padding {
        hover.push_str("\nExpected padding for the selected layout; silence has not been checked.");
    }
    let response = if stacked {
        // Match an empty checkbox's compact cell width. Header text must never
        // enlarge the grid columns or paint into a neighboring channel.
        let mut font = egui::TextStyle::Small.resolve(ui.style());
        font.size = font.size.min(10.0);
        let height = ui.fonts_mut(|fonts| fonts.row_height(&font)) * 2.0;
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(ui.spacing().interact_size.y, height),
            egui::Sense::hover(),
        );
        let mut job =
            egui::text::LayoutJob::simple(text, font, ui.visuals().text_color(), f32::INFINITY);
        job.halign = egui::Align::Center;
        let galley = ui.painter().layout_job(job);
        ui.painter()
            .with_clip_rect(ui.clip_rect().intersect(rect.shrink2(egui::vec2(1.0, 0.0))))
            .galley(rect.center_top(), galley, ui.visuals().text_color());
        response
    } else {
        ui.add(egui::Label::new(text).extend())
    };
    response.on_hover_text(hover)
}

fn path_label(ui: &mut egui::Ui, path: &std::path::Path, width: f32) {
    let full_path = path.display().to_string();
    let text = super::util::truncate_path_keep_basename_to_width(ui, &full_path, "", width);
    ui.add_sized(
        [width, ui.spacing().interact_size.y],
        egui::Label::new(text).truncate(),
    )
    .on_hover_text(full_path);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(output: &egui::FullOutput) -> Vec<String> {
        fn collect(shape: &egui::Shape, labels: &mut Vec<String>) {
            match shape {
                egui::Shape::Text(text) => labels.push(text.galley.text().to_owned()),
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

    #[test]
    fn matrix_roles_follow_each_order_and_keep_original_indices_and_pairs() {
        use crate::wav::channel_interpretation::ChannelOrderPreset;
        let ctx = egui::Context::default();
        let mut model = model::Model::new();
        #[cfg(not(target_arch = "wasm32"))]
        model.files.changes.disable_watcher();
        model.pending_diff_pairing = Some(model::diff_pairing::PendingDiffPairing::new(
            crate::wav::ReadConfig::new("auro.wav"),
            crate::wav::ReadConfig::new("22_2.wav"),
            crate::wav::WavChannelMetadata {
                total_nr_channels: 24,
                channel_mask: None,
            },
            crate::wav::WavChannelMetadata {
                total_nr_channels: 24,
                channel_mask: Some(0xffffff),
            },
            vec![4, 15, 18, 22, 23],
            vec![4, 15, 18, 22, 23],
        ));
        let original = model.pending_diff_pairing.as_ref().unwrap().checked.clone();
        let frame = |model: &mut model::Model| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 900.0),
                    )),
                    ..Default::default()
                },
                |ctx| ui_modal(ctx, model),
            )
        };
        let _ = frame(&mut model);
        let output = frame(&mut model);
        let text = labels(&output);
        for expected in [
            "4 Ls", "15 HCs", "18 LC", "22 BR", "23 BC", "4\nLb", "15\nHLb", "18\nLFE2", "22\nBC",
            "23\nBR",
        ] {
            assert!(
                text.iter().any(|text| text == expected),
                "missing {expected:?}"
            );
        }
        model
            .pending_diff_pairing
            .as_mut()
            .unwrap()
            .file_b
            .channel_interpretation
            .preset = ChannelOrderPreset::AuroIdOrder;
        let _ = frame(&mut model);
        let text = labels(&frame(&mut model));
        for expected in ["4\nLs", "15\nHCs", "18\nLC", "22\nBR", "23\nBC"] {
            assert!(
                text.iter().any(|text| text == expected),
                "missing {expected:?}"
            );
        }
        model
            .pending_diff_pairing
            .as_mut()
            .unwrap()
            .file_b
            .channel_interpretation
            .preset = ChannelOrderPreset::Unassigned;
        let _ = frame(&mut model);
        let text = labels(&frame(&mut model));
        assert!(text.iter().any(|text| text == "4"));
        assert!(!text.iter().any(|text| text.contains('?')));
        assert!(text.iter().any(|text| text == "4 Ls"));
        assert_eq!(
            model.pending_diff_pairing.as_ref().unwrap().checked,
            original
        );
    }

    #[test]
    fn hovering_a_pair_identifies_both_channels_and_highlights_only_its_axes() {
        let ctx = egui::Context::default();
        let mut pending = model::diff_pairing::PendingDiffPairing::new(
            crate::wav::ReadConfig::new("auro.wav"),
            crate::wav::ReadConfig::new("wavex.wav"),
            crate::wav::WavChannelMetadata {
                total_nr_channels: 24,
                channel_mask: None,
            },
            crate::wav::WavChannelMetadata {
                total_nr_channels: 24,
                channel_mask: Some(0xffffff),
            },
            vec![4, 15],
            vec![18, 22],
        );
        let roles_a = pending.file_a.channel_interpretation.resolve(None, 24);
        let roles_b = pending
            .file_b
            .channel_interpretation
            .resolve(Some(0xffffff), 24);
        let original = pending.checked.clone();
        let mut frame = |time, events| {
            let mut headings = (Vec::new(), Vec::new());
            let output = ctx.run(
                egui::RawInput {
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        headings = pairing_matrix(ui, &mut pending, &roles_a, &roles_b);
                    });
                },
            );
            (output, headings)
        };
        let _ = frame(0.0, vec![]);
        let (_, (rows, columns)) = frame(0.01, vec![]);
        let pointer = egui::pos2(columns[1].center().x, rows[0].center().y);
        let (output, (rows, columns)) = frame(0.1, vec![egui::Event::PointerMoved(pointer)]);
        let highlights = |output: &egui::FullOutput| -> Vec<egui::Rect> {
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Rect(rect)
                        if rect.fill == ctx.style().visuals.selection.bg_fill =>
                    {
                        Some(rect.rect)
                    }
                    _ => None,
                })
                .collect()
        };
        assert_eq!(highlights(&output), vec![columns[1], rows[0]]);
        let _ = frame(1.1, vec![]);
        let (output, _) = frame(1.2, vec![]);
        let text = labels(&output);
        assert!(text.iter().any(|text| text.contains("Compare File A, WAV channel 4 (Ls — LeftSurround)\nwith File B, WAV channel 22 (BC — BottomCenter)")), "{text:?}");
        let (output, _) = frame(
            1.3,
            vec![egui::Event::PointerMoved(egui::pos2(500.0, 500.0))],
        );
        assert!(highlights(&output).is_empty());
        assert_eq!(pending.checked, original);
    }

    #[test]
    fn column_roles_do_not_widen_checkbox_cells_or_overlap_neighbors() {
        let ctx = egui::Context::default();
        let roles = crate::wav::channel_interpretation::ChannelInterpretation::default()
            .resolve(Some(0xffffff), 24);
        let mut headings = Vec::new();
        let mut cells = Vec::new();
        let mut bare_cell = egui::Rect::NOTHING;
        let mut output = egui::FullOutput::default();
        for _ in 0..2 {
            headings.clear();
            cells.clear();
            output = ctx.run(egui::RawInput::default(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    bare_cell = ui.checkbox(&mut false, "").rect;
                    egui::Grid::new("compact_headers")
                        .spacing(egui::Vec2::ZERO)
                        .min_col_width(0.0)
                        .show(ui, |ui| {
                            for index in [15, 18, 19] {
                                headings.push(channel_heading(ui, "B", index, &roles, true).rect);
                            }
                            ui.end_row();
                            for _ in 0..3 {
                                cells.push(ui.checkbox(&mut false, "").rect);
                            }
                        });
                });
            });
        }
        for (heading, cell) in headings.iter().zip(&cells) {
            assert!((cell.width() - bare_cell.width()).abs() < 0.1);
            assert!((heading.center().x - cell.center().x).abs() < 0.1);
        }
        for neighbors in cells.windows(2) {
            assert!((neighbors[1].left() - neighbors[0].right()).abs() < 0.1);
        }
        let header_text: Vec<_> = output
            .shapes
            .iter()
            .filter(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.text().contains('\n')))
            .collect();
        assert_eq!(header_text.len(), 3);
        for (shape, heading) in header_text.iter().zip(&headings) {
            assert!(shape.clip_rect.left() >= heading.left());
            assert!(shape.clip_rect.right() <= heading.right());
        }
    }

    #[test]
    fn matrix_headings_show_full_roles_and_expected_padding_on_hover() {
        use crate::wav::channel_interpretation::ChannelInterpretation;
        let mono = ChannelInterpretation {
            active_layout: Some(crate::audio::Layout::LAYOUT_1_0),
            ..Default::default()
        };
        for (resolved, index, visible, full_role) in [
            (
                ChannelInterpretation::default().resolve(Some(0xffffff), 24),
                15,
                "15\nHLb",
                "HeightLeftBack",
            ),
            (
                mono.resolve(None, 3),
                0,
                "0\nL*",
                "Expected padding for the selected layout",
            ),
        ] {
            let ctx = egui::Context::default();
            let frame = |time, events| {
                let mut rect = egui::Rect::NOTHING;
                let output = ctx.run(
                    egui::RawInput {
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            rect = channel_heading(ui, "B", index, &resolved, true).rect;
                        });
                    },
                );
                (output, rect)
            };
            let (output, rect) = frame(0.0, vec![]);
            assert!(labels(&output).iter().any(|text| text == visible));
            let _ = frame(0.1, vec![egui::Event::PointerMoved(rect.center())]);
            let _ = frame(1.1, vec![]);
            let (output, _) = frame(1.2, vec![]);
            assert!(
                labels(&output).iter().any(|text| text.contains(full_role)),
                "missing hover role {full_role:?}: {:?}",
                labels(&output)
            );
            assert!(
                labels(&output)
                    .iter()
                    .any(|text| text.contains(&format!("File B, WAV channel {index}")))
            );
        }
    }

    #[test]
    fn pairing_window_shrinks_after_full_paths_made_it_wide() {
        let ctx = egui::Context::default();
        let window_id = egui::Id::new("diff_pairing_window_v3");
        let input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(2400.0, 900.0),
            )),
            ..Default::default()
        };
        // Simulate the width retained from the old dialog with unshortened paths.
        for _ in 0..3 {
            let _ = ctx.run(input(), |ctx| {
                egui::Window::new("Pair channels for diff")
                    .id(window_id)
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.add(egui::Label::new("long-directory/".repeat(20)).extend());
                        ui.separator();
                    });
            });
        }
        let old_width = egui::containers::AreaState::load(&ctx, window_id)
            .unwrap()
            .rect()
            .width();
        assert!(old_width > 1000.0);

        let mut model = model::Model::new();
        #[cfg(not(target_arch = "wasm32"))]
        model.files.changes.disable_watcher();
        let directory = format!("/home/emile/{}", "long-directory/".repeat(20));
        model.pending_diff_pairing = Some(model::diff_pairing::PendingDiffPairing::new(
            crate::wav::ReadConfig::new(format!("{directory}reference.wav")),
            crate::wav::ReadConfig::new(format!("{directory}decode_cx_to_ac_orig_layout.wav")),
            crate::wav::WavChannelMetadata {
                total_nr_channels: 3,
                channel_mask: None,
            },
            crate::wav::WavChannelMetadata {
                total_nr_channels: 3,
                channel_mask: None,
            },
            vec![0, 1, 2],
            vec![0, 1, 2],
        ));
        for _ in 0..3 {
            let _ = ctx.run(input(), |ctx| ui_modal(ctx, &mut model));
        }
        let width = egui::containers::AreaState::load(&ctx, window_id)
            .unwrap()
            .rect()
            .width();
        assert!(width < 950.0, "pairing window remained {width} points wide");
    }
    #[test]
    fn changing_b_preset_keeps_a_layout_and_checked_pairs() {
        use crate::wav::channel_interpretation::{ChannelInterpretation, ChannelOrderPreset};
        let ctx = egui::Context::default();
        let mut model = model::Model::new();
        #[cfg(not(target_arch = "wasm32"))]
        model.files.changes.disable_watcher();
        let interpretation_a = ChannelInterpretation {
            preset: ChannelOrderPreset::Automatic,
            active_layout: Some(crate::audio::Layout::LAYOUT_1_0),
        };
        model.pending_diff_pairing = Some(model::diff_pairing::PendingDiffPairing::new(
            crate::wav::ReadConfig::new("a.wav")
                .with_channel_interpretation(interpretation_a.clone()),
            crate::wav::ReadConfig::new("b.wav"),
            crate::wav::WavChannelMetadata {
                total_nr_channels: 3,
                channel_mask: None,
            },
            crate::wav::WavChannelMetadata {
                total_nr_channels: 6,
                channel_mask: Some(0x3f),
            },
            vec![0, 2],
            vec![4, 5],
        ));
        let original_pairs = model
            .pending_diff_pairing
            .as_ref()
            .unwrap()
            .selected_pairs();
        fn centers(output: &egui::FullOutput, label: &str) -> Vec<egui::Pos2> {
            fn collect(shape: &egui::Shape, label: &str, positions: &mut Vec<egui::Pos2>) {
                match shape {
                    egui::Shape::Text(text) if text.galley.text() == label => {
                        positions.push(text.pos + text.galley.size() / 2.0)
                    }
                    egui::Shape::Vec(shapes) => {
                        for shape in shapes {
                            collect(shape, label, positions);
                        }
                    }
                    _ => {}
                }
            }
            let mut positions = Vec::new();
            for shape in &output.shapes {
                collect(&shape.shape, label, &mut positions);
            }
            positions.sort_by(|a, b| a.y.total_cmp(&b.y));
            positions
        }
        let mut frame = |events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1200.0, 900.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| ui_modal(ctx, &mut model),
            )
        };
        let click = |position| {
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ]
        };
        let _ = frame(vec![]);
        let output = frame(vec![]);
        let automatic = centers(&output, "Automatic");
        assert_eq!(automatic.len(), 2);
        let offset_header = centers(&output, "offset")[0];
        let order_header = centers(&output, "channel order")[0];
        assert!(order_header.x > offset_header.x);
        let path_a = centers(&output, "a.wav")[0];
        let path_b = centers(&output, "b.wav")[0];
        assert!(automatic[0].x > path_a.x && automatic[1].x > path_b.x);
        assert!((automatic[0].y - path_a.y).abs() < 4.0);
        assert!((automatic[1].y - path_b.y).abs() < 4.0);
        assert!(centers(&output, "Active layout:").is_empty());
        assert!(centers(&output, "1.0 / TestMix1.0").is_empty());
        let _ = frame(click(automatic[1]));
        let output = frame(vec![]);
        let unassigned = centers(&output, "Unassigned");
        assert_eq!(unassigned.len(), 1);
        let _ = frame(click(unassigned[0]));
        let pending = model.pending_diff_pairing.as_ref().unwrap();
        assert_eq!(pending.file_a.channel_interpretation, interpretation_a);
        assert_eq!(
            pending.file_b.channel_interpretation.preset,
            ChannelOrderPreset::Unassigned
        );
        assert_eq!(pending.selected_pairs(), original_pairs);
        assert_eq!(pending.ch_ixs_a, [0, 2]);
        assert_eq!(pending.ch_ixs_b, [4, 5]);
    }
}
