use crate::audio::sample::{self, Sample};
use crate::model::hover_info::HoverInfoE;
use crate::model::ruler::{
    TickType, ValueLattice, sample_value_to_screen_y, screen_y_to_sample_value,
};
use crate::model::track::Track;
use crate::model::{Action, track::TrackId};
use egui::{Color32, FontId, Pos2, Rect, Stroke};

pub const NR_PIXELS_PER_VALUE_TICK: f32 = 50.0;

pub struct ValueRulerContext<'a> {
    pub actions: &'a mut Vec<Action>,
    pub hover_info: &'a HoverInfoE,
    pub audio: &'a crate::audio::manager::AudioManager,
    pub zoom_y_factor: f32,
}

pub struct ValueRulerConfig {
    pub show_hover_tick: bool,
}

impl Default for ValueRulerConfig {
    fn default() -> Self {
        Self {
            show_hover_tick: true,
        }
    }
}

/// Draw a value ruler for a track using its sample_rect and screen_rect mapping.
/// For now we only draw a zero tick centered via the current value range.
pub fn ui(
    ui: &mut egui::Ui,
    track: &Track,
    track_id: TrackId,
    rect: Rect,
    config: ValueRulerConfig,
    ctx: &mut ValueRulerContext<'_>,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }

    let Some(sample_rect) = track.sample_rect else {
        return;
    };

    let Some(val_rng) = sample_rect.val_rng() else {
        return;
    };

    let mut lattice = ValueLattice::default();
    if lattice
        .compute_ticks(val_rng, rect.into(), NR_PIXELS_PER_VALUE_TICK)
        .is_err()
    {
        return;
    }

    let painter = ui.painter();
    let border_stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
    // Match the time ruler tick color exactly so both rulers read as the same UI element.
    let tick_color = ui.style().visuals.text_color();
    let tick_stroke = Stroke::new(1.0, tick_color);
    let zero_stroke = Stroke::new(1.0, tick_color);

    painter.rect(rect, 0.0, Color32::TRANSPARENT, border_stroke);

    const TICK_LEN_LONG: f32 = 10.0;
    const TICK_LEN_MID: f32 = 8.0;
    const TICK_LEN_SHORT: f32 = 6.0;

    for tick in &lattice.ticks {
        let tick_len = match tick.tick_type {
            TickType::Big => TICK_LEN_LONG,
            TickType::Mid => TICK_LEN_MID,
            TickType::Small => TICK_LEN_SHORT,
        };
        let tick_stroke = if tick.sample_value == 0.0 {
            zero_stroke
        } else {
            tick_stroke
        };
        let line = [
            Pos2::new(rect.right() - tick_len, tick.screen_y),
            Pos2::new(rect.right(), tick.screen_y),
        ];
        painter.line_segment(line, tick_stroke);
    }

    // Dragging the value ruler pans the value range of this track.
    let response = ui.interact(
        rect,
        ui.id().with(("value_ruler_drag", track_id)),
        egui::Sense::drag(),
    );
    if response.dragged() {
        let delta = ui.input(|i| i.pointer.delta());
        ctx.actions.push(Action::PanY {
            track_id,
            nr_pixels: delta.y,
        });
    }
    handle_value_ruler_scroll(ui, rect, track_id, ctx);

    let mut occupied: Vec<Rect> = Vec::new();
    if config.show_hover_tick {
        draw_hover_value_from_y(ui, ctx.hover_info, ctx.audio, track, rect, &mut occupied);
    }
    draw_hover_value(
        ui,
        ctx.hover_info,
        ctx.audio,
        track,
        track_id,
        rect,
        &mut occupied,
    );
    draw_lattice_labels(ui, rect, &lattice, &mut occupied);
}

fn handle_value_ruler_scroll(
    ui: &egui::Ui,
    rect: Rect,
    track_id: TrackId,
    ctx: &mut ValueRulerContext<'_>,
) {
    let hovered = ui
        .ctx()
        .pointer_hover_pos()
        .map(|pos| rect.contains(pos))
        .unwrap_or(false);
    if !hovered {
        return;
    }
    let (scroll, modifiers, hover_pos) = ui
        .ctx()
        .input(|i| (i.raw_scroll_delta, i.modifiers, i.pointer.hover_pos()));
    // Some systems emit horizontal scroll for shift-wheel; use whichever axis is non-zero.
    let scroll_y = if scroll.y != 0.0 { scroll.y } else { scroll.x };
    if modifiers.shift && !modifiers.ctrl && scroll_y != 0.0 {
        ctx.actions.push(Action::PanY {
            track_id,
            nr_pixels: scroll_y,
        });
    } else if modifiers.ctrl && scroll_y != 0.0 {
        // Zoom around the mouse Y position for intuitive value scaling.
        ctx.actions.push(Action::ZoomY {
            track_id,
            nr_pixels: scroll_y * ctx.zoom_y_factor,
            center_y: hover_pos.map(|p| p.y).unwrap_or(rect.center().y),
        });
    }
}

fn format_tick_label(value: f64, step: f64) -> String {
    // Precision follows the active lattice step, not the absolute value. Without that, zooming
    // around 0.5 would stay stuck at one decimal place even when nearby ticks differ by 0.01.
    let decimals = decimals_for_step(step);
    let mut text = format!("{value:.decimals$}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text == "-0" {
        String::from("0")
    } else {
        text
    }
}

fn decimals_for_step(step: f64) -> usize {
    if step <= 0.0 {
        return 0;
    }

    let normalized_step = normalize_step(step);
    if normalized_step >= 1.0 {
        return 0;
    }

    // `0.5 -> 1`, `0.05 -> 2`, `0.005 -> 3`, etc.
    let decimals = (-normalized_step.log10()).ceil().max(0.0) as usize;
    decimals.min(6)
}

fn normalize_step(step: f64) -> f64 {
    if step.abs() < 1e-9 { 0.0 } else { step.abs() }
}

fn draw_value_label(ui: &egui::Ui, rect: Rect, y: f32, text: String) -> Rect {
    let (text_rect, galleys, color) = layout_value_label(ui, rect, y, &text);
    let mut cur_y = text_rect.top();
    for galley in galleys {
        ui.painter()
            .galley(Pos2::new(text_rect.left(), cur_y), galley.clone(), color);
        cur_y += galley.size().y;
    }
    text_rect
}

fn draw_hover_label(ui: &egui::Ui, rect: Rect, y: f32, text: String, tick_color: Color32) -> Rect {
    let tick_line = [
        Pos2::new(rect.right() - 10.0, y),
        Pos2::new(rect.right(), y),
    ];
    ui.painter()
        .line_segment(tick_line, Stroke::new(1.0, tick_color));
    draw_value_label(ui, rect, y, text)
}

fn layout_value_label(
    ui: &egui::Ui,
    rect: Rect,
    y: f32,
    text: &str,
) -> (Rect, Vec<std::sync::Arc<egui::Galley>>, Color32) {
    let font_id = FontId::proportional(12.0);
    let color = ui.style().visuals.text_color();
    let lines: Vec<String> = text.lines().map(|line| line.to_string()).collect();
    let galleys: Vec<_> = lines
        .iter()
        .map(|line| ui.fonts(|fonts| fonts.layout_no_wrap(line.clone(), font_id.clone(), color)))
        .collect();
    let total_height: f32 = galleys.iter().map(|g| g.size().y).sum();
    let max_width: f32 = galleys
        .iter()
        .map(|g| g.size().x)
        .fold(0.0, |a, b| a.max(b));
    let mut text_pos = Pos2::new(rect.left() + 4.0, y - total_height / 2.0);
    if text_pos.y + total_height > rect.bottom() {
        text_pos.y = rect.bottom() - total_height - 2.0;
    } else if text_pos.y < rect.top() {
        text_pos.y = rect.top() + 2.0;
    }
    let text_rect = Rect::from_min_size(text_pos, egui::vec2(max_width, total_height));
    (text_rect, galleys, color)
}

fn draw_hover_value(
    ui: &egui::Ui,
    hover_info: &HoverInfoE,
    audio: &crate::audio::manager::AudioManager,
    track: &Track,
    track_id: TrackId,
    rect: Rect,
    occupied: &mut Vec<Rect>,
) {
    let HoverInfoE::IsHovered(hover_info) = hover_info else {
        return;
    };

    let screen_rect = match track.screen_rect {
        Some(rect) => rect,
        None => {
            return;
        }
    };
    let sample_rect = match track.single.item.sample_rect() {
        Some(rect) => rect,
        None => {
            return;
        }
    };

    let sample_view = match track.single.item.sample_view.as_ref() {
        Some(view) => view,
        None => {
            return;
        }
    };
    if sample_view.samples_per_pixel >= 0.5 {
        return;
    }

    let sample_ix = hover_info.sample_ix.round() as i64;
    if sample_ix < 0 {
        return;
    }
    let sample_ix = sample_ix as usize;
    let buffer_id = track.single.item.buffer_id;
    let Ok(buffer) = audio.get_buffer(buffer_id) else {
        return;
    };
    let ruler_rect: crate::rect::Rect = rect.into();
    let Some(val_rng) = sample_rect.val_rng() else {
        return;
    };
    let (y, label) = match buffer {
        crate::audio::buffer::BufferE::F32(buffer) => {
            let Some(sample_value) = buffer.data.get(sample_ix) else {
                return;
            };
            let db = crate::audio::db::gain_to_db(sample_value.abs());
            (
                sample_value_to_screen_y(
                    (*sample_value).to_norm(buffer.bit_depth),
                    val_rng,
                    ruler_rect,
                ),
                format!("{sample_value:.3}\n{db:.3} dB"),
            )
        }
        crate::audio::buffer::BufferE::I16(buffer) => {
            let Some(sample_value) = buffer.data.get(sample_ix) else {
                return;
            };
            let scaled = crate::audio::sample::convert::pcm162flt(*sample_value) as f32;
            let db = crate::audio::db::gain_to_db(scaled.abs());
            (
                sample_value_to_screen_y(
                    (*sample_value).to_norm(buffer.bit_depth),
                    val_rng,
                    ruler_rect,
                ),
                format!("{sample_value}\n{scaled:.3}\n{db:.3} dB"),
            )
        }
        crate::audio::buffer::BufferE::I32(buffer) => {
            let Some(sample_value) = buffer.data.get(sample_ix) else {
                return;
            };
            let scaled = (*sample_value).to_norm(buffer.bit_depth) as f32;
            let db = crate::audio::db::gain_to_db(scaled.abs());
            (
                sample_value_to_screen_y(
                    (*sample_value).to_norm(buffer.bit_depth),
                    val_rng,
                    ruler_rect,
                ),
                format!("{sample_value}\n{scaled:.3}\n{db:.3} dB"),
            )
        }
    };
    let Some(y) = y else {
        return;
    };
    if y < rect.top() || y > rect.bottom() {
        return;
    }

    let label_rect = draw_hover_label(ui, rect, y, label, Color32::LIGHT_BLUE);
    occupied.push(label_rect);
}

fn draw_hover_value_from_y(
    ui: &egui::Ui,
    hover_info: &HoverInfoE,
    audio: &crate::audio::manager::AudioManager,
    track: &Track,
    rect: Rect,
    occupied: &mut Vec<Rect>,
) {
    let HoverInfoE::IsHovered(hover_info) = hover_info else {
        return;
    };

    let screen_rect = match track.screen_rect {
        Some(rect) => rect,
        None => return,
    };
    let hover_pos = hover_info.screen_pos;
    if !screen_rect.contains(hover_pos) {
        return;
    }

    let sample_rect = match track.single.item.sample_rect() {
        Some(rect) => rect,
        None => return,
    };

    let Ok(buffer) = audio.get_buffer(track.single.item.buffer_id) else {
        return;
    };

    let ruler_rect: crate::rect::Rect = rect.into();
    let Some(val_rng) = sample_rect.val_rng() else {
        return;
    };
    let hover_label = match buffer {
        crate::audio::buffer::BufferE::F32(_) => {
            let Some(sample_value) = screen_y_to_sample_value(hover_pos.y, val_rng, screen_rect)
            else {
                return;
            };
            let Some(y_ruler) = sample_value_to_screen_y(sample_value, val_rng, ruler_rect) else {
                return;
            };
            let db = crate::audio::db::gain_to_db(sample_value.abs() as f32);
            Some((y_ruler, format!("{sample_value:.3}\n{db:.3} dB")))
        }
        crate::audio::buffer::BufferE::I16(_) => {
            let Some(sample_value) = screen_y_to_sample_value(hover_pos.y, val_rng, screen_rect)
            else {
                return;
            };
            let Some(y_ruler) = sample_value_to_screen_y(sample_value, val_rng, ruler_rect) else {
                return;
            };
            let raw = (sample_value * sample::convert::float2pcm_factor(16) as f64).round() as i16;
            let db = crate::audio::db::gain_to_db(sample_value.abs() as f32);
            Some((y_ruler, format!("{raw}\n{sample_value:.3}\n{db:.3} dB")))
        }
        crate::audio::buffer::BufferE::I32(buffer_t) => {
            let Some(sample_value) = screen_y_to_sample_value(hover_pos.y, val_rng, screen_rect)
            else {
                return;
            };
            let Some(y_ruler) = sample_value_to_screen_y(sample_value, val_rng, ruler_rect) else {
                return;
            };
            let bit_depth = buffer_t.bit_depth.clamp(1, 32) as u32;
            let raw =
                (sample_value * sample::convert::float2pcm_factor(bit_depth) as f64).round() as i32;
            let db = crate::audio::db::gain_to_db(sample_value.abs() as f32);
            Some((y_ruler, format!("{raw}\n{sample_value:.3}\n{db:.3} dB")))
        }
    };

    let Some((y_ruler, label)) = hover_label else {
        return;
    };
    if y_ruler < rect.top() || y_ruler > rect.bottom() {
        return;
    }

    let tick_color = ui.style().visuals.text_color();
    let label_rect = draw_hover_label(ui, rect, y_ruler, label, tick_color);
    occupied.push(label_rect);
}

fn draw_lattice_labels(
    ui: &egui::Ui,
    rect: Rect,
    lattice: &ValueLattice,
    occupied: &mut Vec<Rect>,
) {
    for tick_type in [TickType::Big, TickType::Mid] {
        // Mid labels intentionally use half the major step, so when the lattice shows e.g.
        // `0.50, 0.55, 0.60`, the mid label can carry the extra precision it needs.
        let step = match tick_type {
            TickType::Big => lattice.major_step,
            TickType::Mid => lattice.major_step / 2.0,
            TickType::Small => continue,
        };
        for tick in lattice
            .ticks
            .iter()
            .filter(|tick| tick.tick_type == tick_type)
        {
            let text = format_tick_label(tick.sample_value, step);
            let (label_rect, _galleys, _color) = layout_value_label(ui, rect, tick.screen_y, &text);
            if occupied.iter().any(|r| r.intersects(label_rect)) {
                continue;
            }
            draw_value_label(ui, rect, tick.screen_y, text);
            occupied.push(label_rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{decimals_for_step, format_tick_label};

    #[test]
    fn step_based_precision_allows_more_digits_away_from_zero() {
        assert_eq!(format_tick_label(0.5, 0.1), "0.5");
        assert_eq!(format_tick_label(0.5, 0.01), "0.5");
        assert_eq!(format_tick_label(0.51, 0.01), "0.51");
        assert_eq!(format_tick_label(0.55, 0.05), "0.55");
    }

    #[test]
    fn step_precision_scales_with_lattice_spacing() {
        assert_eq!(decimals_for_step(1.0), 0);
        assert_eq!(decimals_for_step(0.5), 1);
        assert_eq!(decimals_for_step(0.05), 2);
        assert_eq!(decimals_for_step(0.005), 3);
    }

    #[test]
    fn negative_zero_label_is_normalized() {
        assert_eq!(format_tick_label(-0.0, 0.01), "0");
    }
}
