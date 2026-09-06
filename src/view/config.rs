use crate::model::{self, shortcuts::ShortcutScope};
use egui;

pub fn show_config(ui: &mut egui::Ui, config: &mut model::Config) {
    ui.group(|ui| {
        ui.heading("Settings");
        ui.separator();
        ui.group(|ui| {
            ui.label("Navigation (scroll wheel)").on_hover_text(
                "Sensitivity and direction of scroll-wheel pan/zoom on the waveform. \
                     Does not affect mouse-drag panning.",
            );
            ui.separator();
            let nav = &mut config.navigation;
            // label, factor, invert, what the axis controls (used for both column tooltips)
            let axes: [(&str, &mut f32, &mut bool, &str); 4] = [
                (
                    "Pan X",
                    &mut nav.pan_x_factor,
                    &mut nav.invert_pan_x,
                    "panning left/right in time",
                ),
                (
                    "Pan Y",
                    &mut nav.pan_y_factor,
                    &mut nav.invert_pan_y,
                    "panning up/down in sample value",
                ),
                (
                    "Zoom X",
                    &mut nav.zoom_x_factor,
                    &mut nav.invert_zoom_x,
                    "zooming in/out in time",
                ),
                (
                    "Zoom Y",
                    &mut nav.zoom_y_factor,
                    &mut nav.invert_zoom_y,
                    "zooming in/out in sample value",
                ),
            ];
            egui::Grid::new(ui.id().with("navigation_grid"))
                .num_columns(3)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for (label, factor, invert, what) in axes {
                        ui.label(label).on_hover_text(what);
                        ui.add(
                            egui::DragValue::new(factor)
                                .speed(0.1)
                                .range(0.1..=10.0)
                                .prefix(""),
                        )
                        .on_hover_text(format!("Speed multiplier for {what}. Higher = faster."));
                        ui.checkbox(invert, "Invert")
                            .on_hover_text(format!("Reverse the scroll direction for {what}."));
                        ui.end_row();
                    }
                });
            ui.horizontal(|ui| {
                ui.label("Zoom Y zero deadzone").on_hover_text(
                    "Ruler height around sample value zero where Ctrl-scroll zoom stays anchored to zero. Set to 0 to disable.",
                );
                ui.add(
                    egui::DragValue::new(&mut nav.zoom_y_zero_deadzone_height)
                        .speed(1.0)
                        .range(0.0..=200.0)
                        .suffix(" px"),
                );
            });
        });
        ui.horizontal(|ui| {
            ui.label("Value Skew: ");
            ui.add(
                egui::Slider::new(
                    &mut config.value_display_scale.skew_factor,
                    0.0..=model::ruler::ValueDisplayScale::MAX_SKEW_FACTOR,
                )
                .step_by(0.01)
                .show_value(true),
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
                ui.checkbox(
                    &mut config.track.equal_height_layout_by_default,
                    "Start with even track heights",
                );
                ui.checkbox(&mut config.show_amplitude_ruler, "Show amplitude ruler");
                ui.checkbox(&mut config.show_db_ruler, "Show dB ruler");
                ui.checkbox(
                    &mut config.round_minmax_waveform_to_pixel_center,
                    "Round zoomed-out waveform columns",
                );
            });
        }
        ui.separator();
        if ui.button("Reset all settings").clicked() {
            config.reset_to_default();
            config.save_to_storage();
        }
    });
}
