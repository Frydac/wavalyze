use crate::audio;
use crate::math::round::round_up_to_power_of_10;
use crate::model;
use egui;

pub fn show(ui: &mut egui::Ui, model: &model::SharedModel) {
    let height = 40.0;
    let width = ui.available_width();
    // let rect = egui::Rect::from_x_y_ranges(0.0..=width, 0.0..=height);
    let _stroke = egui::Stroke::new(1.0, egui::Color32::RED);
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

                if model.borrow_mut().tracks.is_empty() {
                    return;
                }
                {
                    let model = model.borrow();
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
                                    last_sample_ix,
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
                                let min_nr_samples_per_tick = sample_width / max_nr_ticks;
                                let nr_samples_per_tick = round_up_to_power_of_10(min_nr_samples_per_tick) as u64;
                                let start_sample_ix = track.sample_rect.ix_rng.start() as u64 / nr_samples_per_tick * nr_samples_per_tick;
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
                                    } else if let Some(screen_x) = track.sample_ix_to_screen_x(cur_sample_ix as audio::SampleIx) {
                                        let y_0 = y_1 - (ruler_rect.height() / 4.0);
                                        let pos_0 = [screen_x, y_0].into();
                                        let pos_1 = [screen_x, y_1].into();
                                        let pos_0 = ui.painter().round_pos_to_pixel_center(pos_0);
                                        let pos_1 = ui.painter().round_pos_to_pixel_center(pos_1);
                                        ui.painter().line_segment([pos_0, pos_1], stroke);
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
            });
    });
}
