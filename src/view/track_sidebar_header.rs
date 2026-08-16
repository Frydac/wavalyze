use crate::model::{self, config::RULER_SLOT_WIDTH};

/// Controls for the area aligned with the per-track sidebars, above the track list.
///
/// The whole rectangle belongs to this module so more shared track controls can be added without
/// coupling them to the time ruler.
pub fn ui(ui: &mut egui::Ui, model: &mut model::Model, rect: egui::Rect) {
    let mut header_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    header_ui.set_min_size(rect.size());

    // Align the skew control with the right-most per-track ruler slot and its Y zoom controls.
    let slot_width = RULER_SLOT_WIDTH.min(rect.width());
    let skew_rect = egui::Rect::from_min_size(
        egui::pos2(rect.right() - slot_width, rect.top()),
        egui::vec2(slot_width, rect.height()),
    );
    let mut skew_ui = header_ui.new_child(egui::UiBuilder::new().max_rect(skew_rect).layout(
        egui::Layout::centered_and_justified(egui::Direction::TopDown),
    ));
    skew_ui.spacing_mut().slider_width = (skew_rect.height() - 8.0).max(0.0);

    let response = skew_ui.add(
        egui::Slider::new(
            &mut model.user_config.value_display_scale.skew_factor,
            0.0..=model::ruler::ValueDisplayScale::MAX_SKEW_FACTOR,
        )
        .vertical()
        .step_by(0.01)
        .show_value(false),
    );
    response.on_hover_text(format!(
        "Value skew: {:.2}\nMagnifies quiet amplitudes without changing the audio.",
        model.user_config.value_display_scale.skew_factor
    ));
}
