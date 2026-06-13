use crate::model::{self, Action};
use crate::wav;

/// Chooser dialog shown when exactly two files are dropped: diff the two files, or load them both
/// as separate tracks. Shown while `model.pending_drop_choice` is `Some`.
pub fn ui_modal(ctx: &egui::Context, model: &mut model::Model) {
    let Some(pending) = model.pending_drop_choice.as_ref() else {
        return;
    };
    let path_a = pending.path_a.clone();
    let path_b = pending.path_b.clone();

    let mut diff = false;
    let mut load_both = false;
    let mut cancel = false;
    egui::Window::new("Two files dropped")
        .id(egui::Id::new("drop_choice_window_v1"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(format!("A: {}", path_a.display()));
            ui.label(format!("B: {}", path_b.display()));
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Diff").clicked() {
                    diff = true;
                }
                if ui.button("Load both").clicked() {
                    load_both = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });

    if diff {
        model.actions.push(Action::OpenDiffFilePaths {
            file_a: wav::ReadConfig::new(path_a),
            file_b: wav::ReadConfig::new(path_b),
        });
    } else if load_both {
        model
            .actions
            .push(Action::OpenFilePath(wav::ReadConfig::new(path_a)));
        model
            .actions
            .push(Action::OpenFilePath(wav::ReadConfig::new(path_b)));
    }
    if diff || load_both || cancel {
        model.pending_drop_choice = None;
    }
}
