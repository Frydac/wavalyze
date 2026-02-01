/// draw a rectangle with a text label
pub fn debug_rect_text(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    color: egui::Color32,
    text: impl ToString,
) {
    let galley =
        ui.fonts(|f| f.layout_no_wrap(text.to_string(), egui::FontId::monospace(12.0), color));
    // let rect = rect.expand(2.0);
    ui.painter().galley(rect.min, galley, color);
    debug_rect(ui, rect, color);
}

/// draw a rectangle
pub fn debug_rect(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.0, color);
    ui.painter()
        .rect(rect, 0.0, egui::Color32::TRANSPARENT, stroke);
    // ui.painter().line_segment([rect.min, rect.max], egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
    // ui.painter().line_segment([rect.left_top(), rect.right_top()], egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
    ui.painter().line_segment([rect.min, rect.max], stroke);
}

/// round to pixel center (TODO: move to somehwere more general)
pub fn rpc(ui: &egui::Ui, pos: egui::Pos2) -> egui::Pos2 {
    ui.painter().round_pos_to_pixel_center(pos)
}
