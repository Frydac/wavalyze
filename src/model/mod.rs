pub mod action;
pub mod config;
pub mod demo;
pub mod hover_info;
pub mod jobs;
pub mod ruler;
pub mod sample_ix_zoom;
pub mod selection_info;
pub mod shortcuts;
pub mod track;
pub mod tracks2;
pub mod types;
pub mod view_buffer;

pub use self::config::Config;
pub use self::jobs::JobManager;
pub use self::sample_ix_zoom::SampleIxZoom;
pub use self::types::{BitDepth, PixelCoord, SampleRate};
pub use self::view_buffer::ViewBufferE;
pub use jobs::FinishedJob;
// pub use self::hover_info::HoverInfo;
use crate::audio;
use crate::audio::thumbnail::ThumbnailE;
use crate::model::track::TrackId;
pub use action::Action;
use tracing::{info, trace};

// NOTE: move all under this?

use crate::wav;
use anyhow::Result;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct Model {
    pub user_config: Config,
    pub files2: Vec<wav::file2::File>,
    pub audio: audio::manager::AudioManager,
    pub tracks: tracks2::Tracks,
    pub actions: Vec<Action>,
    /// Sender cloned to background workers so they can push follow-up actions back into the
    /// model's action queue. Drained into `actions` each frame via `drain_action_messages`.
    pub actions_tx: Sender<Action>,
    actions_rx: Receiver<Action>,
    pub job_mgr: JobManager,
}

impl Default for Model {
    fn default() -> Self {
        let (actions_tx, actions_rx) = std::sync::mpsc::channel();
        Self {
            user_config: Config::default(),
            files2: Vec::new(),
            audio: audio::manager::AudioManager::default(),
            tracks: tracks2::Tracks::default(),
            actions: Vec::new(),
            actions_tx,
            actions_rx,
            job_mgr: JobManager::default(),
        }
    }
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

    pub fn remove_channel_track(&mut self, buffer_id: audio::BufferId) -> bool {
        let Some(track_id) = self.find_track_id_for_buffer(buffer_id) else {
            return false;
        };
        self.tracks.remove_track(track_id);
        true
    }

    pub fn restore_channel_track(&mut self, buffer_id: audio::BufferId) -> Result<bool> {
        if self.find_track_id_for_buffer(buffer_id).is_some() {
            return Ok(false);
        }
        let Some(insert_ix) = self.track_insert_index_for_buffer(buffer_id) else {
            return Ok(false);
        };
        let track_id = self
            .tracks
            .insert_track(buffer_id, insert_ix, &self.user_config.track)?;
        self.tracks.set_track_height(
            track_id,
            crate::model::track::min_total_height(&self.user_config.track),
        );
        Ok(true)
    }

    fn track_insert_index_for_buffer(&self, buffer_id: audio::BufferId) -> Option<usize> {
        let mut insert_ix = 0;
        for file in &self.files2 {
            for channel in file.channels.values() {
                if channel.buffer_id == buffer_id {
                    return Some(insert_ix);
                }
                if self.find_track_id_for_buffer(channel.buffer_id).is_some() {
                    insert_ix += 1;
                }
            }
        }
        None
    }

    pub fn zoom_to_full(&mut self) -> Result<()> {
        self.tracks.zoom_to_full(&self.audio)
    }

    pub fn add_loaded_file(&mut self, loaded: wav::read::LoadedFile) -> Result<()> {
        let mut channels = std::collections::BTreeMap::new();
        let mut thumbnails = loaded.thumbnails;
        for (ch_ix, buffer) in loaded.channels {
            let thumbnail = thumbnails
                .remove(&ch_ix)
                .unwrap_or_else(|| ThumbnailE::from_buffer_e(&buffer, None));
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

    pub fn drain_job_events(&mut self) -> bool {
        self.job_mgr.drain_events()
    }

    /// Drain actions queued by background workers into the synchronous action queue. Run each
    /// frame before `process_actions` so worker side-effects land on the next dispatch pass.
    pub fn drain_action_messages(&mut self) -> bool {
        let mut had_messages = false;
        while let Ok(action) = self.actions_rx.try_recv() {
            had_messages = true;
            self.actions.push(action);
        }
        had_messages
    }

    pub fn start_demo_job(&mut self, config: jobs::DemoTimedConfig) -> jobs::JobId {
        let label_ix = self.job_mgr.pending() + 1;
        let job_id = self
            .job_mgr
            .start_job(jobs::JobKind::DemoTimed, format!("Demo job #{label_ix}"));
        jobs::spawn_demo_timed_job(
            job_id,
            config,
            self.job_mgr.sender(),
            self.actions_tx.clone(),
        );
        job_id
    }

    pub fn start_load_wav_job(&mut self, config: wav::ReadConfigBytes) -> jobs::JobId {
        let label = config.name.clone().unwrap_or_else(|| "file".to_string());
        let job_id = self.job_mgr.start_job(jobs::JobKind::LoadWav, label);
        jobs::spawn_load_wav_job(
            job_id,
            config,
            self.job_mgr.sender(),
            self.actions_tx.clone(),
        );
        job_id
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_load_wav_path_job(&mut self, config: wav::ReadConfig) -> jobs::JobId {
        let label = config
            .filepath
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("file")
            .to_string();
        let job_id = self.job_mgr.start_job(jobs::JobKind::LoadWav, label);
        jobs::spawn_load_wav_path_job(
            job_id,
            config,
            self.job_mgr.sender(),
            self.actions_tx.clone(),
        );
        job_id
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
        model::track,
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

    #[test]
    fn remove_channel_track_marks_channel_missing() {
        let mut model = Model::new();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        model.files2.push(file.clone());

        assert!(model.remove_channel_track(buffers[0]));
        assert!(model.find_track_id_for_buffer(buffers[0]).is_none());
        assert_eq!(
            model.file_visibility_state_at(0),
            Some(FileVisibilityState::PartiallyVisible)
        );
        assert!(
            model
                .get_file_channel_for_track(model.find_track_id_for_buffer(buffers[1]).unwrap())
                .is_some()
        );
    }

    #[test]
    fn restore_channel_track_recreates_missing_track() {
        let mut model = Model::new();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        model.files2.push(file);

        assert!(model.remove_channel_track(buffers[0]));
        assert!(model.restore_channel_track(buffers[0]).unwrap());
        let restored_track_id = model.find_track_id_for_buffer(buffers[0]).unwrap();

        assert_eq!(model.tracks.tracks_order.len(), 2);
        assert_eq!(model.tracks.tracks_order[0], restored_track_id);
        assert!(model.tracks.get_track(restored_track_id).unwrap().visible);
        assert_eq!(
            model.file_visibility_state_at(0),
            Some(FileVisibilityState::AllVisible)
        );
    }

    #[test]
    fn restore_channel_track_uses_minimum_track_height() {
        let mut model = Model::new();
        model.user_config.track.min_height = 42.0;
        let buffers = [add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        model.files2.push(file);

        let track_id = model.find_track_id_for_buffer(buffers[0]).unwrap();
        model.tracks.set_track_height(track_id, 120.0);

        assert!(model.remove_channel_track(buffers[0]));
        assert!(model.restore_channel_track(buffers[0]).unwrap());

        let restored_track_id = model.find_track_id_for_buffer(buffers[0]).unwrap();
        assert_eq!(
            model.tracks.get_track(restored_track_id).unwrap().height,
            42.0 + track::HEADER_HEIGHT
        );
    }

    #[test]
    fn restore_channel_track_preserves_file_channel_order() {
        let mut model = Model::new();
        let first_file_buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let second_file_buffers = [add_buffer(&mut model)];
        let first_file = make_file(&first_file_buffers);
        let second_file = make_file(&second_file_buffers);

        model
            .tracks
            .add_tracks_from_file(&first_file, &model.user_config.track)
            .unwrap();
        model
            .tracks
            .add_tracks_from_file(&second_file, &model.user_config.track)
            .unwrap();
        model.files2.push(first_file);
        model.files2.push(second_file);

        assert!(model.remove_channel_track(first_file_buffers[1]));
        assert!(model.restore_channel_track(first_file_buffers[1]).unwrap());

        let ordered_buffers: Vec<_> = model
            .tracks
            .tracks_order
            .iter()
            .map(|track_id| {
                model
                    .tracks
                    .get_track(*track_id)
                    .unwrap()
                    .single
                    .item
                    .buffer_id
            })
            .collect();
        assert_eq!(
            ordered_buffers,
            vec![
                first_file_buffers[0],
                first_file_buffers[1],
                second_file_buffers[0]
            ]
        );
    }

    #[test]
    fn restore_channel_track_is_noop_when_track_already_loaded() {
        let mut model = Model::new();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        model.files2.push(file);

        assert!(!model.restore_channel_track(buffers[0]).unwrap());
        assert_eq!(model.tracks.tracks_order.len(), 2);
    }
}
