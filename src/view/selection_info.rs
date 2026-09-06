use crate::model::{
    self, Action,
    config::StartEditMode,
    selection_info::{SelectionInfo, SelectionInfoE},
};
use egui_custom_widgets::DigitwiseNumberEditor;

const SELECTION_EDITOR_DIGITS: usize = 9;
const SELECTION_EDITOR_MAX: u64 = 999_999_999;
const SELECTION_EDITOR_DIGIT_WIDTH: f32 = 12.0;

fn block_coordinates(samples: u64, block_size: u64) -> (u64, u64) {
    (samples / block_size, samples % block_size)
}

fn samples_from_block(block_ix: u64, offset: u64, block_size: u64) -> u64 {
    block_ix
        .saturating_mul(block_size)
        .saturating_add(offset)
        .min(SELECTION_EDITOR_MAX)
}

fn number_editor(ui: &mut egui::Ui, id: &str, value: &mut u64, max: u64) -> bool {
    DigitwiseNumberEditor::new(id, value)
        .digits(SELECTION_EDITOR_DIGITS)
        .digit_width(SELECTION_EDITOR_DIGIT_WIDTH)
        .dim_leading_zeroes(true)
        .max(max)
        .show(ui)
        .changed
}

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
    block_size: u64,
    actions: &mut Vec<Action>,
) {
    let block_size = block_size.max(1);
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
    let (mut start_block, mut start_offset) = block_coordinates(start_val, block_size);
    let (mut length_block, mut length_offset) = block_coordinates(length_val, block_size);
    let (mut end_block, mut end_offset) = block_coordinates(end_val, block_size);
    let max_block_ix = SELECTION_EDITOR_MAX / block_size;
    let max_offset = block_size.saturating_sub(1).min(SELECTION_EDITOR_MAX);

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
                .num_columns(4)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    ui.label("");
                    ui.label("samples");
                    ui.label("block index");
                    ui.label("block offset");
                    ui.end_row();

                    ui.label("start");
                    let start_sample_changed =
                        number_editor(ui, "selection_start", &mut start_val, SELECTION_EDITOR_MAX);
                    let start_block_changed =
                        number_editor(ui, "selection_start_block", &mut start_block, max_block_ix);
                    let start_offset_changed =
                        number_editor(ui, "selection_start_offset", &mut start_offset, max_offset);
                    ui.end_row();

                    ui.label("length");
                    let length_sample_changed = number_editor(
                        ui,
                        "selection_length",
                        &mut length_val,
                        SELECTION_EDITOR_MAX,
                    );
                    let length_block_changed = number_editor(
                        ui,
                        "selection_length_block",
                        &mut length_block,
                        max_block_ix,
                    );
                    let length_offset_changed = number_editor(
                        ui,
                        "selection_length_offset",
                        &mut length_offset,
                        max_offset,
                    );
                    ui.end_row();

                    ui.label("end");
                    let end_sample_changed =
                        number_editor(ui, "selection_end", &mut end_val, SELECTION_EDITOR_MAX);
                    let end_block_changed =
                        number_editor(ui, "selection_end_block", &mut end_block, max_block_ix);
                    let end_offset_changed =
                        number_editor(ui, "selection_end_offset", &mut end_offset, max_offset);
                    ui.end_row();

                    let start_changed =
                        start_sample_changed || start_block_changed || start_offset_changed;
                    let length_changed =
                        length_sample_changed || length_block_changed || length_offset_changed;
                    let end_changed = end_sample_changed || end_block_changed || end_offset_changed;
                    if !start_changed && !length_changed && !end_changed {
                        return;
                    }

                    if start_block_changed || start_offset_changed {
                        start_val = samples_from_block(start_block, start_offset, block_size);
                    }
                    if length_block_changed || length_offset_changed {
                        length_val = samples_from_block(length_block, length_offset, block_size);
                    }
                    if end_block_changed || end_offset_changed {
                        end_val = samples_from_block(end_block, end_offset, block_size);
                    }

                    if !had_selection {
                        if start_changed {
                            end_val = start_val;
                            length_val = 1;
                        }
                        if length_changed {
                            length_val = length_val.max(1);
                            let desired_end =
                                start_val.saturating_add(length_val.saturating_sub(1));
                            end_val = desired_end.min(SELECTION_EDITOR_MAX);
                            length_val = end_val.saturating_sub(start_val).saturating_add(1);
                        }
                        if end_changed {
                            if end_val < start_val {
                                end_val = start_val;
                            }
                            length_val = end_val.saturating_sub(start_val).saturating_add(1);
                        }
                    } else {
                        if start_changed {
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

                        if length_changed {
                            length_val = length_val.max(1);
                            let desired_end =
                                start_val.saturating_add(length_val.saturating_sub(1));
                            end_val = desired_end.min(SELECTION_EDITOR_MAX);
                            length_val = end_val.saturating_sub(start_val).saturating_add(1);
                        }

                        if end_changed {
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

#[cfg(test)]
mod tests {
    use super::{SELECTION_EDITOR_MAX, block_coordinates, samples_from_block};

    #[test]
    fn sample_block_round_trips() {
        for samples in [0, 1, 1_023, 1_024, 2_050, SELECTION_EDITOR_MAX] {
            let (block_ix, offset) = block_coordinates(samples, 1_024);
            assert_eq!(samples_from_block(block_ix, offset, 1_024), samples);
        }
    }

    #[test]
    fn exact_block_boundaries_have_zero_offset() {
        assert_eq!(block_coordinates(1_023, 1_024), (0, 1_023));
        assert_eq!(block_coordinates(1_024, 1_024), (1, 0));
        assert_eq!(block_coordinates(2_048, 1_024), (2, 0));
    }

    #[test]
    fn length_uses_block_quotient_and_remainder() {
        assert_eq!(block_coordinates(2_050, 1_024), (2, 2));
        assert_eq!(samples_from_block(2, 2, 1_024), 2_050);
    }

    #[test]
    fn block_conversion_clamps_to_selection_maximum() {
        assert_eq!(
            samples_from_block(u64::MAX, u64::MAX, 1_024),
            SELECTION_EDITOR_MAX
        );
    }
}
