use crate::audio::sample;
use crate::model::hover_info::HoverInfoE;
use crate::model::ruler::{
    sample_value_to_screen_y, sample_value_to_screen_y_e, screen_y_to_sample_value,
};
use crate::model::track2::Track;
use crate::model::{Action, track2::TrackId};
use egui::{Color32, FontId, Pos2, Rect, Stroke};

/// Draw a value ruler for a track using its sample_rect and screen_rect mapping.
/// For now we only draw a zero tick centered via the current value range.
pub fn ui(
    ui: &mut egui::Ui,
    track: &Track,
    track_id: TrackId,
    rect: Rect,
    actions: &mut Vec<Action>,
    hover_info: &HoverInfoE,
    audio: &crate::audio::manager::AudioManager,
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

    let zero_y = sample_value_to_screen_y_e(0.0, val_rng, rect.into());

    let painter = ui.painter();
    let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
    let minor_stroke = Stroke::new(stroke.width, stroke.color.linear_multiply(0.6));
    let zero_stroke = Stroke::new(stroke.width + 1.0, stroke.color);

    painter.rect(rect, 0.0, Color32::TRANSPARENT, stroke);

    const TICK_LEN: f32 = 10.0;
    const MINOR_TICK_LEN: f32 = 6.0;

    // Value ticks at -1.0, -0.5, 0.5, 1.0 (mapped to PCM ranges when needed).
    for value in [-1.0_f32, -0.5_f32, 0.5_f32, 1.0_f32] {
        let Some(y) = sample_value_to_screen_y_e(value, val_rng, rect.into()) else {
            continue;
        };
        if (rect.top()..=rect.bottom()).contains(&y) {
            let line = [
                Pos2::new(rect.right() - MINOR_TICK_LEN, y),
                Pos2::new(rect.right(), y),
            ];
            painter.line_segment(line, minor_stroke);
        }
    }

    // Zero tick line (short, like the time ruler ticks).
    if let Some(zero_y) = zero_y
        && (rect.top()..=rect.bottom()).contains(&zero_y)
    {
        let zero_line = [
            Pos2::new(rect.right() - TICK_LEN, zero_y),
            Pos2::new(rect.right(), zero_y),
        ];
        painter.line_segment(zero_line, zero_stroke);
    }

    // Dragging the value ruler pans the value range of this track.
    let response = ui.interact(
        rect,
        ui.id().with(("value_ruler_drag", track_id)),
        egui::Sense::drag(),
    );
    if response.dragged() {
        let delta = ui.input(|i| i.pointer.delta());
        actions.push(Action::PanY {
            track_id,
            nr_pixels: delta.y,
        });
    }

    let mut occupied: Vec<Rect> = Vec::new();
    draw_hover_value_from_y(ui, hover_info, track, rect, &mut occupied);
    draw_hover_value(ui, hover_info, audio, track, track_id, rect, &mut occupied);
    draw_lattice_labels(ui, rect, val_rng, &mut occupied);
}

fn format_tick_label(value: f32, val_rng: sample::ValRangeE) -> String {
    match val_rng {
        sample::ValRangeE::PCM16(_) => {
            let scaled = (value * i16::MAX as f32).round() as i16;
            format!("{scaled}")
        }
        sample::ValRangeE::PCM24(_) | sample::ValRangeE::PCM32(_) => {
            let scaled = (value * 8_388_607.0).round() as i32;
            format!("{scaled}")
        }
        sample::ValRangeE::F32(_) => format!("{value:.2}"),
    }
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
    let (y, label) = match (sample_rect, buffer) {
        (
            crate::audio::sample_rect2::SampleRectE::F32(rect),
            crate::audio::buffer2::BufferE::F32(buffer),
        ) => {
            let Some(val_rng) = rect.val_rng else {
                return;
            };
            let Some(sample_value) = buffer.data.get(sample_ix) else {
                return;
            };
            (
                sample_value_to_screen_y(*sample_value, val_rng, ruler_rect),
                format!("{sample_value:.3}"),
            )
        }
        (
            crate::audio::sample_rect2::SampleRectE::I16(rect),
            crate::audio::buffer2::BufferE::I16(buffer),
        ) => {
            let Some(val_rng) = rect.val_rng else {
                return;
            };
            let Some(sample_value) = buffer.data.get(sample_ix) else {
                return;
            };
            (
                sample_value_to_screen_y(*sample_value, val_rng, ruler_rect),
                format!("{sample_value}"),
            )
        }
        (
            crate::audio::sample_rect2::SampleRectE::I32(rect),
            crate::audio::buffer2::BufferE::I32(buffer),
        ) => {
            let Some(val_rng) = rect.val_rng else {
                return;
            };
            let Some(sample_value) = buffer.data.get(sample_ix) else {
                return;
            };
            (
                sample_value_to_screen_y(*sample_value, val_rng, ruler_rect),
                format!("{sample_value}"),
            )
        }
        _ => return,
    };
    let Some(y) = y else {
        return;
    };
    if y < rect.top() || y > rect.bottom() {
        return;
    }

    let tick_line = [
        Pos2::new(rect.right() - 10.0, y),
        Pos2::new(rect.right(), y),
    ];
    ui.painter()
        .line_segment(tick_line, Stroke::new(1.0, Color32::LIGHT_BLUE));
    let label_rect = draw_value_label(ui, rect, y, label);
    occupied.push(label_rect);
}

fn draw_hover_value_from_y(
    ui: &egui::Ui,
    hover_info: &HoverInfoE,
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

    let ruler_rect: crate::rect::Rect = rect.into();
    let hover_label = match sample_rect {
        crate::audio::sample_rect2::SampleRectE::F32(rect_t) => {
            let Some(val_rng) = rect_t.val_rng else {
                return;
            };
            let Some(sample_value) = screen_y_to_sample_value(hover_pos.y, val_rng, screen_rect)
            else {
                return;
            };
            let Some(y_ruler) = sample_value_to_screen_y(sample_value, val_rng, ruler_rect) else {
                return;
            };
            let db = crate::audio::db::gain_to_db(sample_value.abs());
            Some((y_ruler, format!("{sample_value:.3}\n{db:.0} dB")))
        }
        crate::audio::sample_rect2::SampleRectE::I16(rect_t) => {
            let Some(val_rng) = rect_t.val_rng else {
                return;
            };
            let Some(sample_value) = screen_y_to_sample_value(hover_pos.y, val_rng, screen_rect)
            else {
                return;
            };
            let Some(y_ruler) = sample_value_to_screen_y(sample_value, val_rng, ruler_rect) else {
                return;
            };
            let scaled = crate::audio::sample::convert::pcm162flt(sample_value) as f32;
            let db = crate::audio::db::gain_to_db(scaled.abs());
            Some((y_ruler, format!("{sample_value}\n{scaled:.3}\n{db:.0} dB")))
        }
        crate::audio::sample_rect2::SampleRectE::I32(rect_t) => {
            let Some(val_rng) = rect_t.val_rng else {
                return;
            };
            let Some(sample_value) = screen_y_to_sample_value(hover_pos.y, val_rng, screen_rect)
            else {
                return;
            };
            let Some(y_ruler) = sample_value_to_screen_y(sample_value, val_rng, ruler_rect) else {
                return;
            };
            let scaled = match sample_rect.val_rng() {
                Some(sample::ValRangeE::PCM24(_)) => {
                    crate::audio::sample::convert::pcm242flt(sample_value) as f32
                }
                Some(sample::ValRangeE::PCM32(_)) => {
                    crate::audio::sample::convert::pcm322flt(sample_value) as f32
                }
                _ => sample_value as f32,
            };
            let db = crate::audio::db::gain_to_db(scaled.abs());
            Some((y_ruler, format!("{sample_value}\n{scaled:.3}\n{db:.0} dB")))
        }
    };

    let Some((y_ruler, label)) = hover_label else {
        return;
    };
    if y_ruler < rect.top() || y_ruler > rect.bottom() {
        return;
    }

    let tick_line = [
        Pos2::new(rect.right() - 10.0, y_ruler),
        Pos2::new(rect.right(), y_ruler),
    ];
    let tick_color = ui.style().visuals.text_color();
    ui.painter()
        .line_segment(tick_line, Stroke::new(1.0, tick_color));
    let label_rect = draw_value_label(ui, rect, y_ruler, label);
    occupied.push(label_rect);
}

fn draw_lattice_labels(
    ui: &egui::Ui,
    rect: Rect,
    val_rng: sample::ValRangeE,
    occupied: &mut Vec<Rect>,
) {
    for value in [-1.0_f32, -0.5_f32, 0.0_f32, 0.5_f32, 1.0_f32] {
        let Some(y) = sample_value_to_screen_y_e(value, val_rng, rect.into()) else {
            continue;
        };
        if (rect.top()..=rect.bottom()).contains(&y) {
            let text = format_tick_label(value, val_rng);
            let (label_rect, _galleys, _color) = layout_value_label(ui, rect, y, &text);
            if occupied.iter().any(|r| r.intersects(label_rect)) {
                continue;
            }
            draw_value_label(ui, rect, y, text);
            occupied.push(label_rect);
        }
    }
}
