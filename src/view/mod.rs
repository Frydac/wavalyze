pub mod config;
pub mod grid;
pub mod track;
use std::collections::HashMap;

use crate::audio;
use crate::math::round::round_up_to_power_of_10;
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

    pub fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // NOTE: order of panels is important
        self.top_panel_menu_bar(ctx);
        self.side_panel(ctx);

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
        self.central_panel(ctx);
    }

    fn side_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(150.0)
            .width_range(80.0..=ctx.available_rect().width() / 1.5)
            .show(ctx, |ui| {
                config::show_config(ui, &mut self.model.borrow_mut().config);

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

    fn top_panel_menu_bar(&mut self, ctx: &egui::Context) {
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
        });
    }

    fn ui_ruler_x(&mut self, ui: &mut egui::Ui) {
        let height = 40.0;
        let width = ui.available_width();
        // let rect = egui::Rect::from_x_y_ranges(0.0..=width, 0.0..=height);
        let stroke = egui::Stroke::new(1.0, egui::Color32::RED);
        ui.allocate_ui([width, height].into(), |ui| {
            egui::Frame::default()
                .inner_margin(egui::Margin::same(5.0))
                .outer_margin(egui::Margin::symmetric(0.0, 5.0))
                .stroke(ui.style().visuals.window_stroke())
                .show(ui, |ui| {
                    ui.set_min_size(ui.available_size());
                    // ui.label("ruler");
                    // let color = egui::Color32::from_rgb(100, 100, 100);
                    let color = ui.style().visuals.text_color();
                    // let color = egui::Color32::LIGHT_YELLOW;
                    let stroke = egui::Stroke::new(1.0, color);
                    let ruler_rect = ui.min_rect();

                    if self.model.borrow_mut().tracks.is_empty() {
                        return;
                    }
                    {
                        let model = self.model.borrow();
                        let track = model.tracks.track(0);
                        let last_sample_ix = model.tracks.get_total_buffer_range().last() as u64;

                        // lets try and draw the 0 sample tick
                        if let Some(track) = track {
                            // draw zero tick
                            if track.sample_rect.ix_rng.contains(0.0) {
                                if let Some(screen_x0) = track.sample_ix_to_screen_x(0.0) {
                                    let pos_0 = [screen_x0, ruler_rect.top()].into();
                                    let pos_1 = [screen_x0, ruler_rect.bottom()].into();
                                    let pos_0 = ui.painter().round_pos_to_pixel_center(pos_0);
                                    let pos_1 = ui.painter().round_pos_to_pixel_center(pos_1);
                                    ui.painter().line_segment([pos_0, pos_1], stroke);

                                    let text_pos = pos_0 + egui::vec2(5.0, 0.0);
                                    ui.painter().text(
                                        text_pos,
                                        egui::Align2::LEFT_TOP,
                                        "0",
                                        egui::FontId::default(),
                                        ui.style().visuals.text_color(),
                                    );
                                }
                            }

                            // draw last tick
                            {
                                if let Some(screen_x) = track.sample_ix_to_screen_x(last_sample_ix as audio::SampleIx) {
                                    let pos_0 = [screen_x, ruler_rect.top()].into();
                                    let pos_1 = [screen_x, ruler_rect.bottom()].into();
                                    let pos_0 = ui.painter().round_pos_to_pixel_center(pos_0);
                                    let pos_1 = ui.painter().round_pos_to_pixel_center(pos_1);
                                    ui.painter().line_segment([pos_0, pos_1], stroke);

                                    let text_pos = pos_0 + egui::vec2(5.0, 0.0);
                                    ui.painter().text(
                                        text_pos,
                                        egui::Align2::LEFT_TOP,
                                        last_sample_ix as u64,
                                        egui::FontId::default(),
                                        ui.style().visuals.text_color(),
                                    );
                                }
                            }
                            // draw grid ticks
                            {
                                let nr_pixels_per_tick = 120.0f64;
                                let max_nr_ticks = ruler_rect.width() as f64 / nr_pixels_per_tick;
                                let sample_width = track.sample_rect.ix_rng.width();

                                if sample_width != 0.0 {
                                    let min_nr_samples_per_tick = sample_width / max_nr_ticks as f64;
                                    let nr_samples_per_tick = round_up_to_power_of_10(min_nr_samples_per_tick) as u64;
                                    let start_sample_ix =
                                        track.sample_rect.ix_rng.start() as u64 / nr_samples_per_tick * nr_samples_per_tick;
                                    let mut cur_sample_ix = start_sample_ix as u64;

                                    // draw ticks with labels
                                    while cur_sample_ix < last_sample_ix {
                                        if let Some(screen_x) = track.sample_ix_to_screen_x(cur_sample_ix as audio::SampleIx) {
                                            // let screen_x = track.sample_ix_to_screen_x(start_sample_ix).unwrap();
                                            let pos_0 = [screen_x, ruler_rect.top()].into();
                                            let pos_1 = [screen_x, ruler_rect.bottom()].into();
                                            let pos_0 = ui.painter().round_pos_to_pixel_center(pos_0);
                                            let pos_1 = ui.painter().round_pos_to_pixel_center(pos_1);
                                            ui.painter().line_segment([pos_0, pos_1], stroke);

                                            let text_pos = pos_0 + egui::vec2(5.0, 0.0);
                                            ui.painter().text(
                                                text_pos,
                                                egui::Align2::LEFT_TOP,
                                                cur_sample_ix as u64,
                                                egui::FontId::default(),
                                                ui.style().visuals.text_color(),
                                            );
                                        }
                                        cur_sample_ix += nr_samples_per_tick;
                                    }

                                    // draw ticks without labels
                                    let mut cur_sample_ix = start_sample_ix;
                                    let offset = cur_sample_ix % nr_samples_per_tick;
                                    let mut cur_sample_ix_flt = start_sample_ix as f64;
                                    let nr_samples_per_small_tick = nr_samples_per_tick as f64 * 0.1;
                                    while cur_sample_ix < last_sample_ix {
                                        let y_1 = ruler_rect.bottom();
                                        if cur_sample_ix % nr_samples_per_tick == offset {
                                            // we already drawn this, put the other code here
                                        } else if cur_sample_ix % (nr_samples_per_tick / 2) == offset {
                                            if let Some(screen_x) = track.sample_ix_to_screen_x(cur_sample_ix as audio::SampleIx) {
                                                let y_0 = y_1 - (ruler_rect.height() / 2.0);
                                                let pos_0 = [screen_x, y_0].into();
                                                let pos_1 = [screen_x, y_1].into();
                                                let pos_0 = ui.painter().round_pos_to_pixel_center(pos_0);
                                                let pos_1 = ui.painter().round_pos_to_pixel_center(pos_1);
                                                ui.painter().line_segment([pos_0, pos_1], stroke);
                                            }
                                        } else {
                                            if let Some(screen_x) = track.sample_ix_to_screen_x(cur_sample_ix as audio::SampleIx) {
                                                let y_0 = y_1 - (ruler_rect.height() / 4.0);
                                                let pos_0 = [screen_x, y_0].into();
                                                let pos_1 = [screen_x, y_1].into();
                                                let pos_0 = ui.painter().round_pos_to_pixel_center(pos_0);
                                                let pos_1 = ui.painter().round_pos_to_pixel_center(pos_1);
                                                ui.painter().line_segment([pos_0, pos_1], stroke);
                                            }
                                        }
                                        cur_sample_ix_flt += nr_samples_per_small_tick;
                                        cur_sample_ix = cur_sample_ix_flt as u64;
                                    }
                                }
                            }
                            if let Some(hover_info) = track.hover_info() {
                                let screen_x = hover_info.screen_pos.x;
                                let pos_0 = [screen_x, ruler_rect.top()].into();
                                let pos_1 = [screen_x, ruler_rect.bottom()].into();
                                let pos_0 = ui.painter().round_pos_to_pixel_center(pos_0);
                                let pos_1 = ui.painter().round_pos_to_pixel_center(pos_1);
                                ui.painter().line_segment([pos_0, pos_1], stroke);

                                if let Some(sample) = hover_info.samples.first() {
                                    let sample_ix = sample.0;
                                    let text_pos = pos_0 + egui::vec2(5.0, 0.0);
                                    ui.painter().text(
                                        text_pos,
                                        egui::Align2::LEFT_TOP,
                                        sample_ix as u64,
                                        egui::FontId::default(),
                                        ui.style().visuals.text_color(),
                                    );
                                }
                            }
                        }
                    }

                    // let track = model.tracks[0];
                    // need
                    // * screen width of tracks
                    // * zoom level of tracks
                    // * constant: nr of pixels per tick
                    //   * maybe we have label ticks and non-label ticks? (start with label
                    //     ticks only)
                    // * determine 'tick width' in samples
                    // * draw ticks on sample positions
                    //   * add label
                });
            // ui.label("ruler");
            // ui.painter().line_segment([0.0, 0.0], [width, 0.0], stroke);
            // ui.painter().line_segment([0.0, 0.0], [0.0, height], stroke);
        });
        // ui.painter().rect_stroke(rect, 0.0, stroke);
    }

    fn ui_tracks(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let mut model = self.model.borrow_mut();

        // update view tracks if needed
        // TODO: delete? tracks
        {
            for (id, track_model) in &mut model.tracks.tracks {
                if !self.tracks.contains_key(id) {
                    println!("adding new view track: {}", track_model.name);
                    self.tracks.insert(*id, Track::new(track_model.name.clone(), *id));
                }
            }
        }
        // render view tracks in specified order
        {
            let rect_height = ui.available_height();
            let height_track = rect_height / self.tracks.len() as f32;
            let width_track = ui.available_width();

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

    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.ui_top_panel_tool_bar(ui, ctx);
            self.ui_ruler_x(ui);
            self.ui_tracks(ctx, ui);
        });
    }
}
