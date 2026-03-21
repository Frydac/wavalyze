use crate::model::{self, shortcuts::ShortcutScope};
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
        ui.group(|ui| {
            ui.label("Shortcuts");
            ui.separator();
            for scope in ShortcutScope::ALL {
                ui.label(scope.label());
                egui::Grid::new(ui.id().with(("shortcuts_grid", scope)))
                    .num_columns(2)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        for binding in &config.shortcuts.bindings {
                            if binding.scope != scope {
                                continue;
                            }
                            ui.label(binding.action.label());
                            ui.monospace(binding.formatted(ui.ctx()));
                            ui.end_row();
                        }
                    });
                ui.add_space(4.0);
            }
            if ui.button("Reset shortcuts").clicked() {
                config.reset_shortcuts_to_default();
                #[cfg(not(target_arch = "wasm32"))]
                config.save_to_storage();
            }
        });
        {
            ui.group(|ui| {
                ui.label("Tracks");
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Min Height: ");
                    ui.add(
                        egui::DragValue::new(&mut config.track.min_height)
                            .speed(0.1)
                            .range(10.0..=200.0)
                            .prefix(""),
                    );
                });
            });
        }
        ui.separator();
        if ui.button("Reset all settings").clicked() {
            config.reset_to_default();
            #[cfg(not(target_arch = "wasm32"))]
            config.save_to_storage();
        }
    });
}
