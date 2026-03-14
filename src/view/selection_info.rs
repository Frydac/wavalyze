use crate::model::selection_info::SelectionInfoE;

pub fn ui_selection_info_side_panel(ui: &mut egui::Ui, selection_info: &SelectionInfoE) {
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
                        selection_info.ix_rng.start, selection_info.ix_rng.end
                    ));
                    ui.label(format!("count: {}", selection_info.ix_rng.len()));
                    // ui.label(format!("pos x range: {:?}", selection_info.screen_x_start..=selection_info.screen_x_end));
                }
            }
        });
    });
}
