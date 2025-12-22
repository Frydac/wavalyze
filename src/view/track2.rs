use crate::model::{self, track2::TrackId, Model};
use anyhow::Result;

pub struct Track {
    id: TrackId,
}

impl Track {
    pub fn new(id: TrackId) -> Self {
        Self { id }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, model: &mut Model) -> Result<()> {
        // ui.label(format!("Track: {:?}", self.id));
        let track = model
            .tracks2
            .tracks
            .get_mut(self.id)
            .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", self.id))?;

        let screen_rect = ui.min_rect();
        track.set_screen_rect(screen_rect.into());

        self.ui_track_header(ui, track)?;

        Ok(())
    }

    pub fn ui_track_header(&mut self, ui: &mut egui::Ui, track: &mut model::track2::Track) -> Result<()> {
        Ok(())
    }
}
