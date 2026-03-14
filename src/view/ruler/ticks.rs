use crate::{
    audio::sample,
    model::{self, ruler},
    view::util::{rp, rpc},
};
use thousands::Separable;

/// Indicates how many pixels apart the grid lines should be (at least)
pub const NR_PIXELS_PER_TICK: f32 = 50.0;
pub(crate) const TICK_HEIGHT_LONG: f32 = 13.0;
const TICK_HEIGHT_MID: f32 = 8.0;
const TICK_HEIGHT_SHORT: f32 = 4.0;

#[derive(Clone)]
pub(crate) enum TickLabel {
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

#[derive(Clone)]
struct TickLabelLayout {
    galley: std::sync::Arc<egui::Galley>,
    text_pos: egui::Pos2,
    text_rect: egui::Rect,
    color: egui::Color32,
}

pub(crate) enum TriangleType {
    Left,
    Right,
    #[allow(dead_code)]
    Full,
}

pub(crate) fn ui_selection_tick_label_pair(
    ui: &mut egui::Ui,
    left: (f32, TickLabel),
    right: (f32, TickLabel),
    existing_rects: &[egui::Rect],
) -> Option<Vec<egui::Rect>> {
    let natural_left_layout = layout_tick_label(ui, left.0, left.1.clone());
    let natural_right_layout = layout_tick_label(ui, right.0, right.1.clone());

    let natural_pair_fits = !natural_left_layout
        .text_rect
        .intersects(natural_right_layout.text_rect);
    let natural_left_is_free = !existing_rects
        .iter()
        .any(|rect| rect.intersects(natural_left_layout.text_rect));
    let natural_right_is_free = !existing_rects
        .iter()
        .any(|rect| rect.intersects(natural_right_layout.text_rect));

    if natural_pair_fits {
        let mut drawn_rects = Vec::new();
        if natural_left_is_free {
            paint_tick_label(ui, &natural_left_layout, true);
            drawn_rects.push(natural_left_layout.text_rect);
        }
        if natural_right_is_free {
            paint_tick_label(ui, &natural_right_layout, true);
            drawn_rects.push(natural_right_layout.text_rect);
        }
        return (!drawn_rects.is_empty()).then_some(drawn_rects);
    }

    const LABEL_GAP: f32 = 2.0;

    let mut left_layout = layout_tick_label(ui, left.0, left.1);
    let mut right_layout = layout_tick_label(ui, right.0, right.1);
    let mid_x = (left.0 + right.0) * 0.5;

    let left_target_right = mid_x - LABEL_GAP * 0.5;
    let right_target_left = mid_x + LABEL_GAP * 0.5;

    set_tick_label_right(ui, &mut left_layout, left_target_right);
    set_tick_label_left(ui, &mut right_layout, right_target_left);

    let left_min = ui.min_rect().left();
    let right_max = ui.min_rect().right();
    let pair_fits = !left_layout.text_rect.intersects(right_layout.text_rect)
        && left_layout.text_rect.left() >= left_min
        && right_layout.text_rect.right() <= right_max;

    if !pair_fits {
        let total_width =
            left_layout.text_rect.width() + LABEL_GAP + right_layout.text_rect.width();
        if total_width > ui.min_rect().width() {
            return None;
        }

        if left_target_right - left_layout.text_rect.width() < left_min {
            set_tick_label_left(ui, &mut left_layout, left_min);
            set_tick_label_left(
                ui,
                &mut right_layout,
                left_layout.text_rect.right() + LABEL_GAP,
            );
        } else {
            let right_width = right_layout.text_rect.width();
            set_tick_label_left(ui, &mut right_layout, right_max - right_width);
            set_tick_label_right(
                ui,
                &mut left_layout,
                right_layout.text_rect.left() - LABEL_GAP,
            );
        }
    }

    if left_layout.text_rect.intersects(right_layout.text_rect) {
        return None;
    }

    let left_is_free = !existing_rects
        .iter()
        .any(|rect| rect.intersects(left_layout.text_rect));
    let right_is_free = !existing_rects
        .iter()
        .any(|rect| rect.intersects(right_layout.text_rect));

    let mut drawn_rects = Vec::new();
    if left_is_free {
        paint_tick_label(ui, &left_layout, true);
        drawn_rects.push(left_layout.text_rect);
    }
    if right_is_free && left_is_free {
        paint_tick_label(ui, &right_layout, true);
        drawn_rects.push(right_layout.text_rect);
    }

    (!drawn_rects.is_empty()).then_some(drawn_rects)
}

pub(crate) fn ui_tick_label(
    ui: &mut egui::Ui,
    screen_x: f32,
    text: TickLabel,
    existing_rects: Option<&[egui::Rect]>,
    draw_rect: bool,
) -> Option<egui::Rect> {
    let layout = layout_tick_label(ui, screen_x, text);
    if let Some(rects) = existing_rects {
        for rect in rects {
            if rect.intersects(layout.text_rect) {
                return None;
            }
        }
    }
    paint_tick_label(ui, &layout, draw_rect);
    Some(layout.text_rect)
}

pub(crate) fn ui_triangle(ui: &mut egui::Ui, screen_x: f32, triangle_type: TriangleType) {
    let height = TICK_HEIGHT_LONG;
    let side = 10.0;
    let screen_y_top = ui.min_rect().bottom() - height - 2.0;
    let screen_y_bottom = screen_y_top + side;
    let mut points = Vec::<egui::Pos2>::new();

    match triangle_type {
        TriangleType::Left => {
            let left_top = rp(ui, [screen_x - side / 2.0, screen_y_top].into());
            let right_top = rp(ui, [screen_x, screen_y_top].into());
            let bottom = rp(ui, [screen_x, screen_y_bottom].into());
            points.push(left_top);
            points.push(right_top);
            points.push(bottom);
        }
        TriangleType::Right => {
            let left_top = rp(ui, [screen_x, screen_y_top].into());
            let right_top = rp(ui, [screen_x + side / 2.0, screen_y_top].into());
            let bottom = rp(ui, [screen_x, screen_y_bottom].into());
            points.push(left_top);
            points.push(right_top);
            points.push(bottom);
        }
        TriangleType::Full => {
            let left_top = rp(ui, [screen_x - side / 2.0, screen_y_top].into());
            let right_top = rp(ui, [screen_x + side / 2.0, screen_y_top].into());
            let bottom = rp(ui, [screen_x, screen_y_bottom].into());
            points.push(left_top);
            points.push(right_top);
            points.push(bottom);
        }
    }

    let color = egui::Color32::LIGHT_BLUE;
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::new(0.0, color),
    ));
}

pub(crate) fn ui_tick_line(
    ui: &mut egui::Ui,
    screen_x: f32,
    height: f32,
    color: Option<egui::Color32>,
) {
    let rect_bottom = ui.min_rect().bottom();
    let pos_top = rpc(ui, [screen_x, rect_bottom - height].into());
    let pos_bottom = rpc(ui, [screen_x, rect_bottom].into());
    let color = color.unwrap_or(ui.style().visuals.text_color());
    ui.painter()
        .line_segment([pos_top, pos_bottom], (1.0, color));
}

#[allow(dead_code)]
pub(crate) fn ui_start_end(ui: &mut egui::Ui, ix_range: sample::FracIxRange) {
    let rect = ui.min_rect();
    ui_tick_line(ui, rect.left(), TICK_HEIGHT_LONG, None);
    let text_pos = rect.left_top() + egui::vec2(5.0, -5.0);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_TOP,
        ix_range.start,
        egui::FontId::default(),
        ui.style().visuals.text_color(),
    );

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

pub(crate) fn ui_ix_lattice(
    ui: &mut egui::Ui,
    ruler: &mut model::ruler::Time,
    existing_rects: &mut Vec<egui::Rect>,
) {
    let Some(ix_lattice) = ruler.ix_lattice() else {
        return;
    };

    for tick in &ix_lattice.ticks {
        let tick_height = match tick.tick_type {
            ruler::TickType::Big => TICK_HEIGHT_LONG,
            ruler::TickType::Mid => TICK_HEIGHT_MID,
            ruler::TickType::Small => TICK_HEIGHT_SHORT,
        };
        ui_tick_line(ui, tick.screen_x, tick_height, None);

        if tick.tick_type == ruler::TickType::Big {
            let rect = ui_tick_label(
                ui,
                tick.screen_x,
                tick.sample_ix.into(),
                Some(existing_rects.as_slice()),
                false,
            );
            if let Some(rect) = rect {
                existing_rects.push(rect);
            }
        }
    }

    for tick in &ix_lattice.ticks {
        if tick.tick_type == ruler::TickType::Mid {
            let rect = ui_tick_label(
                ui,
                tick.screen_x,
                tick.sample_ix.into(),
                Some(existing_rects.as_slice()),
                false,
            );
            if let Some(rect) = rect {
                existing_rects.push(rect);
            }
        }
    }
}

fn layout_tick_label(ui: &egui::Ui, screen_x: f32, text: TickLabel) -> TickLabelLayout {
    let font_id = egui::FontId::proportional(14.0);
    let color = ui.style().visuals.text_color();
    let text = match text {
        TickLabel::SampleIx(sample_ix) => format_compact(sample_ix),
        TickLabel::Text(text) => text,
    };
    let galley = ui.fonts(|fonts| fonts.layout_no_wrap(text, font_id, color));
    let text_size = galley.size();
    let mut text_pos: egui::Pos2 =
        [screen_x - (text_size.x / 2.0), ui.min_rect().top() + 3.0].into();
    if text_pos.x + text_size.x > ui.min_rect().right() {
        text_pos.x = ui.min_rect().right() - text_size.x - 2.0;
    } else if text_pos.x < ui.min_rect().left() {
        text_pos.x = ui.min_rect().left() + 2.0;
    }
    let text_rect = egui::Rect::from_min_size(text_pos, text_size).expand(2.0);

    TickLabelLayout {
        galley,
        text_pos,
        text_rect,
        color,
    }
}

fn set_tick_label_left(ui: &egui::Ui, layout: &mut TickLabelLayout, left: f32) {
    let inner_width = layout.text_rect.width() - 4.0;
    let clamped_left = left.clamp(
        ui.min_rect().left(),
        ui.min_rect().right() - layout.text_rect.width(),
    );
    layout.text_pos.x = clamped_left + 2.0;
    layout.text_rect = egui::Rect::from_min_size(
        layout.text_pos,
        egui::vec2(inner_width, layout.text_rect.height() - 4.0),
    )
    .expand(2.0);
}

fn set_tick_label_right(ui: &egui::Ui, layout: &mut TickLabelLayout, right: f32) {
    let desired_left = right - layout.text_rect.width();
    set_tick_label_left(ui, layout, desired_left);
}

fn paint_tick_label(ui: &mut egui::Ui, layout: &TickLabelLayout, draw_rect: bool) {
    ui.painter()
        .galley(layout.text_pos, layout.galley.clone(), layout.color);
    if draw_rect {
        ui.painter()
            .rect_stroke(layout.text_rect, 3.0, egui::Stroke::new(1.0, layout.color));
    }
}

fn format_compact(n: i64) -> String {
    format_compact_exact(n, 2)
}

fn format_compact_exact(n: i64, max_decimals: usize) -> String {
    const BASE: i64 = 1000;
    const SUFFIXES: [&str; 7] = ["", "k", "M", "G", "T", "P", "E"];

    let abs_n = n.abs();

    for exp in (1..SUFFIXES.len()).rev() {
        let scale = BASE.pow(exp as u32);
        if abs_n < scale {
            continue;
        }

        let q = n / scale;
        let r = n % scale;

        if r == 0 {
            return format!("{}{}", q.separate_with_commas(), SUFFIXES[exp]);
        }

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

    n.separate_with_commas()
}
