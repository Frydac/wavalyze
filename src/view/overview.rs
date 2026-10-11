//! Shared movement and viewport styling for the time and value overviews.

#[derive(Debug, Clone, Copy)]
pub(super) struct DragMotion {
    units_per_pixel: f64,
    fine_units_per_pixel: f64,
    previous_position: f32,
    total: f64,
}

impl DragMotion {
    /// Capture both sensitivities at the start. Signed units support either axis
    /// direction; precision is relative to the original visible span.
    pub(super) fn new(units_per_pixel: f64, span: f64, extent: f32, origin: f32) -> Self {
        Self {
            units_per_pixel,
            fine_units_per_pixel: units_per_pixel.signum()
                * units_per_pixel.abs().min(span / extent as f64)
                * 0.1,
            previous_position: origin,
            total: 0.0,
        }
    }

    pub(super) fn advance(&mut self, position: f32, fine: bool) -> f64 {
        let unit = if fine {
            self.fine_units_per_pixel
        } else {
            self.units_per_pixel
        };
        let delta = (position - self.previous_position) as f64 * unit;
        self.previous_position = position;
        self.total += delta;
        delta
    }

    pub(super) fn total(self) -> f64 {
        self.total
    }
}

pub(super) fn viewport_stroke(accent: egui::Color32) -> egui::Stroke {
    egui::Stroke::new(1.0, accent.gamma_multiply(0.55))
}

pub(super) fn draw_viewport(
    painter: &egui::Painter,
    rect: egui::Rect,
    rounding: u8,
    accent: egui::Color32,
) {
    if rect.is_positive() {
        painter.rect(
            rect,
            rounding,
            accent.gamma_multiply(0.12),
            viewport_stroke(accent),
            egui::epaint::StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precision_changes_do_not_jump_and_use_the_original_span() {
        for sign in [-1.0, 1.0] {
            let mut drag = DragMotion::new(sign * 0.02, 0.5, 100.0, 20.0);
            assert!((drag.advance(30.0, false) - sign * 0.2).abs() < 1e-12);
            assert_eq!(drag.advance(30.0, true), 0.0);
            assert!((drag.advance(40.0, true) - sign * 0.005).abs() < 1e-12);
            assert_eq!(drag.advance(40.0, false), 0.0);
            assert!((drag.advance(30.0, false) + sign * 0.2).abs() < 1e-12);
            assert!((drag.total() - sign * 0.005).abs() < 1e-12);
        }
    }
}
