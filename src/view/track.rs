use crate::model;
// use egui;
// use crate::pos;
use crate::util::Id;
use eframe::egui;
use egui::Pos2;

// Add with_alpha() to egui::Color32 type
trait Color32Ext {
    fn with_alpha(self, alpha: u8) -> Self;
}

impl Color32Ext for egui::Color32 {
    fn with_alpha(self, alpha: u8) -> Self {
        egui::Color32::from_rgba_unmultiplied(self.r(), self.g(), self.b(), alpha)
    }
}

#[derive(Debug)]
pub struct Track {
    name: String,
    id: Id,

    stroke_middle_line: egui::Stroke,
    mouse_hover_info: MouseHover,
    mouse_select: MouseSelect,
}

impl Track {
    pub fn new(name: String, model_track_id: Id) -> Self {
        Self {
            name,
            id: model_track_id,
            stroke_middle_line: egui::Stroke::new(1.0, egui::Color32::GRAY),
            mouse_hover_info: MouseHover::default(),
            mouse_select: MouseSelect::default(),
        }
    }

    pub fn ui2(&mut self, ui: &mut egui::Ui, model: &mut model::Model) {
        let model_track = model.tracks.track_mut(self.id).unwrap();
        self.ui(ui, model_track);
        // model.tracks.hover
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, model_track: &mut model::track::Track) {
        ui.label(&self.name);
        let width = ui.available_width();
        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            ui.set_min_size(egui::vec2(ui.available_width(), ui.available_height()));

            // This gets the absolute position of the canvas
            let canvas_rect = ui.min_rect();
            let painter = ui.painter_at(canvas_rect);

            // model_track.set_nr_pixels_to_fill_x(canvas_rect.width() as usize);
            let _ = model_track.set_screen_rect(canvas_rect.into());

            // Should do interactions here? and then model.update_view_buffer
            // 1. interactions to update model
            // 2. model.update_view_buffer
            // 3. transform/draw view buffer

            // let track_view_buffer = model_track.view_buffer2();
            // let track_view_rect = model_track.view_rect();

            // let to_screen = egui::emath::RectTransform::from_to(track_view_rect.clone().into(), canvas_rect);

            // let screen_pos: vec<pos2> = track_view_buffer
            //     .iter()
            //     .map(|track_pos| {
            //         let pos = egui::pos2(track_pos.x, track_pos.y);
            //         // println!("{}: {}", "pos before", pos);
            //         // let pos = painter.round_pos_to_pixels(pos);
            //         let pos = painter.round_pos_to_pixel_center(pos);
            //         // println!("{}: {}", "pos after", pos);
            //         to_screen.transform_pos(pos)
            //     })
            //     .collect();

            // let view_buffer = model_track.view_buffer();
            let track_view_rect = model_track.view_rect();
            // dbg!(track_view_rect);
            // dbg!(canvas_rect);

            let to_screen = egui::emath::RectTransform::from_to(track_view_rect.clone().into(), canvas_rect);
            let to_buffer = to_screen.inverse();

            let color = egui::Color32::LIGHT_RED;
            let stroke_line = egui::Stroke::new(1.0, color.with_alpha(128));

            match model_track.view_buffer3() {
                model::ViewBuffer::SingleSamples(buffer_pos) => {
                    for pos_sample in buffer_pos {
                        let pos_sample: egui::Pos2 = pos_sample.into();
                        let pos_sample_mid = egui::pos2(pos_sample.x, 0.0);
                        let pos_sample_screen = painter.round_pos_to_pixel_center(to_screen.transform_pos(pos_sample));
                        let pos_sample_mid_screen = painter.round_pos_to_pixel_center(to_screen.transform_pos(pos_sample_mid));

                        painter.line_segment([pos_sample_mid_screen, pos_sample_screen], stroke_line);
                        painter.circle_filled(pos_sample_screen, 1.5, color);
                    }
                }
                model::ViewBuffer::OneLine(buffer_pos) => {
                    let screen_pos: Vec<Pos2> = buffer_pos
                        .iter()
                        .map(|pos_sample| {
                            let pos_sample = egui::pos2(pos_sample.x, pos_sample.y);
                            // let pos_sample_screen = painter.round_pos_to_pixel_center(to_screen.transform_pos(pos_sample));
                            let pos_sample_screen = to_screen.transform_pos(pos_sample);
                            pos_sample_screen
                        })
                        .collect();

                    // painter.add(egui::Shape::line(screen_pos, stroke_line));
                    painter.line(screen_pos, stroke_line);
                }
                model::ViewBuffer::LinePerPixelColumn(buffer_pos_min_max) => {
                    let mut prev_max_y = f32::MAX;
                    let mut prev_min_y = f32::MIN;
                    for [min, max] in buffer_pos_min_max {
                        let min = egui::pos2(min.x, min.y);
                        let max = egui::pos2(max.x, max.y);
                        // let min = painter.round_pos_to_pixel_center(to_screen.transform_pos(min));
                        // let max = painter.round_pos_to_pixel_center(to_screen.transform_pos(max));
                        // let min = to_screen.transform_pos(min) + egui::Vec2::new(0.5, 0.0);
                        // let max = to_screen.transform_pos(max) + egui::Vec2::new(0.5, 0.0);
                        let min = to_screen.transform_pos(min);
                        let max = to_screen.transform_pos(max);
                        painter.line_segment([min, max], stroke_line);
                        if prev_max_y < min.y {
                            painter.line_segment([egui::pos2(min.x, prev_max_y), min], stroke_line);
                        } else if prev_min_y > max.y {
                            painter.line_segment([egui::pos2(max.x, prev_min_y), max], stroke_line);
                        }
                        prev_max_y = max.y;
                        prev_min_y = min.y;
                    }
                }
            }

            // let points: Vec<Pos2> = view_buffer
            //     .iter()
            //     .enumerate()
            //     .map(|(i, v)| {
            //         let x = i as f32;
            //         let y = *v;
            //         let buffer_pos = emath::pos2(x, y);
            //         let screen_pos = to_screen.transform_pos(buffer_pos);
            //         screen_pos
            //     })
            //     .collect();
            // // painter.add(egui::epaint::LineSegment {
            // //     points,
            // //     stroke: Stroke::new(1.0, egui::Color32::WHITE),
            // // });
            // painter.add(egui::Shape::line(
            //     points,
            //     // egui::Stroke::new(1.0, egui::Color32::LIGHT_RED), // Line thickness and color
            //     egui::Stroke::new(1.5, egui::Color32::LIGHT_BLUE), // Line thickness and color
            // ));

            // NOTE:
            // use Shape::line_segment to draw a line between two points (more efficient than Shape::line)
            // use Shape::hline/vline?
            // use Shape::circle_filled for the dots?
            // use Shape::text
            //
            // Shape::transform can be used to apply a transform (scale,translate) to an existing shape
            self.ui_middle_line(ui, model_track, &to_screen);
            self.mouse_hover_info.ui(ui, model_track);
            self.mouse_select.ui(ui);
            // };

            // let response = ui.interact(ui.min_rect(), ui.unique_id(), egui::Sense::hover());
            // if response.contains_pointer() {
            //     ui.ctx().input(|i| {
            //         for event in &i.events {
            //             match event {
            //                 egui::Event::MouseWheel { unit, delta, modifiers } => {
            //                     println!("unit: {:?}, delta: {}, modifiers: {:?}", unit, delta, modifiers);
            //                 }
            //                 _ => (),
            //             }
            //         }
            //     });
            // }
            // });
        });
    }

    fn ui_middle_line(&self, ui: &mut egui::Ui, model_track: &model::track::Track, to_screen: &egui::emath::RectTransform) {
        let min_x = model_track.view_rect().min.x;
        let max_x = model_track.view_rect().max.x;

        let start_point_screen = to_screen.transform_pos(egui::pos2(min_x, 0.0));
        // start_point_screen.x += 10.0;
        let painter = ui.painter();
        painter.line_segment(
            [start_point_screen, to_screen.transform_pos(egui::pos2(max_x, 0.0))]
                .each_ref()
                .map(|pos| painter.round_pos_to_pixel_center(*pos)),
            self.stroke_middle_line,
        );
    }
}

#[derive(Debug)]
struct MouseHover {
    // pub screen_pos: Pos2,
    stroke_vline: egui::Stroke, // for vertical line where mouse pointer is
    pub hovering: bool,
}

impl Default for MouseHover {
    fn default() -> Self {
        Self {
            // screen_pos: Pos2::new(0.0, 0.0),
            stroke_vline: egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(200, 200, 200, 100)),
            hovering: false,
        }
    }
}

impl MouseHover {
    fn ui(&mut self, ui: &mut egui::Ui, model_track: &mut model::track::Track) {
        // TODO: make sure ui.min_rect is indeed the rect we want to use for this?
        let response = ui
            .interact(ui.min_rect(), ui.unique_id(), egui::Sense::hover())
            .on_hover_cursor(egui::CursorIcon::None);

        if !response.contains_pointer() {
            self.hovering = false;

            // TODO:
            // * send hover info to all tracks
            // * each track decides how to render
            //   * when in track: render v and h lines
            //   * when not in track: render h line
            // * figure out how to know the mouse is not hovering over any tracks?
            //   * maybe reset hover info each time before rendering: but then we need to render in
            //     the correct order to have the correct hover info
            //   * maybe tracks needs to reset after all tracks have been rendered, but none have
            //     set hover info? Seems to increase coupling?
            return;
        }
        else
        {
            self.hovering = true;
            let Some(pos) = ui.ctx().pointer_hover_pos() else {
                return;
            };
            model_track.set_hover_info((&pos).into());
        }



        if let Some(hover_info) = model_track.hover_info() {
            // TODO: draw vline when mous in other track
            // Draw crosshair on the mouse position
            {
                if ui.min_rect().contains((&hover_info.screen_pos).into())
                {
                    let vline_start = ui
                        .painter()
                        .round_pos_to_pixel_center(egui::pos2(hover_info.screen_pos.x, ui.min_rect().top()));
                    let vline_end = ui
                        .painter()
                        .round_pos_to_pixel_center(egui::pos2(hover_info.screen_pos.x, ui.min_rect().bottom()));
                    ui.painter().line_segment([vline_start, vline_end], self.stroke_vline);
                }

                let hline_start = ui
                    .painter()
                    .round_pos_to_pixel_center(egui::pos2(ui.min_rect().left(), hover_info.screen_pos.y));
                let hline_end = ui
                    .painter()
                    .round_pos_to_pixel_center(egui::pos2(ui.min_rect().right(), hover_info.screen_pos.y));
                ui.painter().line_segment([hline_start, hline_end], self.stroke_vline);
            }

            // Draw HoverInfor (TODO extract)
            let rect = egui::Rect::from_min_max(
                // TODO: position left of line when at right side of window
                egui::pos2(hover_info.screen_pos.x, ui.min_rect().top()),
                // TODO: calculate width somehow?
                egui::pos2(hover_info.screen_pos.x + 200.0, ui.min_rect().bottom()),
            );
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect), |ui| {
                egui::Frame::popup(ui.style()).outer_margin(10.0).show(ui, |ui| {
                    // ui.label(format!("index: {}", model_track.hover_info().));
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
                        ui.label(format!("index: {}", min_ix));
                        ui.label(format!("value: {}", min_sample.unwrap().1));
                    } else {
                        ui.label(format!("index:  [{}, {}]", min_ix, max_ix));
                        ui.label(format!("min value: {}", min_sample.unwrap().1));
                        ui.label(format!("max value: {}", max_sample.unwrap().1));
                    }
                });
            });
        }
    }
}

#[derive(Debug, Default)]
struct MouseSelect {
    pub drag_start: Option<Pos2>,
    pub drag_end: Option<Pos2>,
}

impl MouseSelect {
    fn ui(&mut self, ui: &mut egui::Ui) {
        // println!("{}: {:?} {:p}", "\nbefore drag_start", self.drag_start, &self.drag_start);
        // println!("{}: {:?} {:p}", "before drag_end", self.drag_end, &self.drag_start);

        let response = ui.interact(ui.min_rect(), ui.unique_id(), egui::Sense::drag());
        if response.contains_pointer() {
            if let Some(id) = ui.ctx().drag_started_id() {
                // println!("drag start_id {}: {:?}", "id", id);
                if let Some(pos) = ui.ctx().pointer_hover_pos() {
                    self.drag_start = Some(pos);
                    // println!("{}: {:?} {:p}", "start drag_start", self.drag_start, &self.drag_start);
                    self.drag_end = None;
                }
            }
            if let Some(id) = ui.ctx().drag_stopped_id() {
                // println!("drag stop_id {}: {:?}", "id", id);
                if let Some(pos) = ui.ctx().pointer_hover_pos() {
                    self.drag_end = Some(pos);
                    // println!("{}: {:?} {:p}", "end drag_start", self.drag_start, &self.drag_start);
                    // println!("{}: {:?} {:p}", "end drag_end", self.drag_end, &self.drag_start);
                }
            }
        }

        // println!("{}: {:?}", "after drag_start", self.drag_start);
        // println!("{}: {:?}", "after drag_end", self.drag_end);

        if let Some(drag_start) = self.drag_start {
            if let Some(drag_end) = self.drag_end {
                ui.painter().rect_filled(
                    egui::Rect::from_two_pos(
                        egui::pos2(drag_start.x, ui.min_rect().top()),
                        egui::pos2(drag_end.x, ui.min_rect().bottom()),
                    ),
                    egui::Rounding::ZERO,
                    egui::Color32::from_rgba_unmultiplied(200, 200, 200, 50),
                );
            } else {
                if let Some(current_mouse_pos) = ui.ctx().pointer_hover_pos() {
                    ui.painter().rect_filled(
                        egui::Rect::from_two_pos(
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
