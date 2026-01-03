use crate::model;
use egui;

pub fn show_config(ui: &mut egui::Ui, config: &mut model::Config) {
    ui.group(|ui| {
        ui.heading("Settings");
        ui.separator();
        ui.horizontal(|ui| {
            ui.label("Zoom X Factor: ");
            ui.add(
                egui::DragValue::new(&mut config.zoom_x_scroll_factor)
                    .speed(0.1)
                    .range(0.1..=10.0)
                    .prefix(""),
            );
        });
        ui.checkbox(&mut config.show_hover_info, "Show floating hover info");
    });
}
