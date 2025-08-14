use crate::model;
use crate::model::track;
use crate::view::grid::KeyValueGrid;
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

    pub fn ui(&mut self, ui: &mut egui::Ui, model: &mut model::Model) {
        self.ui_track_header(ui, model);
        self.ui_waveform(ui, model);
    }

    pub fn ui_track_header(&mut self, ui: &mut egui::Ui, model: &mut model::Model) {
        // Track title above the waveform view
        egui::Frame::default()
            .inner_margin(egui::Margin::same(5.0))
            // .outer_margin(egui::Margin::symmetric(0.0, 5.0))
            .outer_margin(egui::Margin {
                left: 0.0,
                right: 0.0,
                top: 5.0,
                bottom: 0.0,
            })
            .stroke(ui.style().visuals.window_stroke())
            .show(ui, |ui| {
                ui.label(&self.name);
            });
    }

    pub fn ui_waveform(&mut self, ui: &mut egui::Ui, model: &mut model::Model) {
        // Frame that contains the waveform drawing
        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.set_max_width(ui.available_width());

            // This gets the absolute position of the canvas
            let canvas_rect = ui.min_rect(); // TODO: don't know for sure what min_rect means in this context
                                             // dbg!(canvas_rect);

            // update model track with current screen rect
            model
                .tracks
                .track_mut(self.id)
                .unwrap()
                .set_screen_rect(canvas_rect.into())
                .unwrap();

            // Draw/interact all the things
            self.ui_start_end(ui, model);
            self.ui_samples(ui, model);
            self.ui_middle_line(ui, model);
            self.mouse_hover_info.ui(ui, model, self.id);
            // self.mouse_select.ui(ui);
        });
    }

    fn ui_start_end(&self, ui: &mut egui::Ui, model: &model::Model) {
        let model_track = model.tracks.track(self.id).unwrap();
        let view_rect = model_track.view_rect();
        let screen_rect = model_track.screen_rect;
        let to_screen = egui::emath::RectTransform::from_to((*view_rect).into(), screen_rect.into());
        // let color = egui::Color32::from_rgba_unmultiplied(200, 200, 200, 10);
        let color = egui::Color32::from_rgb(100, 100, 100);
        let stroke = egui::Stroke::new(1.0, color);

        // start
        if model_track.sample_rect.ix_rng.contains(0.0) {
            let view_x0 = model_track.sample_ix_to_view_x(-model_track.sample_rect.ix_rng.start());
            // dbg!(-model_track.sample_rect.ix_rng.start());
            // dbg!(view_x0);
            let mut screen_x0 = to_screen.transform_pos(egui::pos2(view_x0, 0.0));
            screen_x0.x = screen_x0.x.ceil() - 0.5;
            // dbg!(screen_x0);
            let padding = 10.0;
            let min = egui::pos2(screen_x0.x, screen_rect.top() + padding);
            let max = egui::pos2(screen_x0.x, screen_rect.bottom() - padding);
            ui.painter().line_segment([min, max], stroke);
        }

        // end
        let last_sample_ix = (model_track.buffer.borrow().nr_samples() - 1) as crate::audio::SampleIx;
        if model_track.sample_rect.ix_rng.contains(last_sample_ix) {
            let view_x0 = model_track.sample_ix_to_view_x(last_sample_ix - model_track.sample_rect.ix_rng.start());
            // dbg!(-model_track.sample_rect.ix_rng.start());
            // dbg!(view_x0);
            let mut screen_x0 = to_screen.transform_pos(egui::pos2(view_x0, 0.0));
            screen_x0.x = screen_x0.x.ceil() - 0.5;
            // dbg!(screen_x0);
            let padding = 10.0;
            let min = egui::pos2(screen_x0.x, screen_rect.top() + padding);
            let max = egui::pos2(screen_x0.x, screen_rect.bottom() - padding);
            ui.painter().line_segment([min, max], stroke);
        }
    }

    // TODO: highlight selected/hovered sample(s)
    pub fn ui_samples(&mut self, ui: &mut egui::Ui, model: &mut model::Model) {
        let model_track = model.tracks.track_mut(self.id).unwrap();
        let track_view_rect = model_track.view_rect();

        ui.set_min_size(egui::vec2(ui.available_width(), ui.available_height()));

        let screen_rect = ui.min_rect();
        let to_screen = egui::emath::RectTransform::from_to((*track_view_rect).into(), screen_rect);
        let to_view = to_screen.inverse();
        let line_color = egui::Color32::LIGHT_RED.linear_multiply(0.7).with_alpha(255);
        // let line_color = egui::Color32::from_rgb(128, 64, 64);
        let stroke_line = egui::Stroke::new(1.0, line_color);
        let painter = ui.painter_at(screen_rect);

        match model_track.view_buffer() {
            model::ViewBuffer::SingleSamples(buffer_pos) => {
                for (ix, pos_sample) in buffer_pos.iter().enumerate() {
                    let pos_sample: egui::Pos2 = pos_sample.into();
                    let pos_sample_mid = egui::pos2(pos_sample.x, 0.0);
                    let pos_sample_screen = painter.round_pos_to_pixel_center(to_screen.transform_pos(pos_sample));
                    let pos_sample_mid_screen = painter.round_pos_to_pixel_center(to_screen.transform_pos(pos_sample_mid));

                    painter.line_segment([pos_sample_mid_screen, pos_sample_screen], stroke_line);
                    let circle_size = 1.5;
                    let circle_color = line_color;
                    painter.circle_filled(pos_sample_screen, circle_size, circle_color);
                }
            }
            model::ViewBuffer::OneLine(buffer_pos) => {
                let screen_pos: Vec<Pos2> = buffer_pos
                    .iter()
                    .map(|pos_sample| {
                        let pos_sample = egui::pos2(pos_sample.x, pos_sample.y);
                        // let pos_sample_screen = painter.round_pos_to_pixel_center(to_screen.transform_pos(pos_sample));
                        to_screen.transform_pos(pos_sample)
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

                    // NOTE: swapping min and max, as the Y-axis is inverted in egui
                    let max_screen = painter.round_pos_to_pixel_center(to_screen.transform_pos(min));
                    let min_screen = painter.round_pos_to_pixel_center(to_screen.transform_pos(max));

                    // draw line between samples on the same pixel column
                    painter.line_segment([min_screen, max_screen], stroke_line);
                    // Fill any gaps between samples on subsequent pixel columns
                    // TODO: we might be drawing lines next to each other where not necessary,
                    // probably need to adjust prev_min_y and prev_max_y to account for this
                    if prev_max_y < min_screen.y {
                        painter.line_segment([egui::pos2(min_screen.x, prev_max_y), min_screen], stroke_line);
                    } else if prev_min_y > max_screen.y {
                        painter.line_segment([max_screen, egui::pos2(max_screen.x, prev_min_y)], stroke_line);
                    }
                    prev_max_y = max_screen.y;
                    prev_min_y = min_screen.y;
                }
            }
        }
        // });
    }

    // fn ui_middle_line(&self, ui: &mut egui::Ui, model_track: &model::track::Track, to_screen: &egui::emath::RectTransform) {
    fn ui_middle_line(&self, ui: &mut egui::Ui, model: &model::Model) {
        let model_track = model.tracks.track(self.id).unwrap();
        let min_x = model_track.view_rect().min.x;
        let max_x = model_track.view_rect().max.x;
        let track_view_rect = model_track.view_rect();
        let screen_rect = ui.min_rect();
        let to_screen = egui::emath::RectTransform::from_to((*track_view_rect).into(), screen_rect);

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
}

impl Default for MouseHover {
    fn default() -> Self {
        Self {
            // screen_pos: Pos2::new(0.0, 0.0),
            stroke_vline: egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(200, 200, 200, 100)),
        }
    }
}

impl MouseHover {
    fn ui_sample_info_floating_rect2(&mut self, ui: &mut egui::Ui, track_id: Id, hover_info: &track::HoverInfo) {
        if hover_info.samples.is_empty() {
            return;
        }

        // Use the track_id to create a unique ID for each popup instance.
        let popup_id = ui.id().with(track_id).with("sample_info_popup");
        let last_size: Option<egui::Vec2> = ui.ctx().memory_mut(|m| m.data.get_persisted::<egui::Vec2>(popup_id));

        let canvas_rect = ui.min_rect();

        // Default position is to the right of the cursor.
        let offset_x = 10.0; // Small offset from the mouse pointer
        let mut popup_pos = egui::pos2(hover_info.screen_pos.x + offset_x, canvas_rect.top());

        // If we have the size from the last frame, use it to check for screen overflow.
        if let Some(size) = last_size {
            if hover_info.screen_pos.x + offset_x + size.x > canvas_rect.right() {
                // It would overflow, so place it to the left of the cursor instead.
                popup_pos.x = hover_info.screen_pos.x - size.x - offset_x;
            }
        }

        // Use an egui::Area to place the popup at the calculated position.
        let area_response = egui::Area::new(popup_id)
            .fixed_pos(popup_pos)
            .interactable(false) // Add this line
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style()).outer_margin(10.0).show(ui, |ui| {
                    // --- The content of the popup remains the same ---
                    let mut min_sample = Option::<(i32, f32)>::None;
                    let mut max_sample = Option::<(i32, f32)>::None;
                    for (ix, sample) in hover_info.samples.iter() {
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

                    let min_ix = hover_info.samples.first().unwrap().0;
                    let max_ix = hover_info.samples.last().unwrap().0;

                    let mut grid = KeyValueGrid::new(track_id);
                    if min_ix == max_ix {
                        grid.row("index:", format!("{}", min_ix));
                        grid.row("value:", format!("{}", min_sample.unwrap().1));
                    } else {
                        grid.row("indices:", format!("[{}, {}]", min_ix, max_ix));
                        grid.row("min value:", format!("{}", min_sample.unwrap().1));
                        grid.row("max value:", format!("{}", max_sample.unwrap().1));
                    }
                    grid.show(ui);
                });
            });

        // Store the size of the popup for the next frame.
        ui.ctx()
            .memory_mut(|m| m.data.insert_persisted(popup_id, area_response.response.rect.size()));
    }

    fn ui_sample_info_floating_rect(&mut self, ui: &mut egui::Ui, track_id: Id, hover_info: &track::HoverInfo) {
        if hover_info.samples.is_empty() {
            return;
        }

        let canvas_rect = ui.min_rect();
        // Draw HoverInfor (TODO extract)
        let width = 120.0;
        // let left_x = if  hover_info.screen_pos.x > canvas_rect.right() - width {
        //     hover_info.screen_pos.x - width
        // } else {
        //     hover_info.screen_pos.x
        // };
        let left_x = hover_info.screen_pos.x;
        egui::Window::new("")
            .id(ui.id().with(track_id).with("popup"))
            .fixed_pos(egui::pos2(left_x, canvas_rect.top()))
            .resizable(false)
            .collapsible(false)
            .title_bar(false)
            .frame(egui::Frame::popup(ui.style()).outer_margin(10.0))
            .show(ui.ctx(), |ui| {
                ui.set_max_width(width);
                // min/max sample value under mouse pointer
                let mut min_sample = Option::<(i32, f32)>::None;
                let mut max_sample = Option::<(i32, f32)>::None;
                for (ix, sample) in hover_info.samples.iter() {
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

                // min/max sample index
                let min_ix = hover_info.samples.first().unwrap().0;
                let max_ix = hover_info.samples.last().unwrap().0;

                // let mut grid = KeyValueGrid::new("sample_info_grid");
                // if min_ix == max_ix {
                //     grid.row("index:", format!("{}", min_ix));
                //     grid.row("value:", format!("{}", min_sample.unwrap().1));
                // } else {
                //     grid.row("indices:", format!("[{}, {}]", min_ix, max_ix));
                //     grid.row("min value:", format!("{}", min_sample.unwrap().1));
                //     grid.row("max value:", format!("{}", max_sample.unwrap().1));
                // }
                // grid.show(ui);
            });
    }

    fn ui_mouse_pos_vline(&mut self, ui: &mut egui::Ui, hover_info: &track::HoverInfo, canvas_rect: &egui::Rect) {
        let painter = ui.painter();
        let x = hover_info.screen_pos.x;
        let vline_start = painter.round_pos_to_pixel_center((x, canvas_rect.top()).into());
        let vline_end = painter.round_pos_to_pixel_center((x, canvas_rect.bottom()).into());
        ui.painter().line_segment([vline_start, vline_end], self.stroke_vline);
    }

    fn ui_mouse_pos_hline(&mut self, ui: &mut egui::Ui, hover_info: &track::HoverInfo, canvas_rect: &egui::Rect) {
        let painter = ui.painter();
        let y = hover_info.screen_pos.y;
        let hline_start = painter.round_pos_to_pixel_center((canvas_rect.left(), y).into());
        let hline_end = painter.round_pos_to_pixel_center((canvas_rect.right(), y).into());
        ui.painter().line_segment([hline_start, hline_end], self.stroke_vline);
    }

    // fn ui(&mut self, ui: &mut egui::Ui, model_track: &mut model::track::Track) {
    fn ui(&mut self, ui: &mut egui::Ui, model: &mut model::Model, track_id: Id) {
        let canvas_rect = ui.min_rect();

        let hover_response = ui
            .interact(canvas_rect, egui::Id::new(track_id), egui::Sense::hover())
            .on_hover_cursor(egui::CursorIcon::None);

        // NOTE: we don't check hovered or contains_pointer as the popup seems to overtake the
        // mouse event and when moving fast, the mouse can be over the popup, causing flickering of
        // the popup.
        if let Some(pos) = ui.ctx().pointer_hover_pos() {
            if canvas_rect.contains(pos) {
                model.tracks.update_hover_info(track_id, (&pos).into());

                ui.ctx().input(|i| {
                    if i.modifiers.shift {
                        let scroll = i.raw_scroll_delta;
                        if scroll.x != 0.0 {
                            model.tracks.shift_x(scroll.x).unwrap();
                        }
                    } else if i.modifiers.ctrl {
                        let scroll = i.raw_scroll_delta;
                        if scroll.y != 0.0 {
                            let factor = model.config.zoom_x_factor;
                            model.tracks.zoom_x(pos.x, scroll.y * factor).unwrap();
                        }
                    }
                });
            }
        }

        let model_track = model.tracks.track_mut(track_id).unwrap();
        if let Some(hover_info) = model_track.hover_info() {
            // --- Logging for debugging flickering ---
            // dbg!(track_id, hover_info.samples.is_empty());
            // --- End logging ---

            // if let Some(hover_info) = model.tracks.tracks_hover_info.previous {

            // Draw vertical line/sample info where mouse pointer is for all tracks, when mouse is
            // over any track
            if canvas_rect.x_range().contains(hover_info.screen_pos.x) {
                self.ui_mouse_pos_vline(ui, hover_info, &canvas_rect);

                // Draw horizontal line where mouse pointer is for only the track the mouse is over
                if canvas_rect.y_range().contains(hover_info.screen_pos.y) {
                    self.ui_mouse_pos_hline(ui, hover_info, &canvas_rect);
                }
                self.ui_sample_info_floating_rect2(ui, track_id, hover_info);
            }
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
            } else if let Some(current_mouse_pos) = ui.ctx().pointer_hover_pos() {
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
