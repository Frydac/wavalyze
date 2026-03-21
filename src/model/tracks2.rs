use crate::{
    audio::{self, BufferId},
    model::{
        action::SelectionEdge, config::TrackConfig, hover_info::HoverInfoE, ruler,
        selection_info::SelectionInfoE,
    },
};
use anyhow::Result;
use slotmap::SlotMap;

use crate::{
    model::track::{Track, TrackId},
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
    pub selection_info: SelectionInfoE,
    // zoom
    pub available_height: f32,
    pub width_info: f32,
}

impl Tracks {
    const SELECTION_EDGE_ZOOM_SAMPLES_PER_PIXEL: f64 = 0.1;

    pub fn visible_tracks_len(&self) -> usize {
        self.tracks.values().filter(|track| track.visible).count()
    }

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

    pub fn set_track_visibility(&mut self, track_id: TrackId, visible: bool) {
        if let Some(track) = self.tracks.get_mut(track_id) {
            track.visible = visible;
        } else {
            tracing::warn!("Track {:?} not found", track_id);
        }
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

    /// Reset the value range to full scale (pan/zoom reset) for a single track.
    pub fn recenter_track_value_range(&mut self, track_id: TrackId) -> Result<()> {
        let track = self
            .tracks
            .get_mut(track_id)
            .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
        let mut sample_rect = track
            .sample_rect
            .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
        let Some(val_rng) = sample_rect.val_rng() else {
            return Ok(());
        };

        sample_rect.set_val_rng(audio::sample::ValRange {
            min: -1.0,
            max: 1.0,
        });
        track.set_sample_rect(sample_rect);
        Ok(())
    }

    /// Reset the value range to full scale (pan/zoom reset) for all tracks.
    pub fn recenter_all_value_ranges(&mut self) -> Result<()> {
        for track_id in self.tracks_order.clone() {
            if self.tracks.get(track_id).is_some_and(|track| track.visible) {
                self.recenter_track_value_range(track_id)?;
            }
        }
        Ok(())
    }

    pub fn zoom_track_value_range(
        &mut self,
        track_id: TrackId,
        delta_pixels: f32,
        center_y: f32,
    ) -> Result<()> {
        if delta_pixels == 0.0 {
            return Ok(());
        }
        let track = self
            .tracks
            .get_mut(track_id)
            .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
        let screen_rect = track
            .screen_rect
            .ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
        let mut sample_rect = track
            .sample_rect
            .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
        let Some(val_rng) = sample_rect.val_rng() else {
            return Ok(());
        };
        if !screen_rect.contains_y(center_y) {
            return Ok(());
        }
        // Invert Y so higher sample values map toward the top of the screen.
        let center_frac = (screen_rect.max.y - center_y) / screen_rect.height();
        let delta_val = ruler::value::pixels_to_value_delta(delta_pixels, val_rng, screen_rect);
        let range_len = ruler::value::val_range_len(val_rng);
        if delta_val < 0.0 && delta_val.abs() >= range_len {
            return Ok(());
        }
        let zoomed = ruler::value::zoom_val_range(val_rng, delta_val, center_frac as f64);
        if zoomed.is_empty() {
            return Ok(());
        }
        sample_rect.set_val_rng(zoomed);
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
    ) -> Option<audio::SampleRect> {
        // {
        // let max_sample_rect = self.tracks.values().map(|track| {
        //     let buffer_id = track.single.item.buffer_id;
        //     let buffer = audio.get_buffer(buffer_id).ok()?;
        //     let sample_rect = audio::SampleRect::from_buffere(buffer);
        //     Some(sample_rect)
        // }).max_by_key(|sample_rect| sample_rect.width() as u64)?;
        // }
        let mut max_sample_rect: Option<audio::SampleRect> = None;
        for track in self.tracks.values() {
            if !track.visible {
                continue;
            }
            let buffer_id = track.single.item.buffer_id;
            let buffer = audio.get_buffer(buffer_id).ok()?;
            let sample_rect = audio::SampleRect::from_buffere(buffer);
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

    pub fn zoom_to_selection(&mut self, audio: &audio::manager::AudioManager) -> Result<()> {
        let SelectionInfoE::IsSelected(selection_info) = self.selection_info else {
            return Ok(());
        };
        if selection_info.ix_rng.end <= selection_info.ix_rng.start {
            return Ok(());
        }
        anyhow::ensure!(
            self.ruler.screen_rect().width() > 0.0,
            "Ruler screen rect width is zero"
        );
        self.ruler
            .zoom_to_ix_range_clamped(audio::sample::FracIxRange {
                start: selection_info.ix_rng.start as f64,
                end: selection_info.ix_rng.end as f64,
            });
        self.update_tracks_sample_ix_ranges_to_ruler(audio)?;
        Ok(())
    }

    pub fn zoom_to_selection_edge(
        &mut self,
        audio: &audio::manager::AudioManager,
        edge: SelectionEdge,
    ) -> Result<()> {
        let SelectionInfoE::IsSelected(selection_info) = self.selection_info else {
            return Ok(());
        };
        if selection_info.ix_rng.end <= selection_info.ix_rng.start {
            return Ok(());
        }
        anyhow::ensure!(
            self.ruler.screen_rect().width() > 0.0,
            "Ruler screen rect width is zero"
        );

        let edge_ix = match edge {
            SelectionEdge::Left => selection_info.ix_rng.start as f64,
            SelectionEdge::Right => selection_info.ix_rng.end as f64,
        };
        let visible_len =
            self.ruler.screen_rect().width() as f64 * Self::SELECTION_EDGE_ZOOM_SAMPLES_PER_PIXEL;
        let half_visible_len = visible_len / 2.0;
        self.ruler.zoom_to_ix_range(audio::sample::FracIxRange {
            start: edge_ix - half_visible_len,
            end: edge_ix + half_visible_len,
        });
        self.update_tracks_sample_ix_ranges_to_ruler(audio)?;
        Ok(())
    }

    /// Update track heights to equally distribute the available height, taking min_height into account.
    pub fn fill_screen_height(&mut self, min_height: f32) -> Result<()> {
        let visible_tracks = self.visible_tracks_len();
        if visible_tracks == 0 {
            return Ok(());
        }
        let track_height = self.available_height / visible_tracks as f32;
        for track in self.tracks.values_mut() {
            if track.visible {
                track.height = track_height.max(min_height);
            }
        }
        Ok(())
    }

    fn zoom_to_sample_rect(
        &mut self,
        sample_rect: audio::SampleRect,
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

    pub fn samples_per_pixel(&self) -> Option<f64> {
        self.ruler.samples_per_pixel()
    }
}

impl Tracks {
    pub fn sample_ix_to_screen_x(&self, sample_ix: f64) -> Option<f32> {
        self.ruler.sample_ix_to_screen_x(sample_ix)
    }
    pub fn screen_x_to_sample_ix(&self, screen_x: f32) -> Option<f64> {
        self.ruler.screen_x_to_sample_ix(screen_x)
    }
}

#[cfg(test)]
mod tests {
    use super::Tracks;
    use crate::{
        audio,
        model::action::SelectionEdge,
        model::config::TrackConfig,
        model::selection_info::{SelectionInfo, SelectionInfoE},
        rect::Rect,
    };

    fn insert_buffer(
        audio: &mut audio::manager::AudioManager,
        nr_samples: usize,
    ) -> audio::BufferId {
        let buffer =
            audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(48_000, 32, nr_samples));
        audio.buffers.insert(buffer)
    }

    #[test]
    fn zoom_to_selection_fits_selected_range() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..300).into(),
            screen_x_start: 10.0,
            screen_x_end: 30.0,
        });

        tracks
            .zoom_to_selection(&audio::manager::AudioManager::default())
            .unwrap();

        assert_eq!(tracks.ruler.ix_range().unwrap().start, 100.0);
        assert_eq!(tracks.ruler.ix_range().unwrap().end, 300.0);
        assert_eq!(tracks.samples_per_pixel(), Some(0.2));
    }

    #[test]
    fn zoom_to_selection_clamps_to_max_zoom_and_centers_selection() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..101).into(),
            screen_x_start: 10.0,
            screen_x_end: 11.0,
        });

        tracks
            .zoom_to_selection(&audio::manager::AudioManager::default())
            .unwrap();

        let ix_range = tracks.ruler.ix_range().unwrap();
        let selection_center = 100.5;
        let view_center = (ix_range.start + ix_range.end) / 2.0;
        assert_eq!(tracks.samples_per_pixel(), Some(0.002));
        assert!((view_center - selection_center).abs() < f64::EPSILON);
    }

    #[test]
    fn zoom_to_selection_without_selection_is_noop() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));

        tracks
            .zoom_to_selection(&audio::manager::AudioManager::default())
            .unwrap();

        assert_eq!(tracks.ruler.ix_range(), None);
    }

    #[test]
    fn zoom_to_selection_with_invalid_range_is_noop() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..100).into(),
            screen_x_start: 10.0,
            screen_x_end: 10.0,
        });

        tracks
            .zoom_to_selection(&audio::manager::AudioManager::default())
            .unwrap();

        assert_eq!(tracks.ruler.ix_range(), None);
    }

    #[test]
    fn zoom_to_selection_left_edge_centers_edge_and_uses_sample_level_zoom() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..300).into(),
            screen_x_start: 10.0,
            screen_x_end: 30.0,
        });

        tracks
            .zoom_to_selection_edge(
                &audio::manager::AudioManager::default(),
                SelectionEdge::Left,
            )
            .unwrap();

        let ix_range = tracks.ruler.ix_range().unwrap();
        assert_eq!(tracks.samples_per_pixel(), Some(0.1));
        assert_eq!(ix_range.start, 50.0);
        assert_eq!(ix_range.end, 150.0);
        assert_eq!(tracks.sample_ix_to_screen_x(100.0), Some(500.0));
    }

    #[test]
    fn zoom_to_selection_right_edge_centers_edge_and_uses_sample_level_zoom() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..300).into(),
            screen_x_start: 10.0,
            screen_x_end: 30.0,
        });

        tracks
            .zoom_to_selection_edge(
                &audio::manager::AudioManager::default(),
                SelectionEdge::Right,
            )
            .unwrap();

        let ix_range = tracks.ruler.ix_range().unwrap();
        assert_eq!(tracks.samples_per_pixel(), Some(0.1));
        assert_eq!(ix_range.start, 250.0);
        assert_eq!(ix_range.end, 350.0);
        assert_eq!(tracks.sample_ix_to_screen_x(300.0), Some(500.0));
    }

    #[test]
    fn zoom_to_selection_edge_without_selection_is_noop() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));

        tracks
            .zoom_to_selection_edge(
                &audio::manager::AudioManager::default(),
                SelectionEdge::Left,
            )
            .unwrap();

        assert_eq!(tracks.ruler.ix_range(), None);
    }

    #[test]
    fn zoom_to_selection_edge_with_invalid_range_is_noop() {
        let mut tracks = Tracks::default();
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 1000.0, 100.0));
        tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
            ix_rng: (100..100).into(),
            screen_x_start: 10.0,
            screen_x_end: 10.0,
        });

        tracks
            .zoom_to_selection_edge(
                &audio::manager::AudioManager::default(),
                SelectionEdge::Left,
            )
            .unwrap();

        assert_eq!(tracks.ruler.ix_range(), None);
    }

    #[test]
    fn fill_screen_height_only_updates_visible_tracks() {
        let mut tracks = Tracks {
            available_height: 120.0,
            ..Tracks::default()
        };
        let config = TrackConfig { min_height: 10.0 };
        let mut audio = audio::manager::AudioManager::default();
        let visible_a = insert_buffer(&mut audio, 64);
        let visible_b = insert_buffer(&mut audio, 64);
        let hidden = insert_buffer(&mut audio, 64);

        let visible_a = tracks.add_track_to_end(visible_a, &config).unwrap();
        let visible_b = tracks.add_track_to_end(visible_b, &config).unwrap();
        let hidden = tracks.add_track_to_end(hidden, &config).unwrap();

        tracks.set_track_height(visible_a, 10.0);
        tracks.set_track_height(visible_b, 15.0);
        tracks.set_track_height(hidden, 25.0);
        tracks.set_track_visibility(hidden, false);

        tracks.fill_screen_height(config.min_height).unwrap();

        assert_eq!(tracks.get_track_height(visible_a), Some(60.0));
        assert_eq!(tracks.get_track_height(visible_b), Some(60.0));
        assert_eq!(tracks.get_track_height(hidden), Some(25.0));
    }

    #[test]
    fn zoom_to_full_uses_only_visible_tracks() {
        let mut tracks = Tracks::default();
        let config = TrackConfig { min_height: 10.0 };
        let mut audio = audio::manager::AudioManager::default();
        let short = insert_buffer(&mut audio, 64);
        let long_hidden = insert_buffer(&mut audio, 640);

        let short = tracks.add_track_to_end(short, &config).unwrap();
        let long_hidden = tracks.add_track_to_end(long_hidden, &config).unwrap();
        tracks.set_track_visibility(long_hidden, false);
        tracks
            .ruler
            .set_screen_rect(Rect::new(0.0, 0.0, 64.0, 100.0));

        tracks.zoom_to_full(&audio).unwrap();

        let visible_track = tracks.get_track(short).unwrap();
        let hidden_track = tracks.get_track(long_hidden).unwrap();
        assert_eq!(tracks.ruler.ix_range().unwrap().end, 64.0);
        assert_eq!(visible_track.sample_rect.unwrap().ix_rng().end, 64.0);
        assert_eq!(hidden_track.sample_rect.unwrap().ix_rng().end, 64.0);
    }
}
