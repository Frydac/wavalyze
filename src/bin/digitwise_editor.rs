use eframe::egui;
use wavalyze::widgets::{
    DigitwiseNumberEditor, DigitwiseNumberEditorAction, DigitwiseNumberEditorOutput,
};

fn main() -> anyhow::Result<()> {
    wavalyze::log::init_tracing(Some("info"))?;

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 320.0])
            .with_min_inner_size([520.0, 240.0]),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "digitwise editor",
        native_options,
        Box::new(|_cc| Ok(Box::new(DemoApp::default()))),
    ) {
        anyhow::bail!("failed to start digitwise editor: {err}");
    }

    Ok(())
}

#[derive(Debug)]
struct DemoApp {
    selection_start: u64,
    selection_end: u64,
    max: u64,
    digits: usize,
    last_action: String,
}

impl Default for DemoApp {
    fn default() -> Self {
        Self {
            selection_start: 12_345,
            selection_end: 67_890,
            max: 999_999,
            digits: 6,
            last_action: "none".to_string(),
        }
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.clamp_demo_values();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Digitwise number editor");
                ui.separator();
                ui.label("Standalone playground for digit-focused numeric editing.");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.group(|ui| {
                    ui.heading("Editor Demo");
                    ui.add_space(6.0);

                    if let Some(last_action) = show_editor_row(
                        ui,
                        "Selection start",
                        "selection_start",
                        self.digits,
                        self.max,
                        &mut self.selection_start,
                    ) {
                        self.last_action = last_action;
                    }

                    if let Some(last_action) = show_editor_row(
                        ui,
                        "Selection end",
                        "selection_end",
                        self.digits,
                        self.max,
                        &mut self.selection_end,
                    ) {
                        self.last_action = last_action;
                    }
                });

                ui.add_space(10.0);

                ui.columns(2, |columns| {
                    columns[0].group(|ui| {
                        ui.heading("Config");
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            ui.label("Digits");
                            ui.add(egui::DragValue::new(&mut self.digits).range(1..=20));
                        });

                        ui.horizontal(|ui| {
                            ui.label("Max");
                            ui.add(egui::DragValue::new(&mut self.max).range(0..=u64::MAX));
                        });

                        ui.label(format!(
                            "Display range: 0..={}",
                            display_max_for_digits(self.digits)
                        ));
                    });

                    columns[1].group(|ui| {
                        ui.heading("Debug");
                        ui.add_space(6.0);
                        ui.monospace(format!("selection_start = {}", self.selection_start));
                        ui.monospace(format!("selection_end   = {}", self.selection_end));
                        ui.monospace(format!("selected width = {}", self.digits.clamp(1, 20)));
                        ui.monospace(format!("max            = {}", self.max));
                        ui.monospace(format!("last action    = {}", self.last_action));
                    });
                });
            });
        });
    }
}

impl DemoApp {
    fn clamp_demo_values(&mut self) {
        let digits = self.digits.clamp(1, 20);
        let max_for_digits = display_max_for_digits(digits);
        self.max = self.max.min(max_for_digits);
        self.selection_start = self.selection_start.min(self.max);
        self.selection_end = self.selection_end.min(self.max);
    }
}

fn show_editor_row(
    ui: &mut egui::Ui,
    label: &str,
    id: &str,
    digits: usize,
    max: u64,
    value: &mut u64,
) -> Option<String> {
    let mut last_action = None;

    ui.horizontal(|ui| {
        ui.label(label);

        let output = DigitwiseNumberEditor::new(id, value)
            .digits(digits)
            .max(max)
            .show(ui);

        if output.changed || output.action.is_some() {
            last_action = Some(describe_action(label, *value, &output));
        }
    });

    last_action
}

fn describe_action(label: &str, value: u64, output: &DigitwiseNumberEditorOutput) -> String {
    let action = match output.action {
        Some(DigitwiseNumberEditorAction::FocusDigit) => "focus",
        Some(DigitwiseNumberEditorAction::MoveLeft) => "left",
        Some(DigitwiseNumberEditorAction::MoveRight) => "right",
        Some(DigitwiseNumberEditorAction::ReplaceDigit) => "replace",
        Some(DigitwiseNumberEditorAction::IncrementPlace) => "increment",
        Some(DigitwiseNumberEditorAction::DecrementPlace) => "decrement",
        None => "none",
    };

    format!(
        "{label}: action={action}, selected_digit={}, value={value}",
        output.selected_digit
    )
}

fn display_max_for_digits(digits: usize) -> u64 {
    if digits >= 20 {
        u64::MAX
    } else {
        10_u64.pow(digits as u32) - 1
    }
}
