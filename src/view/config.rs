use crate::model;
use egui;

pub fn show_config(ui: &mut egui::Ui, config: &mut model::Config) {
    ui.heading("Configuration");
    ui.separator();
    ui.add(
        egui::DragValue::new(&mut config.zoom_x_factor)
            .speed(0.1)
            .range(0.1..=10.0)
            .prefix("Zoom X Factor: "),
    );
}
