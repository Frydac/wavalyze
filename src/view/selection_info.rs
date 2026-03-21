use crate::{
    model::{
        self, Action,
        config::StartEditMode,
        selection_info::{SelectionInfo, SelectionInfoE},
    },
    widgets::DigitwiseNumberEditor,
};

const SELECTION_EDITOR_DIGITS: usize = 9;
const SELECTION_EDITOR_MAX: u64 = 999_999_999;
const SELECTION_EDITOR_DIGIT_WIDTH: f32 = 12.0;

pub fn ui_selection_info_side_panel(ui: &mut egui::Ui, selection_info: &mut SelectionInfoE) {
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.heading("Selection Info");
            ui.separator();
            match selection_info {
                SelectionInfoE::NotSelected => {
                    ui.label("No selection");
                }
                SelectionInfoE::IsSelected(selection_info) => {
                    ui.label(format!(
                        "ix range: [{}, {}]",
                        selection_info.ix_rng.start,
                        selection_info.ix_rng.end - 1
                    ));
                    ui.label(format!("length: {}", selection_info.ix_rng.len()));
                    // ui.label(format!("pos x range: {:?}", selection_info.screen_x_start..=selection_info.screen_x_end));
                    // let mut value = selection_info.ix_rng.start as u64;
                    // let output = DigitwiseNumberEditor::new("selection_start", &mut value)
                    //     .digits(9)
                    //     .max(999999999)
                    //     .show(ui);
                    // if output.changed {
                    //     selection_info.ix_rng.start = value as i64;
                    //     // model
                    // }
                }
            }
        });
    });
}

pub fn ui_selection_info_toolbar(
    ui: &mut egui::Ui,
    config: &mut model::Config,
    selection_info: SelectionInfoE,
    actions: &mut Vec<Action>,
) {
    let has_selection = selection_info.is_selected();
    let (had_selection, screen_x_start, screen_x_end, mut start_val, mut length_val, mut end_val) =
        match selection_info {
            SelectionInfoE::NotSelected => (false, 0.0, 0.0, 0, 0, 0),
            SelectionInfoE::IsSelected(selection_info) => {
                let start_ix = selection_info.ix_rng.start.max(0) as u64;
                let end_exclusive_ix = selection_info
                    .ix_rng
                    .end
                    .max(selection_info.ix_rng.start.saturating_add(1))
                    as u64;
                let start_val = start_ix.min(SELECTION_EDITOR_MAX);
                let end_val = end_exclusive_ix.saturating_sub(1).min(SELECTION_EDITOR_MAX);
                let length_val = end_val.saturating_sub(start_val).saturating_add(1);
                (
                    true,
                    selection_info.screen_x_start,
                    selection_info.screen_x_end,
                    start_val,
                    length_val,
                    end_val,
                )
            }
        };

    ui.group(|ui| {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.vertical(|ui| {
                ui.heading("Selection");
                ui.menu_button("⚙", |ui| {
                    ui.label("Start edit");
                    ui.radio_value(
                        &mut config.selection.start_edit_mode,
                        StartEditMode::KeepEnd,
                        "keep end",
                    );
                    ui.radio_value(
                        &mut config.selection.start_edit_mode,
                        StartEditMode::KeepLength,
                        "keep length",
                    );
                });
                let zoom_button = ui
                    .add_enabled(has_selection, egui::Button::new("🔍▭"))
                    .on_hover_text("Zoom to selection");
                if zoom_button.clicked() {
                    actions.push(Action::ZoomToSelection);
                }
            });

            egui::Grid::new(ui.id().with("selection_toolbar_grid"))
                .striped(true)
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("start");
                    let out_start = DigitwiseNumberEditor::new("selection_start", &mut start_val)
                        .digits(SELECTION_EDITOR_DIGITS)
                        .digit_width(SELECTION_EDITOR_DIGIT_WIDTH)
                        .max(SELECTION_EDITOR_MAX)
                        .show(ui);
                    ui.end_row();

                    ui.label("length");
                    let out_length =
                        DigitwiseNumberEditor::new("selection_length", &mut length_val)
                            .digits(SELECTION_EDITOR_DIGITS)
                            .digit_width(SELECTION_EDITOR_DIGIT_WIDTH)
                            .max(SELECTION_EDITOR_MAX)
                            .show(ui);
                    ui.end_row();

                    ui.label("end");
                    let out_end = DigitwiseNumberEditor::new("selection_end", &mut end_val)
                        .digits(SELECTION_EDITOR_DIGITS)
                        .digit_width(SELECTION_EDITOR_DIGIT_WIDTH)
                        .max(SELECTION_EDITOR_MAX)
                        .show(ui);
                    ui.end_row();

                    let any_changed = out_start.changed || out_length.changed || out_end.changed;
                    if !any_changed {
                        return;
                    }

                    if !had_selection {
                        if out_start.changed {
                            end_val = start_val;
                            length_val = 1;
                        }
                        if out_length.changed {
                            length_val = length_val.max(1);
                            let desired_end =
                                start_val.saturating_add(length_val.saturating_sub(1));
                            end_val = desired_end.min(SELECTION_EDITOR_MAX);
                            length_val = end_val.saturating_sub(start_val).saturating_add(1);
                        }
                        if out_end.changed {
                            if end_val < start_val {
                                end_val = start_val;
                            }
                            length_val = end_val.saturating_sub(start_val).saturating_add(1);
                        }
                    } else {
                        if out_start.changed {
                            match config.selection.start_edit_mode {
                                StartEditMode::KeepEnd => {
                                    if start_val > end_val {
                                        start_val = end_val;
                                    }
                                    length_val =
                                        end_val.saturating_sub(start_val).saturating_add(1);
                                }
                                StartEditMode::KeepLength => {
                                    let desired_end =
                                        start_val.saturating_add(length_val.saturating_sub(1));
                                    end_val = desired_end.min(SELECTION_EDITOR_MAX);
                                    length_val =
                                        end_val.saturating_sub(start_val).saturating_add(1);
                                }
                            }
                        }

                        if out_length.changed {
                            length_val = length_val.max(1);
                            let desired_end =
                                start_val.saturating_add(length_val.saturating_sub(1));
                            end_val = desired_end.min(SELECTION_EDITOR_MAX);
                            length_val = end_val.saturating_sub(start_val).saturating_add(1);
                        }

                        if out_end.changed {
                            if end_val < start_val {
                                end_val = start_val;
                            }
                            length_val = end_val.saturating_sub(start_val).saturating_add(1);
                        }
                    }

                    let new_selection_info = SelectionInfoE::IsSelected(SelectionInfo {
                        ix_rng: (start_val as i64..(end_val + 1) as i64).into(),
                        screen_x_start,
                        screen_x_end,
                    });
                    actions.push(Action::SetSelection(new_selection_info));
                });
        });
    });
}
