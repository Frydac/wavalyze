pub mod config;
pub mod grid;
pub mod ruler;
pub mod ruler2;
pub mod track;
pub mod track2;
use std::collections::HashMap;

use crate::model;
use crate::util;
use crate::view::track::Track;
use egui;

#[derive(Debug)]
pub struct View {
    #[allow(dead_code)]
    model: model::SharedModel,
    tracks: HashMap<util::Id, Track>,
    // TODO: store in model
    scroll_speed: f32,
}

impl View {
    pub fn new(model: model::SharedModel) -> Self {
        Self {
            model,
            tracks: HashMap::new(),
            scroll_speed: 10.0,
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui_handle_dropped_wav_files(ctx);
        self.model.borrow_mut().process_actions();

        // NOTE: order of panels is important
        self.ui_top_panel_menu_bar(ctx);
        self.ui_side_panel(ctx);

        // TODO: place holder
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(0.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Bottom Panel");
                });
            });

        // NOTE: central_panel should always come last
        self.ui_central_panel(ctx);
    }

    fn ui_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=ctx.available_rect().width() / 1.5)
            .show(ctx, |ui| {
                config::show_config(ui, &mut self.model.borrow_mut().user_config);

                ui.separator();

                egui::Frame::default()
                    // .stroke(egui::Stroke::new(1.0, egui::Color32::BLACK))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        let model = self.model.borrow();

                        for track in model.tracks.iter() {
                            ui.label(&track.name);
                            if let Some(spp) = track.samples_per_pixel {
                                ui.label(format!("samples/pixel: {}", spp));
                                let pixels_per_sample = 1.0 / spp;
                                ui.label(format!("pixels/sample: {}", pixels_per_sample));
                            }
                            ui.separator();
                        }

                        if let Some(samples_per_pixel) = model.tracks.samples_per_pixel {
                            ui.label(format!("samples per pixel: {}", samples_per_pixel));
                        } else {
                            ui.label("samples per pixel: not set");
                        }
                    });
            });
    }

    fn ui_handle_dropped_wav_files(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path {
                    if path.extension() == Some(std::ffi::OsStr::new("wav")) {
                        if let Some(path_str) = path.to_str() {
                            if self.model.borrow_mut().add_wav_file(path_str, None, None).is_err() {
                                tracing::error!("Failed to add wav file: {}", path_str);
                            }
                        }
                    }
                }
            }
        });
    }

    fn ui_top_panel_menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel_menu_bar").show(ctx, |ui| {
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
    }

    fn ui_top_panel_tool_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal_top(|ui| {
            let _ = ui.button("Test");
            let _ = ui.button("Button");
            ui.add(egui::DragValue::new(&mut self.scroll_speed).speed(1.0));

            if ui.button("⏴ ").clicked() {
                let _ = self.model.borrow_mut().tracks.shift_x(-self.scroll_speed);
            }
            if ui.button("⏵ ").clicked() {
                let _ = self.model.borrow_mut().tracks.shift_x(self.scroll_speed);
            }
            // × ✖ ❌ 🗑️
            if ui.button("✖").clicked() {
                self.model.borrow_mut().tracks.clear_tracks();
            }
        });
    }

    fn ui_tracks(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut model = self.model.borrow_mut();

        // update view tracks if needed
        {
            for (id, track_model) in &mut model.tracks.tracks {
                if !self.tracks.contains_key(id) {
                    println!("adding new view track: {}", track_model.name);
                    self.tracks.insert(*id, Track::new(track_model.name.clone(), *id));
                }
            }
            self.tracks.retain(|id, _| {
                let should_keep = model.tracks.tracks.contains_key(id);
                if !should_keep {
                    println!("removing view track: {id}");
                }
                should_keep
            });
        }
        // render view tracks in specified order
        {
            // let rect_height = ui.available_height() - 20.0;
            let rect_height = ui.available_height();
            let height_track = rect_height / self.tracks.len() as f32;
            let width_track = ui.available_width();
            let min_height_track = 150.0;
            let height_track = height_track.max(min_height_track);

            // We want to detect if any track is hovered, so reset the current
            model.tracks.tracks_hover_info.current = None;

            for i in 0..model.tracks.track_order.len() {
                // We copy the track_id and don't use an iterator because we want to pass
                // the model by mutable refernce, and then we would have a reference on the
                // track_order member.
                let track_id = model.tracks.track_order[i];
                ui.allocate_ui([width_track, height_track].into(), |ui| {
                    let view_track = &mut self.tracks.get_mut(&track_id).unwrap();
                    // this ui will notify the model of the current hover info
                    view_track.ui(ui, &mut model);
                });
            }
            // No track is hovered, unhover all tracks
            if model.tracks.tracks_hover_info.current.is_none() {
                model.tracks.unhover();
            }
        }
    }

    fn ui_central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui_top_panel_tool_bar(ui, ctx);
            // ruler::ui(ui, &self.model);
            let _ = ruler2::ui(ui, &self.model);
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.allocate_ui([ui.available_width(), ui.available_height() - 20.0].into(), |ui| {
                    self.ui_tracks(ctx, ui);
                });
                // ui.with_max_height(ui.available_height() - 20.0, |ui| {
                //     self.ui_tracks(ctx, ui);
                // });
            });
        });
    }
}
