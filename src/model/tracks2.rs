use crate::{
    audio::{self, BufferId},
    model::{config::TrackConfig, hover_info::HoverInfoE, ruler},
};
use anyhow::Result;
use slotmap::SlotMap;

use crate::{
    model::track2::{Track, TrackId},
    pos,
    wav::file2::File,
};

#[derive(Default, Debug, Clone)]
pub struct Tracks {
    pub ruler: ruler::Time,
    pub tracks: SlotMap<TrackId, Track>,
    pub tracks_order: Vec<TrackId>,
    // hover
    pub hover_info: HoverInfoE,
    // selection
    // zoom
    pub available_height: f32,
    pub width_info: f32,
}

// no screen rect:
// * open app without files
//   -> ruler gets screen rect but no IxZoomOffset
// * load files
//   -> just add track without screen/sample rect
// * next frame
//   * check time_line isn't set
//     * check if we have tracks
//       -> if yes, get max sample range from all tracks
//          initialize time_line, we now have zoom and offset
//          -> now we can set all the sample rects for each track
//          -> we also have the screen rect for each track
//          -> we can now update the view_buffer
//

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct TracksHoverInfo {
    pub track_id: TrackId,
    pub screen_pos: pos::Pos,
}

impl Tracks {
    pub fn add_track_to_end(
        &mut self,
        buffer_id: BufferId,
        track_config: &TrackConfig,
    ) -> Result<TrackId> {
        let track = Track::new2(buffer_id, track_config)?;
        let track_id = self.tracks.insert(track);
        self.tracks_order.push(track_id);
        Ok(track_id)
    }

    pub fn remove_track(&mut self, track_id: TrackId) {
        self.tracks.remove(track_id);
        self.tracks_order.retain(|id| *id != track_id);
    }

    pub fn add_tracks_from_file(&mut self, file: &File, track_config: &TrackConfig) -> Result<()> {
        for (ch_ix, channel) in file.channels.iter() {
            let _ = self.add_track_to_end(channel.buffer_id, track_config)?;
        }
        Ok(())
    }

    pub fn remove_all_tracks(&mut self) {
        self.tracks.clear();
        self.tracks_order.clear();
    }

    pub fn find_track(&self, buffer_id: BufferId) -> Option<(TrackId, &Track)> {
        self.tracks
            .iter()
            .find(|(_, track)| track.single.item.buffer_id == buffer_id)
    }

    pub fn get_track(&self, track_id: TrackId) -> Option<&Track> {
        self.tracks.get(track_id)
    }

    pub fn get_track_mut(&mut self, track_id: TrackId) -> Option<&mut Track> {
        self.tracks.get_mut(track_id)
    }

    pub fn get_track_height(&self, track_id: TrackId) -> Option<f32> {
        self.tracks.get(track_id).map(|track| track.height)
    }

    pub fn set_track_height(&mut self, track_id: TrackId, height: f32) {
        if let Some(track) = self.tracks.get_mut(track_id) {
            track.height = height;
        } else {
            tracing::warn!("Track {:?} not found", track_id);
        }
    }

    pub fn set_tracks_height(&mut self, height: f32) {
        for track in self.tracks.values_mut() {
            track.height = height;
        }
    }

    pub fn pan_track_value_range(&mut self, track_id: TrackId, delta_pixels: f32) -> Result<()> {
        let track = self
            .tracks
            .get_mut(track_id)
            .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
        let screen_rect = track
            .screen_rect
            .ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
        let sample_rect = track
            .sample_rect
            .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
        let Some(val_rng) = sample_rect.val_rng() else {
            return Ok(());
        };

        let delta_val = ruler::value::pixels_to_value_delta(delta_pixels, val_rng, screen_rect);
        let shifted = ruler::value::pan_val_range(val_rng, delta_val);
        let mut sample_rect = sample_rect;
        sample_rect.set_val_rng(shifted);
        track.set_sample_rect(sample_rect);
        Ok(())
    }

    /// Update the sample ranges of all tracks to match the ruler zoom level
    /// Should be called after each change to the ruler zoom level/position
    /// TODO: enforce this somehow?
    pub fn update_tracks_sample_ix_ranges_to_ruler(
        &mut self,
        audio: &audio::manager::AudioManager,
    ) -> Result<()> {
        anyhow::ensure!(
            self.ruler.screen_rect().width() > 0.0,
            "Ruler screen rect width is zero"
        );

        // Get current global sample index range
        let ruler_ix_range = self
            .ruler
            .ix_range()
            .ok_or(anyhow::anyhow!("Ruler has no time line"))?;

        // Update tracks to global sample index range
        for track in self.tracks.values_mut() {
            track.set_ix_range(ruler_ix_range, audio)?;
        }

        Ok(())
    }

    fn get_sample_rect_longest_track(
        &self,
        audio: &audio::manager::AudioManager,
    ) -> Option<audio::SampleRectE> {
        // {
        // let max_sample_rect = self.tracks.values().map(|track| {
        //     let buffer_id = track.single.item.buffer_id;
        //     let buffer = audio.get_buffer(buffer_id).ok()?;
        //     let sample_rect = audio::SampleRectE::from_buffere(buffer);
        //     Some(sample_rect)
        // }).max_by_key(|sample_rect| sample_rect.width() as u64)?;
        // }
        let mut max_sample_rect: Option<audio::SampleRectE> = None;
        for track in self.tracks.values() {
            let buffer_id = track.single.item.buffer_id;
            let buffer = audio.get_buffer(buffer_id).ok()?;
            let sample_rect = audio::SampleRectE::from_buffere(buffer);
            if max_sample_rect
                .as_ref()
                .is_none_or(|max_rect| max_rect.width() < sample_rect.width())
            {
                max_sample_rect = Some(sample_rect);
            }
        }
        max_sample_rect
    }

    /// Zoom to the longest track
    pub fn zoom_to_full(&mut self, audio: &audio::manager::AudioManager) -> Result<()> {
        let max_sample_rect = self
            .get_sample_rect_longest_track(audio)
            .ok_or(anyhow::anyhow!("No tracks"))?;
        self.zoom_to_sample_rect(max_sample_rect, audio)
    }

    /// Update track heights to equally distribute the available height, taking min_height into account.
    pub fn fill_screen_height(&mut self, min_height: f32) -> Result<()> {
        if self.tracks.is_empty() {
            return Ok(());
        }
        let track_height = self.available_height / self.tracks.len() as f32;
        for track in self.tracks.values_mut() {
            track.height = track_height.max(min_height);
        }
        Ok(())
    }

    fn zoom_to_sample_rect(
        &mut self,
        sample_rect: audio::SampleRectE,
        audio: &audio::manager::AudioManager,
    ) -> Result<()> {
        anyhow::ensure!(
            self.ruler.screen_rect().width() > 0.0,
            "Ruler screen rect width is zero"
        );
        let nr_samples = sample_rect.width() as f64;
        let nr_pixels = self.ruler.screen_rect().width() as f64;
        let samples_per_pixel = nr_samples / nr_pixels;
        self.ruler.set_samples_per_pixel(samples_per_pixel);
        self.ruler.time_line.as_mut().unwrap().ix_start = sample_rect.ix_rng().start;
        self.update_tracks_sample_ix_ranges_to_ruler(audio)?;
        Ok(())
    }
}
