pub mod config;
pub mod fps;
pub mod grid;
pub mod ruler2;
pub mod track;
pub mod track2;
pub mod util;
use std::collections::HashMap;

// use crate::view::util::*;
use crate::model::Action;
use crate::view::track::Track;
// use crate::view::track2::Track as Track2;
use crate::{model, wav};
use anyhow::Result;
use egui;
// use model::track2::TrackId;
// use slotmap::SlotMap;

#[derive(Debug)]
pub struct View {
    model: model::Model,
    tracks: HashMap<crate::util::Id, Track>,
    // tracks2: SlotMap<TrackId, Track2>,
    fps: fps::Fps,
}

impl View {
    pub fn new(model: model::Model) -> Self {
        Self {
            model,
            tracks: HashMap::new(),
            // tracks2: SlotMap::default(),
            fps: fps::Fps::new(100),
        }
    }

    pub fn ui_measured(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if cfg!(target_arch = "wasm32") {
            self.ui(ctx, frame);
            return;
        }
        self.fps.start_frame();

        self.ui(ctx, frame);

        self.fps.end_frame();
    }

    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // NOTE: order of panels is important
        self.ui_top_panel_menu_bar(ctx);
        self.ui_right_side_panel(ctx);
        self.ui_left_side_panel(ctx);
        self.ui_bottom_side_panel(ctx);

        // central_panel should always come last
        if let Err(e) = self.ui_central_panel(ctx) {
            tracing::error!("Error drawing central panel");
            tracing::error!("{:#?}", e);
            tracing::error!("{}", e.backtrace());
        }

        self.handle_drag_and_drop_into_app(ctx);

        // We don't stop the program when something fails, like opening a wav file.
        if let Err(e) = self.model.process_actions() {
            tracing::error!("Error processing actions");
            tracing::error!("{:#?}", e);
            tracing::error!("{}", e.backtrace());
        }
    }

    fn ui_bottom_side_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(0.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Bottom Panel");
                });
            });
    }

    fn ui_right_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::right("right_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=ctx.available_rect().width() / 1.5)
            .show(ctx, |ui| {
                ui.add_space(5.0);
                config::show_config(ui, &mut self.model.user_config);
                ui.add_space(5.0);
                self.fps.ui(ui);
                ui.add_space(5.0);
                ruler2::ui_ruler_info_panel(ui, &self.model.tracks2.ruler);
                ui.add_space(5.0);
                ruler2::ui_hover_info_panel(ui, self.model.tracks2.ruler.hover_info.as_ref());
                ruler2::ui_hover_info_panel2(ui, &self.model.tracks2.hover_info.get());
            });
    }

    fn ui_left_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=ctx.available_rect().width() / 1.5)
            .show(ctx, |ui| {

                // ui.separator();

                // egui::Frame::default()
                //     // .stroke(egui::Stroke::new(1.0, egui::Color32::BLACK))
                // .inner_margin(egui::Margin::same(10.0))
                // .show(ui, |ui| {
                //         let model = &self.model;

                //         for track in model.tracks.iter() {
                //             ui.label(&track.name);
                //             if let Some(spp) = track.samples_per_pixel {
                //                 ui.label(format!("samples/pixel: {}", spp));
                //                 let pixels_per_sample = 1.0 / spp;
                //                 ui.label(format!("pixels/sample: {}", pixels_per_sample));
                //             }
                //             ui.separator();
                //         }

                //         if let Some(samples_per_pixel) = model.tracks.samples_per_pixel {
                //             ui.label(format!("samples per pixel: {}", samples_per_pixel));
                //         } else {
                //             ui.label("samples per pixel: not set");
                //         }
                //     });
            });
    }

    /// Handle drag-and-drop wav files
    /// TODO: use actions
    fn handle_drag_and_drop_into_app(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                if let Some(path) = &file.path
                    && path.extension() == Some(std::ffi::OsStr::new("wav"))
                {
                    self.model
                        .actions
                        .push(Action::OpenFile(wav::ReadConfig::new(path)));
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
        ui.horizontal(|ui| {
            if ui.button("reset x zoom").clicked() {
                self.model.actions.push(Action::ZoomToFull);
            }
            if ui.button("fill screen height").clicked() {
                self.model.actions.push(Action::FillScreenHeight);
            }
            if cfg!(target_arch = "wasm32") && ui.button("load demo").clicked() {
                self.model.actions.push(Action::LoadDemo);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.style_mut().spacing.window_margin = egui::Margin::same(4.0);
                if ui.button("close all x").clicked() {
                    // × ✖ ❌ 🗑️
                    // if ui.button("✖").clicked() {
                    self.model.actions.push(Action::RemoveAllTracks);
                    // model.actions.push(Action::RemoveTrack(track_id));
                }
            });
        });
    }

    fn ui_tracks2(&mut self, ui: &mut egui::Ui) -> Result<()> {
        let model = &mut self.model;

        // render view tracks in specified order
        {
            for track_ix in 0..model.tracks2.tracks_order.len() {
                let track_id = model.tracks2.tracks_order[track_ix];
                ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
                crate::view::track2::ui(ui, model, track_id)?;
            }
        }
        Ok(())
    }

    fn ui_tracks(&mut self, ui: &mut egui::Ui) {
        let model = &mut self.model;

        // update view tracks if needed
        {
            for (id, track_model) in &mut model.tracks.tracks {
                if !self.tracks.contains_key(id) {
                    println!("adding new view track: {}", track_model.name);
                    self.tracks
                        .insert(*id, Track::new(track_model.name.clone(), *id));
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
                    view_track.ui(ui, model);
                });
            }
            // No track is hovered, unhover all tracks
            if model.tracks.tracks_hover_info.current.is_none() {
                model.tracks.unhover();
            }
        }
    }

    fn ui_central_panel(&mut self, ctx: &egui::Context) -> Result<()> {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.allocate_ui([ui.available_width(), 50.0].into(), |ui| {
                self.ui_top_panel_tool_bar(ui, ctx);
                // ui.painter().rect(ui.min_rect().shrink(1.0), 0.0, egui::Color32::TRANSPARENT, egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
                // ui.separator();
            });

            ui.allocate_ui([ui.available_width(), 50.0].into(), |ui| {
                let size = ui.available_size();
                ui.set_min_size(size);
                let _ = ruler2::ui(ui, &mut self.model);
            });

            // Reset hover info (but keep/draw previous hover info)
            // before ruler
            self.model.tracks2.hover_info.next();
            // self.ui_top_panel_tool_bar(ui, ctx);

            // ruler::ui(ui, &self.model);
            // let _ = ruler2::ui(ui, &mut self.model);
            egui::ScrollArea::vertical().show(ui, |ui| {
                let size = ui.available_size();
                self.model.tracks2.available_height = size.y;
                ui.allocate_ui(size, |ui| {
                    ui.set_min_width(size.x);

                    // let resp = ui.allocate_exact_size(egui::vec2(ui.available_width(), ui.available_height() - 20.0), egui::Sense::hover());
                    let _ = self.ui_tracks2(ui);

                    // util::debug_rect_text(ui, ui.min_rect().shrink(1.0), egui::Color32::LIGHT_GREEN, "tracks");
                });
            });
        });

        Ok(())
    }

    pub fn model(&self) -> &model::Model {
        &self.model
    }
}
