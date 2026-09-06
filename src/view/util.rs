/// draw a rectangle with a text label
pub fn debug_rect_text(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    color: egui::Color32,
    text: impl ToString,
) {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), egui::FontId::monospace(12.0), color);
    // let rect = rect.expand(2.0);
    ui.painter().galley(rect.min, galley, color);
    debug_rect(ui, rect, color);
}

/// draw a rectangle
pub fn debug_rect(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.0, color);
    ui.painter().rect(
        rect,
        0.0,
        egui::Color32::TRANSPARENT,
        stroke,
        egui::epaint::StrokeKind::Inside,
    );
    // ui.painter().line_segment([rect.min, rect.max], egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
    // ui.painter().line_segment([rect.left_top(), rect.right_top()], egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
    ui.painter().line_segment([rect.min, rect.max], stroke);
}

/// round to pixel center (TODO: move to somehwere more general)
/// Useful for pixel-perfect rendering of lines that are one pixel wide (or any odd number of pixels).
pub fn rpc(ui: &egui::Ui, pos: egui::Pos2) -> egui::Pos2 {
    ui.painter().round_pos_to_pixel_center(pos)
}

/// Useful for pixel-perfect rendering of filled shapes.
pub fn rp(ui: &egui::Ui, pos: egui::Pos2) -> egui::Pos2 {
    ui.painter().round_pos_to_pixels(pos)
}

/// Add a left-aligned, truncated label that fills the remaining horizontal space. Used for
/// the left-panel list rows (Files / Tracks) so long names truncate instead of wrapping.
pub fn add_row_label(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) -> egui::Response {
    let size = egui::vec2(ui.available_width().max(0.0), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Min), |ui| {
        ui.add(egui::Label::new(text).truncate())
    })
    .inner
}

/// Build the visible part of a ruler's zero-centered zoom deadzone.
pub fn ruler_zero_deadzone(rect: egui::Rect, zero_y: f32, height: f32) -> Option<egui::Rect> {
    if height <= 0.0 || !height.is_finite() || zero_y < rect.top() || zero_y > rect.bottom() {
        return None;
    }

    let deadzone = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, zero_y),
        egui::vec2(rect.width(), height),
    )
    .intersect(rect);
    deadzone.is_positive().then_some(deadzone)
}

/// Convert egui's multiplicative zoom factor back into the wheel-like delta used by the app.
pub fn zoom_delta_to_scroll_delta(zoom_delta: f32, scroll_zoom_speed: f32) -> f32 {
    if zoom_delta == 1.0 || scroll_zoom_speed == 0.0 {
        0.0
    } else {
        zoom_delta.ln() / scroll_zoom_speed
    }
}

#[cfg(test)]
mod tests {
    use super::{ruler_zero_deadzone, zoom_delta_to_scroll_delta};

    fn ruler_rect() -> egui::Rect {
        egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(110.0, 120.0))
    }

    #[test]
    fn ruler_zero_deadzone_uses_configured_height_and_full_width() {
        let deadzone = ruler_zero_deadzone(ruler_rect(), 70.0, 16.0).unwrap();

        assert_eq!(deadzone.left(), 10.0);
        assert_eq!(deadzone.right(), 110.0);
        assert_eq!(deadzone.top(), 62.0);
        assert_eq!(deadzone.bottom(), 78.0);
    }

    #[test]
    fn ruler_zero_deadzone_clips_to_ruler() {
        let deadzone = ruler_zero_deadzone(ruler_rect(), 22.0, 16.0).unwrap();

        assert_eq!(deadzone.top(), 20.0);
        assert_eq!(deadzone.bottom(), 30.0);
    }

    #[test]
    fn zero_height_disables_ruler_zero_deadzone() {
        assert_eq!(ruler_zero_deadzone(ruler_rect(), 70.0, 0.0), None);
    }

    #[test]
    fn ruler_zero_deadzone_hit_detection_uses_visible_rectangle() {
        let deadzone = ruler_zero_deadzone(ruler_rect(), 70.0, 16.0).unwrap();

        assert!(deadzone.contains(egui::pos2(60.0, 70.0)));
        assert!(!deadzone.contains(egui::pos2(60.0, 79.0)));
        assert!(!deadzone.contains(egui::pos2(9.0, 70.0)));
    }

    #[test]
    fn zoom_delta_round_trips_to_scroll_delta() {
        let scroll_delta: f32 = 24.0;
        let scroll_zoom_speed: f32 = 1.0 / 200.0;
        let zoom_delta = (scroll_delta * scroll_zoom_speed).exp();

        assert!(
            (zoom_delta_to_scroll_delta(zoom_delta, scroll_zoom_speed) - scroll_delta).abs() < 1e-4
        );
    }
}
