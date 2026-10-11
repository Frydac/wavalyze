use crate::model::{self, Action};

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
    // Leave room for the row labels, offset controls, and window margins.
    let path_width = (ctx.content_rect().width() - 240.0).clamp(80.0, 480.0);
    let mut ok = false;
    let mut cancel = false;
    egui::Window::new("Pair channels for diff")
        .id(egui::Id::new("diff_pairing_window_v3"))
        .collapsible(false)
        .auto_sized()
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            // Avoid retaining an old window width through separators and horizontal rows.
            ui.set_max_width(path_width + 200.0);
            egui::Grid::new("diff_pairing_files")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("");
                    ui.label("path");
                    ui.label("offset").on_hover_text(OFFSET_HOVER_TEXT);
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
                    ui.end_row();
                });
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
            egui::Grid::new("diff_pairing_matrix")
                .striped(true)
                .spacing(egui::Vec2::splat(0.0))
                .min_col_width(0.0)
                .show(ui, |ui| {
                    ui.label("A\\B  ");
                    for ch_b in &pending.ch_ixs_b {
                        ui.label(format!("{ch_b}"));
                    }
                    ui.end_row();
                    for (row, ch_a) in pending.ch_ixs_a.iter().enumerate() {
                        ui.label(format!("{ch_a}"));
                        for checked in &mut pending.checked[row] {
                            ui.checkbox(checked, "");
                            // ui.label("x");
                        }
                        ui.end_row();
                    }
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
        assert!(width < 750.0, "pairing window remained {width} points wide");
    }
}
