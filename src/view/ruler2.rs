use crate::{
    audio::sample,
    model::{
        self,
        ruler::{self},
    },
};
use anyhow::Result;
use thousands::Separable;

/// round to pixel center (TODO: move to somehwere more general)
pub fn rpc(ui: &egui::Ui, pos: egui::Pos2) -> egui::Pos2 {
    let pos = ui.painter().round_pos_to_pixel_center(pos);
    pos
}

pub fn interaction_handle_drag(ui: &mut egui::Ui, response: &egui::Response, model: &model::SharedModel) {
    if response.dragged() {
        let delta = ui.input(|i| i.pointer.delta());
        model
            .borrow_mut()
            .actions
            .push(model::action::Action::ShiftX { nr_pixels: -delta.x });
    }
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
                // Don't let the size depend on the content
                ui.set_min_size(ui.available_size());
                //
                let response = ui.allocate_rect(ui.min_rect(), egui::Sense::click_and_drag());

                // Not 100% sure this is the right way to get the actual rect, but it seems to work.
                // porbably only works when we do the set_min_size() above?
                let rect = ui.min_rect();

                // handle interactions
                interaction_handle_drag(ui, &response, model);

                // TODO: probably don't need the whole model (also need user_config)
                let mut model = model.borrow_mut();

                {
                    let ruler = &mut model.tracks2.ruler;
                    ruler.set_screen_rect(rect.into());

                    // for demo, should come from loaded file?
                    // {
                    //     if !ruler.valid() {
                    //         tracing::warn!("Ruler not valid setting to demo range");
                    //         ruler.zoom_to_ix_range(sample::FracIxRange {
                    //             start: -180.0,
                    //             end: 900_000.0,
                    //         });
                    //     }
                    // }
                }

                let pos_in_rect = ui.ctx().pointer_hover_pos().filter(|&pos| rect.contains(pos));
                match pos_in_rect {
                    Some(pos) => {
                        ui.ctx().input(|i| {
                            if i.modifiers.shift && !i.modifiers.ctrl {
                                let scroll = i.raw_scroll_delta;
                                if scroll.x != 0.0 {
                                    let zoom_x_factor = model.user_config.zoom_x_scroll_factor;
                                    // model.actions.push(model::action::Action::ShiftX { nr_pixels: scroll.x });
                                    model.actions.push(model::action::Action::ShiftX { nr_pixels: scroll.x });
                                }
                            } else if i.modifiers.ctrl && !i.modifiers.shift {
                                let scroll = i.raw_scroll_delta;
                                if scroll.y != 0.0 {
                                    let zoom_x_factor = model.user_config.zoom_x_scroll_factor;
                                    // model.actions.push(model::action::Action::ZoomX {
                                    model.actions.push(model::action::Action::ZoomX {
                                        nr_pixels: scroll.y * zoom_x_factor,
                                        center_x: pos.x,
                                    });
                                }
                            }
                        });

                        // If mouse hovers over the ruler
                        ui.ctx().set_cursor_icon(egui::CursorIcon::None);
                        // hover_info could still be None if we don't have a sample_ix range yet
                        let ruler = &mut model.tracks2.ruler;
                        ruler.hover_info = ruler.screen_x_to_sample_ix(pos.x).map(|sample_ix| ruler::time::HoverInfo {
                            sample_ix: sample_ix.round() as i64,
                            screen_x: pos.x,
                        });
                    }
                    None => {
                        model.tracks2.ruler.hover_info = None;
                    }
                }

                let hover_text_rect = ui_hover_tick_label(ui, &model.tracks2.ruler);
                // let begin_end_rects = ui_begin_end(ui, ruler);
                ui_ix_lattice(ui, &mut model.tracks2.ruler, hover_text_rect);
                // NOTE: we want this later so that the triangle is on top of the ix_lattice ticks
                ui_hover_tick_line_triangle(ui, &model.tracks2.ruler);
            });
    });
    Ok(())
}

fn ui_hover_tick_label(ui: &mut egui::Ui, ruler: &ruler::Time) -> Option<egui::Rect> {
    let &hover_info = ruler.hover_info.as_ref()?;
    ui_tick_label(ui, hover_info.screen_x, hover_info.sample_ix.separate_with_commas().into(), None)
}

/// @return the pixel rect of the text label, so we can avoid drawing other text over it
fn ui_hover_tick_line_triangle(ui: &mut egui::Ui, ruler: &ruler::Time) {
    let Some(&hover_info) = ruler.hover_info.as_ref() else { return };
    let screen_x = hover_info.screen_x;
    let rect_x_range = ui.min_rect().left()..ui.min_rect().right();
    if !rect_x_range.contains(&screen_x) {
        // anyhow::bail!("screen_x {} not in rect {:?}", screen_x, rect_x_range);
        tracing::trace!("screen_x {} not in rect {:?}", screen_x, rect_x_range);
        return;
    }
    let color = egui::Color32::LIGHT_BLUE;
    ui_tick_line(ui, screen_x, TICK_HEIGHT_LONG - 2.0, Some(color));

    // draw triangle (TODO: not sure if this is the way to go :))
    {
        let height = TICK_HEIGHT_LONG;
        let side = 10.0; // size of triangle side and height
        let screen_y_top = ui.min_rect().bottom() - height - 2.0;
        let screen_y_bottom = screen_y_top + side;
        let points: Vec<egui::Pos2> = [
            [screen_x - side / 2.0, screen_y_top].into(),
            [screen_x + side / 2.0, screen_y_top].into(),
            [screen_x, screen_y_bottom].into(),
        ]
        .into_iter()
        .map(|pos| rpc(ui, pos))
        .collect();
        // let color = ui.style().visuals.text_color();
        // let color = egui::Color32::LightBLUE;
        ui.painter()
            .add(egui::Shape::convex_polygon(points, color, egui::Stroke::new(0.0, color)));
    }

    // ui_tick_label(ui, hover_info.screen_x, hover_info.sample_ix.separate_with_commas().into(), None)
}

fn format_compact(n: i64) -> String {
    format_compact_exact(n, 2)
}

fn format_compact_exact(n: i64, max_decimals: usize) -> String {
    const BASE: i64 = 1000;
    const SUFFIXES: [&str; 7] = ["", "k", "M", "G", "T", "P", "E"];

    let abs_n = n.abs();

    let cur_max_decimals = max_decimals;

    for exp in (1..SUFFIXES.len()).rev() {
        let scale = BASE.pow(exp as u32);
        if abs_n < scale {
            continue;
        }

        let q = n / scale;
        let r = n % scale;

        // Case 1: exact integer representation
        if r == 0 {
            return format!("{}{}", q.separate_with_commas(), SUFFIXES[exp]);
        }

        // Case 2: exact fractional representation (up to max_decimals)
        let mut rem = r.abs();
        let mut frac_digits = Vec::new();

        for _ in 0..max_decimals {
            rem *= 10;
            let digit = rem / scale;
            rem %= scale;

            let digit_char = (b'0' + digit as u8) as char;
            frac_digits.push(digit_char);

            if rem == 0 {
                let frac: String = frac_digits.iter().collect();
                return format!("{}.{}{}", q.separate_with_commas(), frac, SUFFIXES[exp]);
            }
        }
    }

    // Fallback: full precision
    n.separate_with_commas()
}

enum TickLabel {
    SampleIx(i64),
    Text(String),
}

impl From<i64> for TickLabel {
    fn from(value: i64) -> Self {
        TickLabel::SampleIx(value)
    }
}
impl From<String> for TickLabel {
    fn from(value: String) -> Self {
        TickLabel::Text(value)
    }
}

/// Draws the text label for a tick, makes sure it doesn't go out of bounds.
/// Returns `None` if the text would overlap with an existing rect
// fn ui_tick_label(ui: &mut egui::Ui, tick: &Tick, existing_rects: Option<&[egui::Rect]>) -> Option<egui::Rect> {
fn ui_tick_label(ui: &mut egui::Ui, screen_x: f32, text: TickLabel, existing_rects: Option<&[egui::Rect]>) -> Option<egui::Rect> {
    let font_id = egui::FontId::proportional(14.0);
    let color = ui.style().visuals.text_color();
    // let text = sample_ix.separate_with_commas();
    // let text = format_compact(sample_ix).to_string();
    let text = match text {
        TickLabel::SampleIx(sample_ix) => format_compact(sample_ix),
        TickLabel::Text(text) => text,
    };
    // let text = sample_ix.to_string();
    let galley = ui.fonts(|fonts| fonts.layout_no_wrap(text, font_id, color));
    let text_size = galley.size();
    let mut text_pos: egui::Pos2 = [screen_x - (text_size.x / 2.0), ui.min_rect().top() - 2.0].into();
    if text_pos.x + text_size.x > ui.min_rect().right() {
        // it would overflow, so place it to the left of the cursor instead.
        text_pos.x = ui.min_rect().right() - text_size.x - 2.0;
    } else if text_pos.x < ui.min_rect().left() {
        text_pos.x = ui.min_rect().left() + 2.0;
    }
    let text_rect = egui::Rect::from_min_size(text_pos, text_size);
    if let Some(rects) = existing_rects {
        for rect in rects {
            if rect.intersects(text_rect) {
                return None;
            }
        }
    }
    ui.painter().galley(text_pos, galley, color);
    Some(text_rect)
}

fn ui_tick_line(ui: &mut egui::Ui, screen_x: f32, height: f32, color: Option<egui::Color32>) {
    let rect_bottom = ui.min_rect().bottom();
    let pos_top = rpc(ui, [screen_x, rect_bottom - height].into());
    let pos_bottom = rpc(ui, [screen_x, rect_bottom].into());
    let color = color.unwrap_or(ui.style().visuals.text_color());
    // let width = height == TICK_HEIGHT_LONG ? 2.0 : 1.0;
    // let width = if height == TICK_HEIGHT_LONG { 1.5 } else { 1.0 };
    let width = 1.0;
    ui.painter().line_segment([pos_top, pos_bottom], (width, color));
}

const TICK_HEIGHT_LONG: f32 = 13.0;
const TICK_HEIGHT_MID: f32 = 8.0;
const TICK_HEIGHT_SHORT: f32 = 4.0;

// TODO: good idea, but need to not draw a tick line?, and store the rects where not to draw the
// other labels
fn ui_start_end(ui: &mut egui::Ui, ix_range: sample::FracIxRange) {
    let rect = ui.min_rect();
    // draw start tick
    {
        ui_tick_line(ui, rect.left(), TICK_HEIGHT_LONG, None);

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
        ui_tick_line(ui, rect.right(), TICK_HEIGHT_LONG, None);

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
pub const NR_PIXELS_PER_TICK: f32 = 50.0;

fn ui_ix_lattice(ui: &mut egui::Ui, ruler: &mut model::ruler::Time, hover_text_rect: Option<egui::Rect>) {
    let Some(ix_lattice) = ruler.ix_lattice() else { return };

    let mut existing_rects = Vec::new();
    if let Some(hover_text_rect) = hover_text_rect {
        existing_rects.push(hover_text_rect);
    }

    for tick in &ix_lattice.ticks {
        // draw tick line with appropriate height
        let tick_height = match tick.tick_type {
            ruler::TickType::Labeled => TICK_HEIGHT_LONG,
            ruler::TickType::Mid => TICK_HEIGHT_MID,
            ruler::TickType::Small => TICK_HEIGHT_SHORT,
        };
        ui_tick_line(ui, tick.screen_x, tick_height, None);

        // draw tick label if needed
        if tick.tick_type == ruler::TickType::Labeled {
            let rect = ui_tick_label(ui, tick.screen_x, tick.sample_ix.into(), Some(existing_rects.as_slice()));
            if let Some(rect) = rect {
                existing_rects.push(rect);
            }
        }
    }

    // after drawing all the ticks with labels, lets see if we can draw more labels that fit
    for tick in &ix_lattice.ticks {
        if tick.tick_type == ruler::TickType::Mid {
            let rect = ui_tick_label(ui, tick.screen_x, tick.sample_ix.into(), Some(existing_rects.as_slice()));
            if let Some(rect) = rect {
                existing_rects.push(rect);
            }
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// InfoPanel
////////////////////////////////////////////////////////////////////////////////

pub fn ui_ruler_info_panel(ui: &mut egui::Ui, ruler: &ruler::Time) {
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.heading("Ruler Info");
            ui.separator();

            let mut grid = crate::view::grid::KeyValueGrid::new(12345);
            {
                let rect = ruler.screen_rect();
                grid.row(
                    "screen rect:",
                    format!("[{:.1}, {:.1}, {:.1}, {:.1}]", rect.min.x, rect.min.y, rect.max.x, rect.max.y),
                );
            }
            if let Some(time_line) = ruler.time_line.as_ref() {
                grid.row("samples per pixel:", format!("{:.3}", time_line.samples_per_pixel));
                let ix_range = time_line.get_ix_range(ruler.screen_rect().width() as f64);
                let ix_range_start = format!("{:.1}", ix_range.start).separate_with_commas();
                let ix_range_end = format!("{:.1}", ix_range.end).separate_with_commas();
                grid.row("ix range:", format!("[{ix_range_start}, {ix_range_end}]"));
            }
            grid.show(ui);
        });
    });
}

pub fn ui_hover_info_panel(ui: &mut egui::Ui, hover_info: Option<&ruler::time::HoverInfo>) {
    ui.group(|ui| {
        ui.vertical(|ui| {
            ui.heading("Hover Info");
            ui.separator();
            match hover_info {
                Some(hover_info) => {
                    let id: u64 = ui.id().with("hover_info_panel").value();
                    let mut grid = crate::view::grid::KeyValueGrid::new(id);
                    grid.row("pos x:", format!("{:.1}", hover_info.screen_x).separate_with_commas());
                    // grid.row("sample ix:", format!("{:.1}", hover_info.sample_ix));
                    grid.row("sample ix:", hover_info.sample_ix.separate_with_commas());
                    grid.show(ui);
                }
                None => {
                    ui.label("No hover info");
                }
            }
        });
    });
}
