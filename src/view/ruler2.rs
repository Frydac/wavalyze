use anyhow::Result;

use crate::{
    audio::sample,
    math::round::round_up_to_power_of_10,
    model::{
        self,
        ruler::{self, ix_lattice::TickType},
    },
};

/// round to pixel center
pub fn rpc(ui: &egui::Ui, pos: egui::Pos2) -> egui::Pos2 {
    let pos = ui.painter().round_pos_to_pixel_center(pos);
    pos
}

// TODO: can we get away with only passing the ruler?
pub fn ui(ui: &mut egui::Ui, model: &model::SharedModel) -> Result<()> {
    let height = 50.0;
    let width = ui.available_width();
    ui.allocate_ui([width, height].into(), |ui| {
        egui::Frame::default()
            .inner_margin(egui::Margin::same(5.0))
            .rounding(3.0)
            .outer_margin(egui::Margin::symmetric(0.0, 5.0))
            .stroke(ui.style().visuals.window_stroke())
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                let rect = ui.min_rect();

                let mut model = model.borrow_mut();
                let ruler = &mut model.tracks2.time_line;
                ruler.set_screen_rect(rect.into());

                // for demo
                {
                    if !ruler.valid() {
                        ruler.zoom_to_ix_range(sample::FracIxRange { start: -180.0, end: 800.0 });
                    }
                    // if ruler.hover_info.is_none() {
                    //     let sample_ix = -65;
                    //     let screen_x = ruler.sample_ix_to_screen_x(sample_ix as f64).unwrap_or(rect.left() + 123.0);
                    //     ruler.hover_info = Some(ruler::time::HoverInfo { sample_ix, screen_x });
                    // }
                }

                // When not initialized, we don't draw anything anymore
                let Some(ix_range) = ruler.ix_range() else { return };


                ruler.hover_info = ui.ctx()
                    .pointer_hover_pos()
                    .filter(|&pos| rect.contains(pos))
                    .and_then(|pos| {
                        ruler.screen_x_to_sample_ix(pos.x).map(|sample_ix| {
                            ruler::time::HoverInfo {
                                sample_ix: sample_ix.floor() as i64,
                                screen_x: pos.x,
                            }
                        })
                    });

                // NOTE: I like this better, but is still in rust unstable, maybe after updating
                // this could work:
                // if let Some(pos) = ui.ctx().pointer_hover_pos()
                //     && rect.contains(pos)
                //     && let Some(sample_ix) = ruler.screen_x_to_sample_ix(pos.x)
                // {
                //     ruler.hover_info = Some(ruler::time::HoverInfo {
                //         sample_ix: sample_ix.floor() as i64,
                //         screen_x: pos.x,
                //     });
                // } else {
                //     ruler.hover_info = None;
                // }

                // ui_start_end(ui, ix_range);
                // ui_grid_10(ui, ruler);
                ui_grid_10_2(ui, ruler);
                let _ = ui_hover_info(ui, ruler);
            });
    });
    Ok(())
}

fn ui_hover_info(ui: &mut egui::Ui, ruler: &ruler::Time) -> Result<()> {
    let Some(hover_info) = ruler.hover_info else { return Ok(()) };

    let screen_x = hover_info.screen_x;
    let rect_x_range = ui.min_rect().left()..ui.min_rect().right();
    if !rect_x_range.contains(&screen_x) {
        anyhow::bail!("screen_x {} not in rect {:?}", screen_x, rect_x_range);
    }
    ui_tick_line(ui, screen_x, TICK_HEIGHT_LONG);

    let height = ui.min_rect().height() * TICK_HEIGHT_LONG;
    let side = 7.5;
    let screen_y_top = ui.min_rect().bottom() - height - side;
    let screen_y_bottom = ui.min_rect().bottom() - height + 1.0;
    let points: Vec<egui::Pos2> = vec![
        [screen_x - side, screen_y_top].into(),
        [screen_x + 0.0, screen_y_top].into(),
        [screen_x, screen_y_bottom].into(),
    ];
    // ui.painter().convex_polygon(points, ui.style().visuals.text_color(), Stroke::new(1.0, Color32::WHITE));
    let color = ui.style().visuals.text_color();
    ui.painter()
        .add(egui::Shape::convex_polygon(points, color, egui::Stroke::new(1.0, color)));
    let text_pos = [screen_x + 2.0, ui.min_rect().top()].into();
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_TOP,
        hover_info.sample_ix,
        egui::FontId::default(),
        ui.style().visuals.text_color(),
    );
    // if !rect.into<Rect>().contains_x(screen_x) {
    //     anyhow::bail!("screen_x {} not in rect {:?}", screen_x, rect);
    // }

    Ok(())
}

fn ui_tick_line(ui: &mut egui::Ui, screen_x: f32, height_frac: f32) {
    let rect = ui.min_rect();
    let height = rect.height() * height_frac;
    let pos_0 = rpc(ui, [screen_x, rect.bottom() - height].into());
    let pos_1 = rpc(ui, [screen_x, rect.bottom()].into());
    ui.painter().line_segment([pos_0, pos_1], (1.0, ui.style().visuals.text_color()));
}

// fraction of pixel rect height
const TICK_HEIGHT_LONG: f32 = 0.5;
const TICK_HEIGHT_MID: f32 = 0.3;
const TICK_HEIGHT_SHORT: f32 = 0.2;

// TODO: good idea, but need to not draw a tick line, and store the rects where not to draw the
// other labels
fn ui_start_end(ui: &mut egui::Ui, ix_range: sample::FracIxRange) {
    let rect = ui.min_rect();
    // draw start tick
    {
        ui_tick_line(ui, rect.left(), TICK_HEIGHT_LONG);

        let text_pos = rect.left_top() + egui::vec2(5.0, -5.0);
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_TOP,
            ix_range.start,
            egui::FontId::default(),
            ui.style().visuals.text_color(),
        );
    }

    // draw end tick
    {
        ui_tick_line(ui, rect.right(), TICK_HEIGHT_LONG);

        let text_pos = rect.right_top() + egui::vec2(-5.0, -5.0);
        ui.painter().text(
            text_pos,
            egui::Align2::RIGHT_TOP,
            ix_range.end.floor() as u64,
            egui::FontId::default(),
            ui.style().visuals.text_color(),
        );
    }
}

/// Indicates how many pixels apart the grid lines should be (at least)
pub const NR_PIXELS_PER_TICK: f32 = 100.0;

fn ui_grid_10_2(ui: &mut egui::Ui, ruler: &mut model::ruler::Time) {
    let pixel_rect = ui.min_rect();
    let Some(ix_lattice) = ruler.ix_lattice() else { return };
    for tick in &ix_lattice.ticks {
        // draw tick line with appropriate height
        let tick_height = match tick.tick_type {
            TickType::Labeled => TICK_HEIGHT_LONG,
            TickType::Mid => TICK_HEIGHT_MID,
            TickType::Small => TICK_HEIGHT_SHORT,
        };
        ui_tick_line(ui, tick.screen_x, tick_height);

        // draw tick label if needed
        if tick.tick_type == TickType::Labeled {
            let text_pos = [tick.screen_x + 2.0, pixel_rect.top() - 2.0].into();
            ui.painter().text(
                text_pos,
                egui::Align2::LEFT_TOP,
                tick.sample_ix,
                egui::FontId::proportional(14.0),
                ui.style().visuals.text_color(),
            );
        }
    }
}

/// Draws a grid with labels at numbers that are a power of 10 (inclusive 10^0, i.e. 1, 2, 3... or
/// 10, 20, 30.., 100, 200, 300.., etc.) with 0 or 10 ticks between each label, and a longer tick
/// on the 50% mark between labeled ticks.
/// TODO: we should extract testable functionality here, with an intermediate representation?
fn ui_grid_10(ui: &mut egui::Ui, ruler: &model::ruler::Time) {
    let Some(sample_ix_range) = ruler.ix_range() else { return };
    let pixel_rect = ui.min_rect();
    let max_nr_ticks: f32 = pixel_rect.width() / NR_PIXELS_PER_TICK;
    let sample_width = sample_ix_range.len();

    if sample_width == 0.0 {
        tracing::warn!("sample width is zero, cannot draw grid");
        return;
    }

    let min_nr_samples_per_label_tick: f64 = sample_width / max_nr_ticks as f64;
    let mut nr_samples_per_label_tick: u64 = round_up_to_power_of_10(min_nr_samples_per_label_tick) as u64;
    // When very zoomed in, this could be smaller than 1, but we want to have at least 1 tick per
    // sample
    if nr_samples_per_label_tick == 0 {
        nr_samples_per_label_tick = 1;
    }
    let start_sample_ix = sample_ix_range.start as u64 / nr_samples_per_label_tick * nr_samples_per_label_tick;

    // Draw ticks with labels
    let mut cur_sample_ix = start_sample_ix;
    while cur_sample_ix < sample_ix_range.end as u64 {
        let Some(screen_x) = ruler.sample_ix_to_screen_x(cur_sample_ix as f64) else {
            break;
        };
        ui_tick_line(ui, screen_x, TICK_HEIGHT_LONG);
        let text_pos = [screen_x + 2.0, pixel_rect.top()].into();
        ui.painter().text(
            text_pos,
            egui::Align2::LEFT_TOP,
            cur_sample_ix as u64,
            egui::FontId::default(),
            ui.style().visuals.text_color(),
        );

        cur_sample_ix += nr_samples_per_label_tick;
    }

    if nr_samples_per_label_tick == 1 {
        // We are done, no intermediate ticks
        return;
    }

    // Draw ticks without labels
    let nr_samples_per_small_tick = (nr_samples_per_label_tick as f64 * 0.1) as u64;
    assert!(nr_samples_per_small_tick > 0); // We know it should be a power of 10 and not 0, or 1
    for cur_sample_ix in (0..sample_ix_range.end as u64)
        .step_by(nr_samples_per_small_tick as usize)
        .skip_while(|cur_sample_ix| {
            // We already drawn this with a label
            cur_sample_ix % nr_samples_per_label_tick == 0
        })
    {
        let Some(screen_x) = ruler.sample_ix_to_screen_x(cur_sample_ix as f64) else {
            break;
        };

        if cur_sample_ix % (nr_samples_per_label_tick / 2) == 0 {
            // We are half way between two labels, i.e. tick with ix 5
            ui_tick_line(ui, screen_x, TICK_HEIGHT_MID);
        } else {
            ui_tick_line(ui, screen_x, TICK_HEIGHT_SHORT);
        }
    }
}
