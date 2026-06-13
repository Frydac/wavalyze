use crate::{
    model::Model,
    view::{track, util::add_row_label},
};

/// Left-panel "Tracks" tab: lists every track in track order with a visibility checkbox and a
/// hover popup matching the central-panel track header. Track-centric counterpart to the
/// file-centric [`super::file::ui`].
pub fn ui(ui: &mut egui::Ui, model: &mut Model) {
    ui.add_space(5.0);
    ui.heading("Tracks");
    ui.add_space(5.0);

    if model.tracks.tracks_order.is_empty() {
        ui.label("No tracks");
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Clone the small id list so we can mutate the model (toggle visibility) inside the loop.
        let track_ids = model.tracks.tracks_order.clone();
        for track_id in track_ids {
            let Some(visible) = model.tracks.get_track(track_id).map(|track| track.visible) else {
                continue;
            };
            let label = track::track_label(model, track_id);
            let hover_info = track::header_hover_info(model, track_id);

            ui.horizontal(|ui| {
                let mut checked = visible;
                let response = ui.add(egui::Checkbox::without_text(&mut checked));
                if response.changed() {
                    model.tracks.set_track_visibility(track_id, checked);
                }
                let label_response = add_row_label(ui, label);
                if let Some(hover_info) = hover_info {
                    label_response.on_hover_ui(move |ui| hover_info.show(ui, track_id));
                }
            });
            ui.add_space(2.0);
        }
    });
}
