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
    let mut ok = false;
    let mut cancel = false;
    egui::Window::new("Pair channels for diff")
        .id(egui::Id::new("diff_pairing_window_v3"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Grid::new("diff_pairing_files")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("");
                    ui.label("path");
                    ui.label("offset").on_hover_text(OFFSET_HOVER_TEXT);
                    ui.end_row();

                    ui.label("A");
                    ui.label(pending.file_a.filepath.display().to_string());
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
                    ui.label(pending.file_b.filepath.display().to_string());
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
