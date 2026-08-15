use crate::{model::config::TrackConfig, wav};
use anyhow::Result;
use slotmap::new_key_type;

#[path = "track/diff.rs"]
pub mod diff;
#[path = "track/single.rs"]
pub mod single;

use crate::{
    audio::manager::{AudioManager, BufferId},
    rect::Rect,
};
use diff::Diff;
use single::Single;

new_key_type! { pub struct TrackId; }
pub const HEADER_HEIGHT: f32 = 22.0;

pub fn min_total_height(track_config: &TrackConfig) -> f32 {
    track_config.min_height + HEADER_HEIGHT
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackMetaData {
    File(wav::file2::File, wav::ChIx),
    None,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Track {
    /// The pixel rectangle in absolute screen coordinates for the track
    /// Is updated by/for the view when displayed
    pub screen_rect: Option<Rect>,

    /// One item for now
    // track_item: TrackItem,
    pub single: Single,

    /// Present for tracks created by a diff action. `single` remains the render target for now;
    /// future diff render modes can use this metadata to draw/overlay the source buffers.
    pub diff: Option<Diff>,

    /// Sample rate of the underlying buffer. Needed to convert between the shared
    /// time-axis camera (seconds) and this track's sample indices.
    pub sample_rate: u32,

    track_md: TrackMetaData,

    // track height in gui
    pub height: f32,
    // track visibility in gui
    pub visible: bool,
    /// Draw a vertical guide at the peak index from the most recently computed statistics.
    pub show_peak_marker: bool,
}

impl Track {
    pub fn new2(buffer_id: BufferId, sample_rate: u32, track_config: &TrackConfig) -> Result<Self> {
        let single = Single::new(buffer_id)?;

        Ok(Self {
            screen_rect: None,
            single,
            diff: None,
            sample_rate,
            track_md: TrackMetaData::None,
            height: min_total_height(track_config),
            visible: true,
            show_peak_marker: false,
        })
    }

    pub fn new_diff(diff: Diff, sample_rate: u32, track_config: &TrackConfig) -> Result<Self> {
        let mut track = Self::new2(diff.buffer_id_diff, sample_rate, track_config)?;
        // The generated diff buffer is a normal waveform buffer, but it may start before/after
        // absolute sample zero depending on the source offsets.
        track.single.sample_ix_offset = diff.sample_ix_offset_diff as f64;
        track.diff = Some(diff);
        Ok(track)
    }
}

impl Track {
    pub fn set_screen_rect(&mut self, screen_rect: Rect) {
        if self.screen_rect != Some(screen_rect) {
            self.screen_rect = Some(screen_rect);
        }
        self.single.set_screen_rect(screen_rect);
    }

    /// Bring the inner items' view buffers up to date for this frame.
    /// Each item internally decides whether anything actually needs recomputing.
    pub fn update(&mut self, audio: &mut AudioManager) -> Result<()> {
        self.single.update(audio)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{
        self,
        buffer::{Buffer, BufferE},
        sample::view::ViewData,
    };
    use crate::model::{config::TrackConfig, ruler::ValueDisplayScale};

    fn insert_buffer(audio: &mut AudioManager, nr_samples: usize) -> BufferId {
        let mut buffer = Buffer::new(48_000, 32);
        buffer.data = (0..nr_samples)
            .map(|i| i as f32 / nr_samples as f32)
            .collect();
        let buffere = BufferE::F32(buffer);
        let buffer_id = audio.buffers.insert(std::sync::Arc::new(buffere.clone()));
        audio.thumbnails.insert(
            buffer_id,
            audio::thumbnail::ThumbnailE::from_buffer_e(&buffere, None),
        );
        buffer_id
    }

    #[test]
    fn set_screen_rect_keeps_track_and_single_geometry_in_sync() {
        let mut audio = AudioManager::default();
        let buffer_id = insert_buffer(&mut audio, 32);
        let mut track = Track::new2(buffer_id, 48_000, &TrackConfig::default()).unwrap();
        let waveform_rect = Rect::new(12.0, 34.0, 212.0, 134.0);

        track.set_screen_rect(waveform_rect);

        assert_eq!(track.screen_rect, Some(waveform_rect));
        assert_eq!(track.single.screen_rect(), Some(waveform_rect));
    }

    #[test]
    fn update_sample_view_replaces_stale_waveform_with_empty_view() {
        let mut audio = AudioManager::default();
        let buffer_id = insert_buffer(&mut audio, 32);
        let mut track = Track::new2(buffer_id, 48_000, &TrackConfig::default()).unwrap();
        track.set_screen_rect(Rect::new(0.0, 0.0, 16.0, 40.0));
        track.single.set_display_scale(ValueDisplayScale::default());

        track
            .single
            .set_ix_range((0.0..16.0).into(), &audio)
            .unwrap();
        track.update(&mut audio).unwrap();

        let initial_view = track.single.get_sample_view().unwrap();
        assert!(
            matches!(initial_view.data, ViewData::Single(ref data) if !data.samples.is_empty())
        );

        track
            .single
            .set_ix_range((64.0..96.0).into(), &audio)
            .unwrap();
        track.update(&mut audio).unwrap();

        let updated_view = track.single.get_sample_view().unwrap();
        assert_eq!(updated_view.data, ViewData::MinMax(vec![]));
    }
}
