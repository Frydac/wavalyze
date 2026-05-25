pub mod config;
pub mod db_ruler;
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
use crate::{log::TracingCollector, model, wav};
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
    show_tracing_window: bool,
    tracing_collector: TracingCollector,
}

impl View {
    pub fn new(model: model::Model, tracing_collector: TracingCollector) -> Self {
        let (picker_tx, picker_rx) = std::sync::mpsc::channel();
        Self {
            model,
            fps: fps::Fps::new(100),
            picker_tx,
            picker_rx,
            picker_pending: 0,
            show_tracing_window: false,
            tracing_collector,
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
        if self.model.drain_action_messages() {
            ctx.request_repaint();
        }
        if self.model.job_mgr.pending() > 0 {
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
        self.ui_tracing_window(ctx);

        let had_dropped_files = self.handle_drag_and_drop_into_app(ctx);

        // We don't stop the program when something fails, like opening a wav file.
        if let Err(e) = self.model.process_actions() {
            tracing::error!("Error processing actions");
            tracing::error!("{:#?}", e);
            tracing::error!("{}", e.backtrace());
        }
        if self.model.job_mgr.pending() > 0 {
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
                ruler::ui_ruler_info_panel(ui, &self.model.tracks);
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

                ui.menu_button("Debug", |ui| {
                    if ui
                        .checkbox(&mut self.show_tracing_window, "Tracing")
                        .clicked()
                    {
                        ui.close_menu();
                    }
                });
                ui.add_space(16.0);

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });
    }

    fn ui_tracing_window(&mut self, ctx: &egui::Context) {
        if !self.show_tracing_window {
            return;
        }

        ctx.request_repaint();

        egui::Window::new("Tracing")
            .default_size([880.0, 420.0])
            .open(&mut self.show_tracing_window)
            .show(ctx, |ui| {
                ui.add(egui_tracing::Logs::new(self.tracing_collector.clone()));
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
                ui.style_mut().spacing.window_margin = egui::Margin::same(4);
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

    /// Modal-as-policy: view-layer decision to surface active LoadWav jobs as a foreground modal.
    /// The data layer (`JobProgress`) is generic — if/when this UX is replaced (e.g., with a
    /// status-bar indicator), this whole function can be deleted without touching `jobs/`.
    fn ui_loading_modal(&mut self, ctx: &egui::Context) {
        let Some(job) = self
            .model
            .job_mgr
            .jobs()
            .find(|job| job.kind == model::jobs::JobKind::LoadWav)
        else {
            return;
        };
        let p = &job.progress;
        let stage_value = if p.stage_total > 0 {
            (p.stage_current as f32 / p.stage_total as f32).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let overall_value = p.overall_fraction.clamp(0.0, 1.0);
        egui::Window::new("Loading")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(format!("Loading {}…", job.label));
                ui.label(format!("Stage: {}", p.stage_name));
                ui.add(egui::ProgressBar::new(stage_value).show_percentage());
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
