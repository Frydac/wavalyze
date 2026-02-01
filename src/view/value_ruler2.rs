use crate::audio::sample;
use crate::model::ruler::sample_value_to_screen_y_e;
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
    let zero_stroke = Stroke::new(stroke.width + 1.0, Color32::LIGHT_BLUE);

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

            draw_value_label(ui, rect, y, format_tick_label(value, val_rng));
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

        // Label for zero tick.
        draw_value_label(ui, rect, zero_y, format_tick_label(0.0, val_rng));
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

fn draw_value_label(ui: &egui::Ui, rect: Rect, y: f32, text: String) {
    let font_id = FontId::proportional(12.0);
    let color = ui.style().visuals.text_color();
    let galley = ui.fonts(|fonts| fonts.layout_no_wrap(text, font_id, color));
    let text_size = galley.size();
    let mut text_pos = Pos2::new(rect.left() + 4.0, y - text_size.y / 2.0);
    if text_pos.y + text_size.y > rect.bottom() {
        text_pos.y = rect.bottom() - text_size.y - 2.0;
    } else if text_pos.y < rect.top() {
        text_pos.y = rect.top() + 2.0;
    }
    ui.painter().galley(text_pos, galley, color);
}
