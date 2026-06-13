use crate::model::{self, Action};

/// Channel-pairing matrix dialog for diffing two files: rows are file A channels, columns are
/// file B channels, one checkbox per cell. Shown while `model.pending_diff_pairing` is `Some`.
pub fn ui_modal(ctx: &egui::Context, model: &mut model::Model) {
    let Some(pending) = model.pending_diff_pairing.as_mut() else {
        return;
    };
    let mut ok = false;
    let mut cancel = false;
    egui::Window::new("Pair channels for diff")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("A: {}", pending.file_a.filepath.display()));
            ui.label(format!("B: {}", pending.file_b.filepath.display()));
            ui.separator();
            egui::Grid::new("diff_pairing_matrix").show(ui, |ui| {
                ui.label("");
                for ch_b in &pending.ch_ixs_b {
                    ui.label(format!("B ch {ch_b}"));
                }
                ui.end_row();
                for (row, ch_a) in pending.ch_ixs_a.iter().enumerate() {
                    ui.label(format!("A ch {ch_a}"));
                    for checked in &mut pending.checked[row] {
                        ui.checkbox(checked, "");
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
