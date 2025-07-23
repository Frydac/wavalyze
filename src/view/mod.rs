pub mod track;
use std::collections::HashMap;

use crate::model;
use crate::util;
use crate::view::track::Track;
use egui;

#[derive(Debug)]
pub struct View {
    #[allow(dead_code)]
    model: model::SharedModel,
    // tracks: Vec<Track>,
    tracks: HashMap<util::Id, Track>,

    // test_frame: egui::Frame,

    // TODO: store in model
    scroll_speed: f32,
}

impl View {
    pub fn new(model: model::SharedModel) -> Self {
        Self {
            model,
            tracks: HashMap::new(),
            scroll_speed: 10.0,
            // test_frame: egui::Frame::default(),
            // test_frame: Frame::default()
            //     .stroke(Stroke::new(1.0, Color32::BLACK))
            //     .inner_margin(Margin::same(10.0))
            //     .inner_margin(12)
            //     .outer_margin(24)
            //     .rounding(14)
            //     .shadow(egui::Shadow {
            //         offset: [8, 12],
            //         blur: 16,
            //         spread: 0,
            //         color: egui::Color32::from_black_alpha(180),
            //     })
            //     .fill(egui::Color32::from_rgba_unmultiplied(97, 0, 255, 128))
            //     .stroke(egui::Stroke::new(1.0, egui::Color32::GRAY)),
        }
    }

    pub fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            .width_range(80.0..=200.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Left Panel");
                });
                egui::Frame::default()
                    // .stroke(egui::Stroke::new(1.0, egui::Color32::BLACK))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        let model = self.model.borrow();

                        for track in model.tracks.iter() {
                            ui.label(format!("{}", track.name));
                            if let Some(hover_info) = track.hover_info() {
                                // ui.label(format!("screen_pos: {:?}", hover_info.screen_pos));

                                let mut min_ix = i32::MAX;
                                let mut max_ix = i32::MIN;
                                let mut min_sample = Option::<(i32, f32)>::None;
                                let mut max_sample = Option::<(i32, f32)>::None;
                                for (ix, sample) in hover_info.samples.iter() {
                                    min_ix = min_ix.min(*ix);
                                    max_ix = max_ix.max(*ix);
                                    min_sample = min_sample.or(Some((*ix, *sample)));
                                    max_sample = max_sample.or(Some((*ix, *sample)));
                                    if let Some(min_sample) = &mut min_sample {
                                        if min_sample.1 > *sample {
                                            min_sample.1 = *sample;
                                        }
                                    }
                                    if let Some(max_sample) = &mut max_sample {
                                        if max_sample.1 < *sample {
                                            max_sample.1 = *sample;
                                        }
                                    }
                                }

                                if min_ix == max_ix {
                                    ui.label(format!("sample ix: {}", min_ix));
                                    ui.label(format!("sample: {}", min_sample.unwrap().1));
                                } else {
                                    ui.label(format!("sample ix: {}..{}", min_ix, max_ix));
                                    ui.label(format!("min sample: {}", min_sample.unwrap().1));
                                    ui.label(format!("max sample: {}", max_sample.unwrap().1));
                                }
                            }

                            // if track is not last track insert ui.separator
                            // if track != model.tracks2.iter().last().unwrap() {
                                ui.separator();
                            // }
                        }

                        // ui.label("This is a group");
                        // ui.label("Label 1");
                        // ui.label("Label 2");
                        // ui.label("Label 3");
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

    fn top_panel_tool_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // egui::TopBottomPanel::top("top_panel_tool_bar").show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                // ui.heading("Top Panel Tool Bar");
                let _ = ui.button("Test");
                let _ = ui.button("Button");
                ui.add(egui::DragValue::new(&mut self.scroll_speed).speed(1.0));

                if ui.button("⏴ ").clicked() {
                    let _ = self.model.borrow_mut().tracks.shift_x(-self.scroll_speed);
                }
                if ui.button("⏵ ").clicked() {
                    let _  = self.model.borrow_mut().tracks.shift_x(self.scroll_speed);
                }
            });


            // egui::Frame::new().inner_margin(egui::Margin::same(5)).show(ui, |ui| {
            // });

        // });
    }

    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.top_panel_tool_bar(ui, ctx);
            // ui.vertical_centered(|ui| {
            //     ui.heading("Central Panel.");
            // });

            {
                // Slider samples per pixel
                {
                    // TODO: zoom slider? or maybe something better :D
                    let model = self.model.borrow_mut();
                    // let Some(samples_per_pixel) = model.samples_per_pixel()
                    // ui.add(
                    //     egui::Slider::new(&mut self.samples_per_pixel, 0.05..=100.00)
                    //         .step_by(0.01)
                    //         .logarithmic(true)
                    //         .text("samples per pixel"),
                    // );
                    // for (id, track_model) in &mut self.model.borrow_mut().tracks2.tracks {
                    //     track_model.set_samples_per_pixel(self.samples_per_pixel);
                    // }
                }
                //Slider example:
                //
                // let mut value_i32 = *value as i32;
                //             ui.add(
                //                 Slider::new(&mut value_i32, (*min as i32)..=(*max as i32))
                //                     .logarithmic(*logarithmic)
                //                     .clamping(*clamping)
                //                     .smart_aim(*smart_aim)
                //                     .orientation(orientation)
                //                     .text("i32 demo slider")
                //                     .step_by(istep)
                //                     .trailing_fill(*trailing_fill)
                //                     .handle_shape(*handle_shape),
                //             );
                //             *value = value_i32 as f64;
            }

            {
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

                    for i in 0..model.tracks.track_order.len() {
                        // We copy the track_id and don't use an iterator because we want to pass
                        // the model by mutable refernce, and then we would have a reference on the
                        // track_order member.
                        let track_id = model.tracks.track_order[i];
                        ui.allocate_ui([width_track, height_track].into(), |ui| {
                            // We want to detect if any track is hovered, so reset the current
                            model.tracks.tracks_hover_info.current = None;

                            let view_track = &mut self.tracks.get_mut(&track_id).unwrap();
                            // this ui will notify the model of the current hover info
                            view_track.ui(ui, &mut *model);
                        });
                    }
                    // No track is hovered, unhover all tracks
                    if model.tracks.tracks_hover_info.current.is_none() {
                        model.tracks.unhover();
                    }
                }
            }
        });

    }
}

// impl eframe::App for App {
//     fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
//         egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
//             // The top panel is often a good place for a menu bar:

//             egui::menu::bar(ui, |ui| {
//                 // NOTE: no File->Quit on web pages!
//                 let is_web = cfg!(target_arch = "wasm32");
//                 if !is_web {
//                     ui.menu_button("File", |ui| {
//                         if ui.button("Quit").clicked() {
//                             ctx.send_viewport_cmd(egui::ViewportCommand::Close);
//                         }
//                     });
//                     ui.add_space(16.0);
//                 }

//                 egui::widgets::global_theme_preference_buttons(ui);
//             });
//         });
//     }
// }
