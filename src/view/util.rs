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
    use super::zoom_delta_to_scroll_delta;

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
