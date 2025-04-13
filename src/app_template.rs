// TODO: remove, maybe has some things I still want to see

use eframe::egui;
use egui::{emath, vec2, Pos2, Rect, Stroke};
use std::f32::consts::PI;

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    label: String,

    num_points: usize,

    sine_period_points: usize,

    #[serde(skip)] // This how you opt-out of serialization of a field
    value: f32,

    mouse_pos: Option<Pos2>,

    #[serde(skip)]
    mouse_hover_info: MouseHover,

    #[serde(skip)]
    mouse_select: MouseSelect,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            label: "Hello World!".to_owned(),
            num_points: 10,
            sine_period_points: 10,
            value: 2.7,
            mouse_pos: None,
            mouse_hover_info: MouseHover::default(),
            mouse_select: MouseSelect::default(),
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

#[derive(Debug, Default)]
struct MouseSelect {
    pub drag_start: Option<Pos2>,
    pub drag_end: Option<Pos2>,
}

impl MouseSelect {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let response = ui.interact(ui.min_rect(), egui::Id::new("mouse_select"), egui::Sense::drag());
        if response.contains_pointer() {
            if let Some(id) = ui.ctx().drag_started_id() {
                println!("drag start_id {}: {:?}", "id", id);
                if let Some(pos) = ui.ctx().pointer_hover_pos() {
                    self.drag_start = Some(pos);
                    println!("{}: {:?}", "drag_start", self.drag_start);
                    self.drag_end = None;
                }
            }
            if let Some(id) = ui.ctx().drag_stopped_id() {
                println!("drag stop_id {}: {:?}", "id", id);
                if let Some(pos) = ui.ctx().pointer_hover_pos() {
                    self.drag_end = Some(pos);
                    println!("{}: {:?}", "drag_end", self.drag_end);
                }
            }
        }

        if let Some(drag_start) = self.drag_start {
            if let Some(drag_end) = self.drag_end {
                ui.painter().rect_filled(
                    Rect::from_two_pos(
                        egui::pos2(drag_start.x, ui.min_rect().top()),
                        egui::pos2(drag_end.x, ui.min_rect().bottom()),
                    ),
                    egui::Rounding::ZERO,
                    egui::Color32::from_rgba_unmultiplied(200, 200, 200, 50),
                );
            } else {
                if let Some(current_mouse_pos) = ui.ctx().pointer_hover_pos() {
                    ui.painter().rect_filled(
                        Rect::from_two_pos(
                            egui::pos2(drag_start.x, ui.min_rect().top()),
                            egui::pos2(current_mouse_pos.x, ui.min_rect().bottom()),
                        ),
                        egui::Rounding::ZERO,
                        egui::Color32::from_rgba_unmultiplied(200, 200, 200, 50),
                    );
                }
            }
        }
    }
}

#[derive(Debug)]
struct MouseHover {
    pub screen_pos: Pos2,
    stroke_vline: Stroke, // for vertical line where mouse pointer is
}

impl Default for MouseHover {
    fn default() -> Self {
        Self {
            screen_pos: Pos2::new(0.0, 0.0),
            stroke_vline: Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(200, 200, 200, 100)),
        }
    }
}

impl MouseHover {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let response = ui
            .interact(ui.min_rect(), egui::Id::new("mouse_hover"), egui::Sense::hover())
            .on_hover_cursor(egui::CursorIcon::None);
        if response.contains_pointer() {
            if let Some(pos) = ui.ctx().pointer_hover_pos() {
                self.screen_pos = pos;
                // println!("{}: {}", "self.pos", self.screen_pos);
            }

            // We only draw the line when we are in the rect
            let vline_start = egui::pos2(self.screen_pos.x, ui.min_rect().top());
            let vline_end = egui::pos2(self.screen_pos.x, ui.min_rect().bottom());
            ui.painter().line_segment([vline_start, vline_end], self.stroke_vline);
            // ui.painter().text(self.screen_pos, egui::Align2::LEFT_CENTER, "Testing", egui::FontId::default(), egui::Color32::WHITE);
        }
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.separator();

            ui.vertical(|ui| {
                ui.label("My Canvas");
                let width = ui.available_width() - 10.0;
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    ui.set_min_size(egui::vec2(width, 100.0));

                    // This gets the absolute position of the canvas
                    let rect = ui.min_rect();
                    let painter = ui.painter_at(rect);

                    self.mouse_hover_info.ui(ui);
                    self.mouse_select.ui(ui);

                    let to_screen = emath::RectTransform::from_to(
                        // top-left corner + size
                        // Rect::from_min_size(Pos2::ZERO, vec2(self.num_points as f32,rect.size().y)),
                        Rect::from_min_size(egui::pos2(0.0, 1.5), vec2(self.num_points as f32, -3.0)),
                        rect,
                    );

                    // let to_local = to_screen.inverse();

                    let points: Vec<egui::Pos2> = (0..self.num_points)
                        .map(|i| {
                            let x = i as f32;
                            let y = ((i as f32 / self.sine_period_points as f32) * 2.0 * PI).sin();
                            let point_in_buffer = egui::pos2(x, y);
                            let point_in_screen = to_screen.transform_pos(point_in_buffer);
                            point_in_screen
                        })
                        .collect();

                    // Use the painter to draw all points as a single line.
                    painter.add(egui::Shape::line(
                        points,
                        egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE), // Line thickness and color
                    ));

                    {
                        // let stroke = Stroke::new(2.0, egui::Color32::LIGHT_GREEN);
                        let test_point = egui::pos2(100.0, 0.0);
                        let test_point = to_screen.transform_pos(test_point);
                        // let circle = Shape::circle_stroke(test_point, 2.0, stroke);
                        // painter.add(circle);
                        painter.circle_filled(test_point, 1.0, egui::Color32::LIGHT_GREEN);

                        // let available_size = ui.available_size();
                        let canvas_size = ui.min_size();
                        let (rect, _response) = ui.allocate_exact_size(canvas_size, egui::Sense::hover());
                        if ui.rect_contains_pointer(rect) {
                            ui.ctx().input(|i| {
                                for event in &i.events {
                                    match event {
                                        egui::Event::PointerMoved(pointer_moved_pos) => {
                                            // dbg!(pointer_moved_pos);
                                            self.mouse_pos = Some(pointer_moved_pos.clone());
                                        }
                                        _ => (),
                                    }
                                }
                            });
                        }
                    }

                    // draw middle line
                    {
                        let start = egui::Pos2::new(0.0, 0.0);
                        let start = to_screen.transform_pos(start);
                        let end = egui::Pos2::new(self.num_points as f32, 0.0);
                        let end = to_screen.transform_pos(end);

                        let color_line = egui::Color32::from_rgba_unmultiplied(220, 220, 220, 50);
                        painter.add(egui::Shape::line(
                            vec![start, end],
                            egui::Stroke::new(1.0, color_line), // Line thickness and color
                        ))
                    }

                    // let available_size = ui.available_size();
                    // let (rect, _response) =
                    //     ui.allocate_exact_size(available_size, egui::Sense::hover());
                });

                ui.label("My 2nd Canvas");
                egui::Frame::canvas(ui.style()).show(ui, |ui| {
                    ui.set_min_size(egui::vec2(width, 100.0));

                    // This gets the absolute position of the canvas
                    let rect = ui.min_rect();
                    let painter = ui.painter_at(rect);
                    // Draw sine wave
                    let points: Vec<egui::Pos2> = (0..self.num_points)
                        .map(|i| {
                            let x = i as f32 / (self.num_points - 1) as f32;
                            let y = ((i as f32 / self.sine_period_points as f32) * 2.0 * PI).sin();
                            let px = rect.left() + x * rect.width();
                            let py = rect.center().y - y * (rect.height() / 2.0);
                            egui::Pos2::new(px, py)
                        })
                        .collect();

                    // Use the painter to draw all points as a single line.
                    painter.add(egui::Shape::line(
                        points,
                        egui::Stroke::new(1.0, egui::Color32::LIGHT_GREEN), // Line thickness and color
                    ));

                    // let available_size = ui.available_size();
                    // let (rect, _response) =
                    //     ui.allocate_exact_size(available_size, egui::Sense::hover());
                });

                // ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                //     powered_by_egui_and_eframe(ui);
                //     egui::warn_if_debug_build(ui);
                // });
            });
        });
    }
}

#[allow(dead_code)]
fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to("eframe", "https://github.com/emilk/egui/tree/master/crates/eframe");
        ui.label(".");
    });
}
