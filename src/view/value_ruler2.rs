use crate::model::ruler::sample_value_to_screen_y_e;
use crate::model::track2::Track;
use crate::model::{Action, track2::TrackId};
use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke};

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

    // Simple lattice: three short guide ticks.
    for frac in [0.25_f32, 0.5_f32, 0.75_f32] {
        let y = rect.top() + rect.height() * frac;
        let line = [
            Pos2::new(rect.right() - MINOR_TICK_LEN, y),
            Pos2::new(rect.right(), y),
        ];
        painter.line_segment(line, minor_stroke);
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
        let label_pos = Pos2::new(rect.left() + 4.0, zero_y);
        painter.text(
            label_pos,
            Align2::LEFT_CENTER,
            "0",
            FontId::proportional(12.0),
            ui.style().visuals.text_color(),
        );
    }

    // Dragging the value ruler pans the value range of this track.
    let response = ui.interact(
        rect,
        ui.id().with(("value_ruler_drag", track_id)),
        egui::Sense::drag(),
    );
    if response.dragged() {
        let delta = ui.input(|i| i.pointer.delta());
        actions.push(Action::ShiftY {
            track_id,
            nr_pixels: delta.y,
        });
    }
}
