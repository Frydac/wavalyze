pub mod action;
pub mod config;
pub mod demo;
pub mod hover_info;
pub mod load_manager;
pub mod ruler;
pub mod sample_ix_zoom;
pub mod selection_info;
pub mod shortcuts;
pub mod track;
pub mod tracks2;
pub mod types;
pub mod view_buffer;

pub use self::config::Config;
pub use self::sample_ix_zoom::SampleIxZoom;
pub use self::types::{BitDepth, PixelCoord, SampleRate};
pub use self::view_buffer::ViewBufferE;
pub use load_manager::{LoadManager, LoadProgressEntry};
// pub use self::hover_info::HoverInfo;
use crate::audio;
use crate::audio::thumbnail::ThumbnailE;
use crate::model::track::TrackId;
pub use action::Action;
use tracing::{info, trace};

// NOTE: move all under this?

use crate::wav;
use anyhow::Result;
// use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Model {
    pub user_config: Config,
    pub files2: Vec<wav::file2::File>,
    pub audio: audio::manager::AudioManager,
    pub tracks: tracks2::Tracks,
    pub actions: Vec<Action>,
    pub load_mgr: LoadManager,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileVisibilityState {
    NoneVisible,
    PartiallyVisible,
    AllVisible,
}

impl Model {
    pub fn new() -> Self {
        let mut res = Self::default();
        res.tracks.width_info = res.user_config.tracks_width_info;
        res
    }

    pub fn load_wav(&mut self, wav_read_config: &wav::ReadConfig) -> Result<()> {
        trace!("Loading wav file: {wav_read_config:?}");

        // Load buffers and associate with buffer id's in a File instance
        let file = self.audio.load_file(wav_read_config)?;
        info!("Loaded file: {file}");

        // Add tracks for the loaded buffers in the file
        if let Err(e) = self
            .tracks
            .add_tracks_from_file(&file, &self.user_config.track)
        {
            tracing::error!("Error adding tracks from file: {e}");
            return Err(e);
        }

        // Store the file instance itself
        self.files2.push(file);

        Ok(())
    }

    pub fn load_demo_waveform(&mut self) -> Result<()> {
        demo::load_demo_waveform(self)
    }

    pub fn get_file_channel_for_track(
        &self,
        track_id: TrackId,
    ) -> Option<(&wav::file2::File, &wav::file2::Channel)> {
        let track = self.tracks.get_track(track_id)?;
        let buffer_id = track.single.item.buffer_id;
        for file in self.files2.iter() {
            if let Some(channel) = file.get_channel(buffer_id) {
                return Some((file, channel));
            }
        }
        None
    }

    pub fn find_track_id_for_buffer(&self, buffer_id: audio::BufferId) -> Option<TrackId> {
        self.tracks
            .find_track(buffer_id)
            .map(|(track_id, _)| track_id)
    }

    pub fn file_visibility_state(&self, file: &wav::file2::File) -> FileVisibilityState {
        let mut any_visible = false;
        let mut any_hidden = false;

        for channel in file.channels.values() {
            match self.find_track_id_for_buffer(channel.buffer_id) {
                Some(track_id) => {
                    let is_visible = self
                        .tracks
                        .get_track(track_id)
                        .is_some_and(|track| track.visible);
                    if is_visible {
                        any_visible = true;
                    } else {
                        any_hidden = true;
                    }
                }
                None => {
                    any_hidden = true;
                }
            }
        }

        match (any_visible, any_hidden) {
            (true, true) => FileVisibilityState::PartiallyVisible,
            (true, false) => FileVisibilityState::AllVisible,
            _ => FileVisibilityState::NoneVisible,
        }
    }

    pub fn set_channel_visible(&mut self, buffer_id: audio::BufferId, visible: bool) -> bool {
        let Some(track_id) = self.find_track_id_for_buffer(buffer_id) else {
            return false;
        };
        self.tracks.set_track_visibility(track_id, visible);
        true
    }

    pub fn set_file_visible(&mut self, file: &wav::file2::File, visible: bool) {
        for channel in file.channels.values() {
            self.set_channel_visible(channel.buffer_id, visible);
        }
    }

    pub fn file_visibility_state_at(&self, file_ix: usize) -> Option<FileVisibilityState> {
        let file = self.files2.get(file_ix)?;
        Some(self.file_visibility_state(file))
    }

    pub fn set_file_visible_at(&mut self, file_ix: usize, visible: bool) -> bool {
        let Some(file) = self.files2.get(file_ix).cloned() else {
            return false;
        };
        self.set_file_visible(&file, visible);
        true
    }

    pub fn zoom_to_full(&mut self) -> Result<()> {
        self.tracks.zoom_to_full(&self.audio)
    }

    pub fn add_loaded_file(
        &mut self,
        loaded: wav::read::LoadedFile,
        progress: Option<wav::read::LoadProgressHandle>,
    ) -> Result<()> {
        let mut channels = std::collections::BTreeMap::new();
        if let Some(progress) = progress.as_ref() {
            progress.set_stage(
                wav::read::LoadStage::Thumbnail,
                loaded.channels.len() as u64,
            );
        }
        let mut thumbnail_count: u64 = 0;
        for (ch_ix, buffer) in loaded.channels {
            let thumbnail = ThumbnailE::from_buffer_e(&buffer, None);
            let buffer_id = self.audio.buffers.insert(buffer);
            self.audio.thumbnails.insert(buffer_id, thumbnail);
            channels.insert(
                ch_ix,
                wav::file2::Channel {
                    ch_ix,
                    buffer_id,
                    channel_id: None,
                },
            );
            thumbnail_count += 1;
            if let Some(progress) = progress.as_ref() {
                progress.set_current(thumbnail_count);
            }
        }

        let file = wav::file2::File {
            channels,
            sample_type: loaded.sample_type,
            bit_depth: loaded.bit_depth,
            sample_rate: loaded.sample_rate,
            layout: loaded.layout,
            path: loaded.path,
            nr_samples: loaded.nr_samples,
        };

        self.tracks
            .add_tracks_from_file(&file, &self.user_config.track)?;
        self.files2.push(file);

        Ok(())
    }

    pub fn drain_load_results(&mut self) -> bool {
        let mut had_results = false;
        let results = self.load_mgr.drain_results();
        for (result, progress) in results {
            had_results = true;
            match result {
                wav::read::LoadResult::Ok(loaded) => {
                    if let Err(err) = self.add_loaded_file(loaded, progress.clone()) {
                        tracing::error!("Failed to integrate loaded file: {err}");
                    } else {
                        if let Some(progress) = progress.as_ref() {
                            progress.set_stage(wav::read::LoadStage::Done, 1);
                            progress.set_current(1);
                        }
                        self.actions.push(Action::ZoomToFull);
                        self.actions.push(Action::FillScreenHeight);
                    }
                }
                wav::read::LoadResult::Err { error, .. } => {
                    tracing::error!("Failed to load wav file: {error}");
                }
            }
        }
        had_results
    }
}

impl Model {
    /// Process actions we want to happen in between interacting with and drawing the UI
    pub fn process_actions(&mut self) -> Result<()> {
        let actions: Vec<_> = self.actions.drain(..).collect();
        for action in actions {
            action.process(self)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{FileVisibilityState, Model};
    use crate::{
        audio,
        wav::{self, file2},
    };

    fn add_buffer(model: &mut Model) -> audio::BufferId {
        model.audio.buffers.insert(audio::buffer::BufferE::F32(
            audio::buffer::Buffer::with_size(48_000, 32, 16),
        ))
    }

    fn make_file(buffers: &[audio::BufferId]) -> file2::File {
        let channels = buffers
            .iter()
            .enumerate()
            .map(|(ch_ix, buffer_id)| {
                (
                    ch_ix as wav::read::ChIx,
                    file2::Channel {
                        ch_ix: ch_ix as wav::read::ChIx,
                        buffer_id: *buffer_id,
                        channel_id: None,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        file2::File {
            channels,
            sample_type: audio::SampleType::Float,
            bit_depth: 32,
            sample_rate: 48_000,
            layout: None,
            path: None,
            nr_samples: 16,
        }
    }

    #[test]
    fn file_visibility_state_tracks_partial_visibility() {
        let mut model = Model::new();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        model.files2.push(file);

        assert_eq!(
            model.file_visibility_state_at(0),
            Some(FileVisibilityState::AllVisible)
        );

        model.set_channel_visible(buffers[0], false);

        assert_eq!(
            model.file_visibility_state_at(0),
            Some(FileVisibilityState::PartiallyVisible)
        );

        model.set_file_visible_at(0, false);

        assert_eq!(
            model.file_visibility_state_at(0),
            Some(FileVisibilityState::NoneVisible)
        );
    }
}
