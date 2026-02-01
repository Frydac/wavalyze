pub mod config;
pub mod fps;
pub mod grid;
pub mod ruler2;
pub mod track;
pub mod track2;
pub mod util;
use std::collections::HashMap;

// use crate::view::util::*;
use crate::model::{Action, hover_info::HoverInfoE};
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
        if self.model.load_mgr.pending() > 0 {
            ctx.request_repaint();
        }
        if self.model.drain_load_results() {
            ctx.request_repaint();
        }

        // Clear hover by default; hover interactions in this frame can override it.
        self.model
            .actions
            .push(Action::SetHoverInfo(HoverInfoE::NotHovered));

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

        self.ui_loading_modal(ctx);

        let had_dropped_files = self.handle_drag_and_drop_into_app(ctx);

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
                ruler2::ui_hover_info_panel2(ui, &self.model.tracks2.hover_info);
            });
    }

    fn ui_left_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=ctx.available_rect().width() / 1.5)
            .show(ctx, |ui| {

                // ui.separator();
            });
    }

    /// Handle drag-and-drop wav files
    /// TODO: use actions
    fn handle_drag_and_drop_into_app(&mut self, ctx: &egui::Context) -> bool {
        let mut had_dropped_files = false;
        ctx.input(|i| {
            for file in &i.raw.dropped_files {
                had_dropped_files = true;
                if cfg!(target_arch = "wasm32") {
                    if let Some(bytes) = &file.bytes {
                        let name = if file.name.is_empty() {
                            None
                        } else {
                            Some(file.name.clone())
                        };
                        let is_wav_by_name = name
                            .as_deref()
                            .map(|name| name.to_lowercase().ends_with(".wav"))
                            .unwrap_or(false);
                        let is_wav_by_header = bytes.len() >= 12
                            && &bytes[0..4] == b"RIFF"
                            && &bytes[8..12] == b"WAVE";
                        if is_wav_by_name || is_wav_by_header {
                            let label = name.clone().or_else(|| Some("dropped.wav".to_string()));
                            self.model.actions.push(Action::OpenFileBytes(
                                wav::ReadConfigBytes::new(label, bytes.to_vec()),
                            ));
                        }
                    }
                } else if let Some(path) = &file.path
                    && path.extension() == Some(std::ffi::OsStr::new("wav"))
                {
                    self.model
                        .actions
                        .push(Action::OpenFile(wav::ReadConfig::new(path)));
                }
            }
        });
        had_dropped_files
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
            if self.tracks.is_empty() {
                return;
            }
            // let rect_height = ui.available_height() - 20.0;
            let rect_height = ui.available_height().max(0.0);
            let height_track = rect_height / self.tracks.len() as f32;
            let width_track = ui.available_width().max(0.0);
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
            let top_width = ui.available_width().max(0.0);
            ui.allocate_ui([top_width, 50.0].into(), |ui| {
                self.ui_top_panel_tool_bar(ui, ctx);
                // ui.painter().rect(ui.min_rect().shrink(1.0), 0.0, egui::Color32::TRANSPARENT, egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
                // ui.separator();
            });

            let ruler_width = ui.available_width().max(0.0);
            ui.allocate_ui([ruler_width, 50.0].into(), |ui| {
                let size = ui.available_size();
                let size = egui::vec2(size.x.max(0.0), size.y.max(0.0));
                ui.set_min_size(size);
                let _ = ruler2::ui(ui, &mut self.model);
            });
            // self.ui_top_panel_tool_bar(ui, ctx);

            // ruler::ui(ui, &self.model);
            // let _ = ruler2::ui(ui, &mut self.model);
            egui::ScrollArea::vertical().show(ui, |ui| {
                let size = ui.available_size();
                let size = egui::vec2(size.x.max(0.0), (size.y - 1.0).max(0.0));
                self.model.tracks2.available_height = size.y;
                ui.allocate_ui(size, |ui| {
                    ui.set_min_width(size.x.max(0.0));

                    // let resp = ui.allocate_exact_size(egui::vec2(ui.available_width(), ui.available_height() - 20.0), egui::Sense::hover());
                    let _ = self.ui_tracks2(ui);

                    // util::debug_rect_text(ui, ui.min_rect().shrink(1.0), egui::Color32::LIGHT_GREEN, "tracks");
                });
            });
        });

        Ok(())
    }

    /// Show a modal with a progress bar when loading files.
    fn ui_loading_modal(&mut self, ctx: &egui::Context) {
        if self.model.load_mgr.pending() == 0 {
            return;
        }

        let (path_label, stage_label, progress_value, overall_value) =
            match self.model.load_mgr.any_progress_entry() {
                Some(entry) => {
                    let (stage, current, total) = entry.handle.snapshot();
                    let value = if total > 0 {
                        (current as f32 / total as f32).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    let overall_value = match stage {
                        crate::wav::read::LoadStage::Start => 0.0,
                        crate::wav::read::LoadStage::ReadingSamples => 0.0 + value * 0.55,
                        crate::wav::read::LoadStage::Deinterleaving => 0.55 + value * 0.15,
                        crate::wav::read::LoadStage::Converting => 0.70 + value * 0.05,
                        crate::wav::read::LoadStage::Thumbnail => 0.75 + value * 0.20,
                        crate::wav::read::LoadStage::Finalizing => 0.95 + value * 0.05,
                        crate::wav::read::LoadStage::Done => 1.0,
                    };
                    (
                        entry
                            .path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("file"),
                        match stage {
                            crate::wav::read::LoadStage::Start => "starting",
                            crate::wav::read::LoadStage::ReadingSamples => "reading samples",
                            crate::wav::read::LoadStage::Deinterleaving => "deinterleaving",
                            crate::wav::read::LoadStage::Converting => "converting",
                            crate::wav::read::LoadStage::Thumbnail => "thumbnails",
                            crate::wav::read::LoadStage::Finalizing => "finalizing",
                            crate::wav::read::LoadStage::Done => "done",
                        },
                        value,
                        overall_value.clamp(0.0, 1.0),
                    )
                }
                None => ("file", "loading", 0.0, 0.0),
            };

        egui::Window::new("Loading")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!("Loading {path_label}…"));
                ui.label(format!("Stage: {stage_label}"));
                ui.add(egui::ProgressBar::new(progress_value).show_percentage());
                ui.add(egui::ProgressBar::new(overall_value).text("overall"));
            });
    }

    pub fn model(&self) -> &model::Model {
        &self.model
    }

    pub fn enqueue_actions<I>(&mut self, actions: I)
    where
        I: IntoIterator<Item = Action>,
    {
        self.model.actions.extend(actions);
    }
}
