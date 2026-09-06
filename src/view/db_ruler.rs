use crate::audio::db;
use crate::audio::sample::{self, Sample};
use crate::model::config::ThemeColors;
use crate::model::hover_info::HoverInfoE;
use crate::model::ruler::{
    DbLattice, TickType, ValueDisplayScale, sample_value_to_screen_y, screen_y_to_sample_value,
};
use crate::model::track::Track;
use crate::model::{Action, track::TrackId};
use crate::view::util::{ruler_zero_deadzone, zoom_delta_to_scroll_delta};
use egui::{Color32, FontId, Pos2, Rect, Stroke};

pub const NR_PIXELS_PER_DB_TICK: f32 = 50.0;

pub struct DbRulerContext<'a> {
    pub actions: &'a mut Vec<Action>,
    pub hover_info: &'a HoverInfoE,
    pub audio: &'a crate::audio::manager::AudioManager,
    pub pan_y_mult: f32,
    pub zoom_y_mult: f32,
    pub zero_deadzone_height: f32,
    pub display_scale: ValueDisplayScale,
}

pub struct DbRulerConfig {
    pub show_hover_tick: bool,
}

#[derive(Clone, Copy)]
struct HoverDbStyle<'a> {
    theme_colors: &'a ThemeColors,
    display_scale: ValueDisplayScale,
}

impl Default for DbRulerConfig {
    fn default() -> Self {
        Self {
            show_hover_tick: true,
        }
    }
}

/// Draw a dB ruler for a track. The vertical mapping is shared with the amplitude ruler — both
/// use `sample_value_to_screen_y` — so the two rulers stay perfectly aligned with the waveform.
/// What differs is the tick cadence: nice dB values (0, -6, -12, ...) instead of nice amplitude
/// values (0.0, 0.5, 1.0, ...).
pub fn ui(
    ui: &mut egui::Ui,
    track: &Track,
    track_id: TrackId,
    rect: Rect,
    config: DbRulerConfig,
    theme_colors: &ThemeColors,
    ctx: &mut DbRulerContext<'_>,
) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }

    let Some(sample_rect) = track.single.sample_rect_raw() else {
        return;
    };

    let Some(val_rng) = sample_rect.val_rng() else {
        return;
    };

    let mut lattice = DbLattice::default();
    if lattice
        .compute_ticks(
            val_rng,
            rect.into(),
            NR_PIXELS_PER_DB_TICK,
            ctx.display_scale,
        )
        .is_err()
    {
        return;
    }

    let zero_y = sample_value_to_screen_y(0.0, val_rng, rect.into(), ctx.display_scale);
    let active_deadzone = zero_y
        .and_then(|zero_y| ruler_zero_deadzone(rect, zero_y, ctx.zero_deadzone_height))
        .filter(|deadzone| {
            ui.ctx().input(|i| {
                i.modifiers.ctrl
                    && i.pointer
                        .hover_pos()
                        .is_some_and(|pos| deadzone.contains(pos))
            })
        });

    let painter = ui.painter();
    if let Some(deadzone) = active_deadzone {
        painter.rect_filled(deadzone, 0.0, theme_colors.waveform_selection_fill);
    }
    let border_stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
    let tick_color = ui.style().visuals.text_color();
    let tick_stroke = Stroke::new(1.0, tick_color);
    let zero_stroke = Stroke::new(1.0, tick_color);

    painter.rect(
        rect,
        0.0,
        Color32::TRANSPARENT,
        border_stroke,
        egui::epaint::StrokeKind::Inside,
    );

    const TICK_LEN_LONG: f32 = 10.0;
    const TICK_LEN_MID: f32 = 8.0;
    const TICK_LEN_SHORT: f32 = 6.0;

    for tick in &lattice.ticks {
        let tick_len = match tick.tick_type {
            TickType::Big => TICK_LEN_LONG,
            TickType::Mid => TICK_LEN_MID,
            TickType::Small => TICK_LEN_SHORT,
        };
        let stroke = if tick.db == 0.0 {
            zero_stroke
        } else {
            tick_stroke
        };
        let line = [
            Pos2::new(rect.right() - tick_len, tick.screen_y),
            Pos2::new(rect.right(), tick.screen_y),
        ];
        painter.line_segment(line, stroke);
    }

    let response = ui.interact(
        rect,
        ui.id().with(("db_ruler_drag", track_id)),
        egui::Sense::drag(),
    );
    if response.dragged() {
        let delta = ui.input(|i| i.pointer.delta());
        ctx.actions.push(Action::PanY {
            track_id,
            nr_pixels: delta.y,
        });
    }
    handle_db_ruler_scroll(ui, rect, track_id, active_deadzone.and(zero_y), ctx);

    let mut occupied: Vec<Rect> = Vec::new();
    let hover_style = HoverDbStyle {
        theme_colors,
        display_scale: ctx.display_scale,
    };
    if config.show_hover_tick {
        draw_hover_db_from_y(
            ui,
            ctx.hover_info,
            ctx.audio,
            track,
            rect,
            &mut occupied,
            hover_style,
        );
    }
    draw_hover_db(
        ui,
        ctx.hover_info,
        ctx.audio,
        track,
        rect,
        &mut occupied,
        hover_style,
    );
    draw_lattice_labels(ui, rect, &lattice, &mut occupied);
}

fn handle_db_ruler_scroll(
    ui: &egui::Ui,
    rect: Rect,
    track_id: TrackId,
    zero_anchor_y: Option<f32>,
    ctx: &mut DbRulerContext<'_>,
) {
    let hovered = ui
        .ctx()
        .pointer_hover_pos()
        .map(|pos| rect.contains(pos))
        .unwrap_or(false);
    if !hovered {
        return;
    }
    let scroll_zoom_speed = ui.ctx().options(|o| o.input_options.scroll_zoom_speed);
    let (scroll, zoom_scroll_delta, modifiers, hover_pos) = ui.ctx().input(|i| {
        (
            i.smooth_scroll_delta,
            zoom_delta_to_scroll_delta(i.zoom_delta(), scroll_zoom_speed),
            i.modifiers,
            i.pointer.hover_pos(),
        )
    });
    let scroll_y = if scroll.y != 0.0 { scroll.y } else { scroll.x };
    if modifiers.shift && !modifiers.ctrl && scroll_y != 0.0 {
        ctx.actions.push(Action::PanY {
            track_id,
            nr_pixels: scroll_y * ctx.pan_y_mult,
        });
    } else if modifiers.ctrl && zoom_scroll_delta != 0.0 {
        ctx.actions.push(Action::ZoomY {
            track_id,
            nr_pixels: zoom_scroll_delta * ctx.zoom_y_mult,
            center_y: zero_anchor_y.unwrap_or_else(|| {
                hover_pos
                    .map(|p: egui::Pos2| p.y)
                    .unwrap_or(rect.center().y)
            }),
        });
    }
}

fn format_db_label(db: f64, step: f64) -> String {
    let decimals = if step >= 1.0 {
        0
    } else {
        // Same idea as the amplitude ruler: precision tracks the active step.
        let exp = (-step.log10()).ceil().max(0.0) as usize;
        exp.min(3)
    };
    let mut text = format!("{db:.decimals$}");
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

fn draw_db_label(ui: &egui::Ui, rect: Rect, y: f32, text: String) -> Rect {
    let (text_rect, galleys, color) = layout_db_label(ui, rect, y, &text);
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
    let text_rect = draw_db_label(ui, rect, y, text);

    let text_rect_draw = text_rect.expand(2.0);
    let line_color = ui.style().visuals.text_color();
    ui.painter().rect_stroke(
        text_rect_draw,
        3.0,
        egui::Stroke::new(1.0, line_color),
        egui::epaint::StrokeKind::Inside,
    );
    text_rect
}

fn layout_db_label(
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
        .map(|line| {
            ui.painter()
                .layout_no_wrap(line.clone(), font_id.clone(), color)
        })
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

fn draw_hover_db(
    ui: &egui::Ui,
    hover_info: &HoverInfoE,
    audio: &crate::audio::manager::AudioManager,
    track: &Track,
    rect: Rect,
    occupied: &mut Vec<Rect>,
    style: HoverDbStyle<'_>,
) {
    let HoverInfoE::IsHovered(hover_info) = hover_info else {
        return;
    };
    {
        let sample_view = match track.single.sample_view.as_ref() {
            Some(view) => view,
            None => return,
        };
        if sample_view.samples_per_pixel >= 1.0 {
            return;
        }
    }
    let sample_rect = match track.single.sample_rect() {
        Some(rect) => rect,
        None => return,
    };

    let global_sample_ix = hover_info.sample_ix.round() as i64;
    let sample_ix = global_sample_ix + track.single.sample_ix_offset as i64;
    if sample_ix < 0 {
        return;
    }
    let sample_ix = sample_ix as usize;
    let buffer_id = track.single.buffer_id;
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
            let db = db::gain_to_db(sample_value.abs());
            (
                sample_value_to_screen_y(
                    (*sample_value).to_norm(buffer.bit_depth),
                    val_rng,
                    ruler_rect,
                    style.display_scale,
                ),
                format!("{db:.2} dB\n{sample_value:.3}"),
            )
        }
        crate::audio::buffer::BufferE::I16(buffer) => {
            let Some(sample_value) = buffer.data.get(sample_ix) else {
                return;
            };
            let scaled = sample::convert::pcm162flt(*sample_value) as f32;
            let db = db::gain_to_db(scaled.abs());
            (
                sample_value_to_screen_y(
                    (*sample_value).to_norm(buffer.bit_depth),
                    val_rng,
                    ruler_rect,
                    style.display_scale,
                ),
                format!("{db:.2} dB\n{scaled:.3}"),
            )
        }
        crate::audio::buffer::BufferE::I32(buffer) => {
            let Some(sample_value) = buffer.data.get(sample_ix) else {
                return;
            };
            let scaled = (*sample_value).to_norm(buffer.bit_depth) as f32;
            let db = db::gain_to_db(scaled.abs());
            (
                sample_value_to_screen_y(
                    (*sample_value).to_norm(buffer.bit_depth),
                    val_rng,
                    ruler_rect,
                    style.display_scale,
                ),
                format!("{db:.2} dB\n{scaled:.3}"),
            )
        }
    };
    let Some(y) = y else {
        return;
    };
    if y < rect.top() || y > rect.bottom() {
        return;
    }

    let label_rect = draw_hover_label(ui, rect, y, label, style.theme_colors.accent);
    occupied.push(label_rect);
}

fn draw_hover_db_from_y(
    ui: &egui::Ui,
    hover_info: &HoverInfoE,
    audio: &crate::audio::manager::AudioManager,
    track: &Track,
    rect: Rect,
    occupied: &mut Vec<Rect>,
    style: HoverDbStyle<'_>,
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

    let sample_rect = match track.single.sample_rect() {
        Some(rect) => rect,
        None => return,
    };

    if audio.get_buffer(track.single.buffer_id).is_err() {
        return;
    }

    let ruler_rect: crate::rect::Rect = rect.into();
    let Some(val_rng) = sample_rect.val_rng() else {
        return;
    };

    let Some(sample_value) =
        screen_y_to_sample_value(hover_pos.y, val_rng, screen_rect, style.display_scale)
    else {
        return;
    };
    let Some(y_ruler) =
        sample_value_to_screen_y(sample_value, val_rng, ruler_rect, style.display_scale)
    else {
        return;
    };
    let db = db::gain_to_db(sample_value.abs() as f32);
    let label = format!("{db:.2} dB\n{sample_value:.3}");

    if y_ruler < rect.top() || y_ruler > rect.bottom() {
        return;
    }

    let label_rect = draw_hover_label(ui, rect, y_ruler, label, style.theme_colors.accent);
    occupied.push(label_rect);
}

fn draw_lattice_labels(ui: &egui::Ui, rect: Rect, lattice: &DbLattice, occupied: &mut Vec<Rect>) {
    // Coarse-to-fine sweep: Big ticks get first refusal on screen real estate, then Mid, then
    // Small fill remaining gaps. In linear mode the Big cadence usually saturates and the
    // later passes silently collide; under skew, the blown-up central region accepts them.
    let passes: [(TickType, f64); 3] = [
        (TickType::Big, lattice.label_step_db),
        (
            TickType::Mid,
            lattice.mid_step_db.unwrap_or(lattice.label_step_db),
        ),
        (TickType::Small, lattice.minor_step_db),
    ];

    for (kind, step) in passes {
        for tick in lattice.ticks.iter().filter(|t| t.tick_type == kind) {
            let text = format_db_label(tick.db, step);
            let (label_rect, _galleys, _color) = layout_db_label(ui, rect, tick.screen_y, &text);
            if occupied.iter().any(|r| r.intersects(label_rect)) {
                continue;
            }
            draw_db_label(ui, rect, tick.screen_y, text);
            occupied.push(label_rect);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_db_label;

    #[test]
    fn integer_step_drops_decimals() {
        assert_eq!(format_db_label(0.0, 6.0), "0");
        assert_eq!(format_db_label(-6.0, 6.0), "-6");
        assert_eq!(format_db_label(-24.0, 12.0), "-24");
    }

    #[test]
    fn neg_zero_label_is_normalized() {
        assert_eq!(format_db_label(-0.0, 1.0), "0");
    }
}
