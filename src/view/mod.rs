pub mod config;
pub mod file;
mod file_loader;
pub mod fps;
pub mod grid;
pub mod jobs;
pub mod ruler;
pub mod selection_info;
pub mod track;
pub mod util;
pub mod value_ruler2;

use crate::model::{Action, hover_info::HoverInfoE, shortcuts};
use crate::{model, wav};
use anyhow::Result;
use egui;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct View {
    model: model::Model,
    fps: fps::Fps,
    picker_tx: Sender<file_loader::PickerMessage>,
    picker_rx: Receiver<file_loader::PickerMessage>,
    picker_pending: usize,
}

impl View {
    pub fn new(model: model::Model) -> Self {
        let (picker_tx, picker_rx) = std::sync::mpsc::channel();
        Self {
            model,
            fps: fps::Fps::new(100),
            picker_tx,
            picker_rx,
            picker_pending: 0,
        }
    }

    /// Draw ui, and measure frame time
    pub fn ui_measured(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.fps.start_frame();
        self.ui(ctx, frame);
        self.fps.end_frame();
    }

    /// Draw ui and handle interactions
    pub fn ui(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.drain_picker_results(ctx);
        if self.picker_pending > 0 {
            ctx.request_repaint();
        }

        if self.model.drain_job_events() {
            ctx.request_repaint();
        }
        if self.model.job_mgr.pending() > 0 {
            ctx.request_repaint();
        }

        if self.model.load_mgr.pending() > 0 {
            ctx.request_repaint();
        }
        if self.model.drain_load_results() {
            ctx.request_repaint();
        }

        // Clear hover by default; hover interactions in this frame can override it.
        // TODO: move to ruler + tracks
        self.model
            .actions
            .push(Action::SetHoverInfo(HoverInfoE::NotHovered));
        shortcuts::dispatch_shortcuts(
            ctx,
            &self.model.tracks,
            &self.model.user_config.shortcuts,
            &mut self.model.actions,
        );

        // NOTE: order of panels is important
        self.ui_top_panel_menu_bar(ctx);
        self.ui_right_side_panel(ctx);
        self.ui_left_side_panel(ctx);
        self.ui_bottom_panel(ctx);

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
        if self.model.job_mgr.pending() > 0 || self.model.load_mgr.pending() > 0 {
            ctx.request_repaint();
        }
    }

    fn ui_bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("bottom_panel")
            .resizable(false)
            .min_height(0.0)
            .show(ctx, |ui| {
                selection_info::ui_selection_info_toolbar(
                    ui,
                    &mut self.model.user_config,
                    self.model.tracks.selection_info,
                    &mut self.model.actions,
                );
                // ui.vertical_centered(|ui| {
                //     ui.heading("Bottom Panel");
                // });
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
                jobs::ui_panel(ui, &mut self.model);
                ui.add_space(5.0);
                ruler::ui_ruler_info_panel(ui, &self.model.tracks.ruler);
                ui.add_space(5.0);
                // ruler::ui_hover_info_panel(ui, self.model.tracks2.ruler.hover_info.as_ref());
                ruler::ui_hover_info_panel2(ui, &self.model.tracks.hover_info);
                selection_info::ui_selection_info_side_panel(
                    ui,
                    &mut self.model.tracks.selection_info,
                );
            });
    }

    fn ui_left_side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(250.0)
            // .width_range(80.0..=ctx.available_rect().width())
            .width_range(100.0..=500.0)
            .show(ctx, |ui| {
                file::ui(ui, &mut self.model);
            });
    }

    /// Handle drag-and-drop wav files
    /// TODO: use actions
    fn handle_drag_and_drop_into_app(&mut self, ctx: &egui::Context) -> bool {
        let mut had_dropped_files = false;
        let mut dropped_bytes = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        let mut dropped_paths = Vec::new();
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
                            dropped_bytes.push(wav::ReadConfigBytes::new(label, bytes.to_vec()));
                        }
                    }
                } else if let Some(path) = &file.path
                    && path.extension() == Some(std::ffi::OsStr::new("wav"))
                {
                    #[cfg(not(target_arch = "wasm32"))]
                    dropped_paths.push(path.clone());
                }
            }
        });
        if !dropped_bytes.is_empty() {
            for config in dropped_bytes {
                self.model.actions.push(Action::OpenFileBytes(config));
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if !dropped_paths.is_empty() {
            self.picker_pending = self.picker_pending.saturating_add(1);
            file_loader::load_paths(dropped_paths, self.picker_tx.clone());
            ctx.request_repaint();
        }
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
            if ui.button("open wav files...").clicked() {
                self.start_file_picker();
            }
            if ui.button("reset x zoom").clicked() {
                self.model.actions.push(Action::ZoomToFull);
            }
            if ui.button("fill screen height").clicked() {
                self.model.actions.push(Action::FillScreenHeight);
            }
            if ui.button("recenter y").clicked() {
                self.model.actions.push(Action::RecenterYAll);
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
            for track_ix in 0..model.tracks.tracks_order.len() {
                let track_id = model.tracks.tracks_order[track_ix];
                if model
                    .tracks
                    .get_track(track_id)
                    .is_none_or(|track| !track.visible)
                {
                    continue;
                }
                ui.style_mut().spacing.item_spacing = egui::vec2(0.0, 0.0);
                crate::view::track::ui(ui, model, track_id)?;
            }
        }
        Ok(())
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
                let _ = ruler::ui(ui, &mut self.model);
            });
            // self.ui_top_panel_tool_bar(ui, ctx);

            // ruler::ui(ui, &self.model);
            // let _ = ruler::ui(ui, &mut self.model);
            egui::ScrollArea::vertical().show(ui, |ui| {
                let size = ui.available_size();
                let size = egui::vec2(size.x.max(0.0), (size.y - 1.0).max(0.0));
                self.model.tracks.available_height = size.y;
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
        if let Some(job) = self
            .model
            .job_mgr
            .jobs()
            .into_iter()
            .find(|job| job.kind == model::jobs::JobKind::LoadWav)
            && let Some(progress) = job.load_progress
        {
            let stage_value = if progress.total > 0 {
                (progress.current as f32 / progress.total as f32).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let overall_value = progress
                .stage
                .overall_fraction(progress.current, progress.total)
                .clamp(0.0, 1.0);
            egui::Window::new("Loading")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(format!("Loading {}…", job.label));
                    ui.label(format!("Stage: {}", progress.stage.label()));
                    ui.add(egui::ProgressBar::new(stage_value).show_percentage());
                    ui.add(egui::ProgressBar::new(overall_value).text("overall"));
                });
            return;
        }

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

    fn drain_picker_results(&mut self, ctx: &egui::Context) {
        let mut had_results = false;
        loop {
            match self.picker_rx.try_recv() {
                Ok(file_loader::PickerMessage::Files(files)) => {
                    had_results = true;
                    self.picker_pending = self.picker_pending.saturating_sub(1);
                    self.model
                        .actions
                        .extend(files.into_iter().map(Action::OpenFileBytes));
                }
                #[cfg(not(target_arch = "wasm32"))]
                Ok(file_loader::PickerMessage::Error(err)) => {
                    had_results = true;
                    self.picker_pending = self.picker_pending.saturating_sub(1);
                    tracing::error!("File picker load failed: {err}");
                }
                Ok(file_loader::PickerMessage::Cancelled) => {
                    had_results = true;
                    self.picker_pending = self.picker_pending.saturating_sub(1);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    tracing::error!("File picker channel disconnected");
                    break;
                }
            }
        }

        if had_results {
            ctx.request_repaint();
        }
    }

    fn start_file_picker(&mut self) {
        self.picker_pending = self.picker_pending.saturating_add(1);
        file_loader::pick_wav_files(self.picker_tx.clone());
    }

    pub fn enqueue_actions<I>(&mut self, actions: I)
    where
        I: IntoIterator<Item = Action>,
    {
        self.model.actions.extend(actions);
    }
}
