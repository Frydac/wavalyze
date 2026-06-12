pub mod action;
pub mod config;
pub mod demo;
pub mod hover_info;
pub mod jobs;
pub mod ruler;
pub mod selection_info;
pub mod shortcuts;
pub mod time_camera;
pub mod track;
pub mod tracks2;
pub mod types;
pub mod view_buffer;

pub use self::config::Config;
pub use self::jobs::JobManager;
pub use self::time_camera::TimeCamera;
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
use crate::wav::file2::FileId;
use anyhow::Result;
use slotmap::SlotMap;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct Model {
    pub user_config: Config,
    pub files: SlotMap<FileId, wav::file2::File>,
    pub files_order: Vec<FileId>,
    pub audio: audio::manager::AudioManager,
    pub tracks: tracks2::Tracks,
    pub actions: Vec<Action>,
    /// Sender cloned to background workers so they can push follow-up actions back into the
    /// model's action queue. Drained into `actions` each frame via `drain_action_messages`.
    pub actions_tx: Sender<Action>,
    actions_rx: Receiver<Action>,
    pub job_mgr: JobManager,

    /// Monotonic token for async job results. Close All increments this so load/diff workers that
    /// were already running can finish without repopulating a cleared model; their integration
    /// actions are ignored when tagged with an older generation.
    generation: u64,
}

impl Default for Model {
    fn default() -> Self {
        let (actions_tx, actions_rx) = std::sync::mpsc::channel();
        Self {
            user_config: Config::default(),
            files: SlotMap::default(),
            files_order: Vec::new(),
            audio: audio::manager::AudioManager::default(),
            tracks: tracks2::Tracks::default(),
            actions: Vec::new(),
            actions_tx,
            actions_rx,
            job_mgr: JobManager::default(),
            generation: 0,
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
        self.insert_file(file);

        Ok(())
    }

    /// Insert a file into the slotmap and append it to the display/order vec. The two fields
    /// are always mutated together through this helper (and `clear_files`/`remove_file`).
    pub fn insert_file(&mut self, file: wav::file2::File) -> FileId {
        let id = self.files.insert(file);
        self.files_order.push(id);
        id
    }

    pub fn clear_files(&mut self) {
        self.files.clear();
        self.files_order.clear();
    }

    pub fn close_all(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.tracks.remove_all_tracks();
        self.tracks.hover_info = Default::default();
        self.tracks.selection_info = Default::default();
        self.files.clear();
        self.files_order.clear();
        self.audio.clear();
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn is_current_generation(&self, generation: u64) -> bool {
        self.generation == generation
    }

    pub fn load_demo_waveform(&mut self) -> Result<()> {
        demo::load_demo_waveform(self)
    }

    pub fn get_file_channel_for_track(
        &self,
        track_id: TrackId,
    ) -> Option<(&wav::file2::File, &wav::file2::Channel)> {
        let track = self.tracks.get_track(track_id)?;
        let buffer_id = track.single.buffer_id;
        for file in self.files.values() {
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

    pub fn file_visibility_state_for(&self, file_id: FileId) -> Option<FileVisibilityState> {
        let file = self.files.get(file_id)?;
        Some(self.file_visibility_state(file))
    }

    pub fn set_file_visible_for(&mut self, file_id: FileId, visible: bool) -> bool {
        let Some(file) = self.files.get(file_id).cloned() else {
            return false;
        };
        self.set_file_visible(&file, visible);
        true
    }

    /// Unload an entire file: remove all of its channel tracks, drop the underlying audio
    /// buffers, and discard the file spec itself.
    pub fn remove_file(&mut self, file_id: FileId) -> bool {
        let Some(file) = self.files.get(file_id).cloned() else {
            return false;
        };
        for channel in file.channels.values() {
            if let Some(track_id) = self.find_track_id_for_buffer(channel.buffer_id) {
                self.tracks.remove_track(track_id);
            }
        }
        self.audio.remove_buffers_from_file(&file);
        self.files.remove(file_id);
        self.files_order.retain(|id| *id != file_id);
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
        let sample_rate = self.audio.get_buffer(buffer_id)?.sample_rate();
        let track_id =
            self.tracks
                .insert_track(buffer_id, sample_rate, insert_ix, &self.user_config.track)?;
        self.tracks.set_track_height(
            track_id,
            crate::model::track::min_total_height(&self.user_config.track),
        );
        Ok(true)
    }

    fn track_insert_index_for_buffer(&self, buffer_id: audio::BufferId) -> Option<usize> {
        let mut insert_ix = 0;
        for file in self.files_order.iter().filter_map(|id| self.files.get(*id)) {
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

    pub fn add_loaded_file(&mut self, loaded: wav::read::LoadedFile) -> Result<FileId> {
        let mut channels = std::collections::BTreeMap::new();
        let mut thumbnails = loaded.thumbnails;
        for (ch_ix, buffer) in loaded.channels {
            let thumbnail = thumbnails
                .remove(&ch_ix)
                .unwrap_or_else(|| ThumbnailE::from_buffer_e(&buffer, None));
            let buffer_id = self.audio.buffers.insert(std::sync::Arc::new(buffer));
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
            sample_ix_offset: loaded.sample_ix_offset,
        };

        self.tracks
            .add_tracks_from_file(&file, &self.user_config.track)?;
        let file_id = self.insert_file(file);

        Ok(file_id)
    }

    pub fn add_loaded_diff(&mut self, diff: jobs::LoadedDiff) -> Result<()> {
        // The CLI diff job loads both source files and computes the diff on a worker. Integrate all
        // three tracks together so the visible order is deterministic: A, B, Diff.
        let file_a_id = self.add_loaded_file(diff.file_a)?;
        let file_b_id = self.add_loaded_file(diff.file_b)?;
        let buffer_id_a = self.single_channel_buffer_id(file_a_id)?;
        let buffer_id_b = self.single_channel_buffer_id(file_b_id)?;
        let sample_rate = self.audio.get_buffer(buffer_id_a)?.sample_rate();
        anyhow::ensure!(
            sample_rate == self.audio.get_buffer(buffer_id_b)?.sample_rate(),
            "diff source sample rates differ"
        );

        let buffer_id_diff = self
            .audio
            .buffers
            .insert(std::sync::Arc::new(diff.diff_buffer));
        self.audio
            .thumbnails
            .insert(buffer_id_diff, diff.diff_thumbnail);
        self.tracks.add_diff_track_to_end(
            track::diff::Diff {
                buffer_id_diff,
                buffer_id_a,
                buffer_id_b,
                sample_ix_offset_a: diff.sample_ix_offset_a,
                sample_ix_offset_b: diff.sample_ix_offset_b,
                sample_ix_offset_diff: diff.sample_ix_offset_diff,
            },
            sample_rate,
            &self.user_config.track,
        )?;

        Ok(())
    }

    pub fn add_diff_buffer(&mut self, diff: jobs::ComputedDiff) -> Result<()> {
        // Existing-buffer diffs skip file integration and only add the computed render buffer plus
        // Diff metadata pointing back to the already-loaded source buffers.
        let sample_rate = self.audio.get_buffer(diff.buffer_id_a)?.sample_rate();
        anyhow::ensure!(
            sample_rate == self.audio.get_buffer(diff.buffer_id_b)?.sample_rate(),
            "diff source sample rates differ"
        );
        let buffer_id_diff = self
            .audio
            .buffers
            .insert(std::sync::Arc::new(diff.diff_buffer));
        self.audio
            .thumbnails
            .insert(buffer_id_diff, diff.diff_thumbnail);
        self.tracks.add_diff_track_to_end(
            track::diff::Diff {
                buffer_id_diff,
                buffer_id_a: diff.buffer_id_a,
                buffer_id_b: diff.buffer_id_b,
                sample_ix_offset_a: diff.sample_ix_offset_a,
                sample_ix_offset_b: diff.sample_ix_offset_b,
                sample_ix_offset_diff: diff.sample_ix_offset_diff,
            },
            sample_rate,
            &self.user_config.track,
        )?;

        Ok(())
    }

    fn single_channel_buffer_id(&self, file_id: FileId) -> Result<audio::BufferId> {
        let file = self
            .files
            .get(file_id)
            .ok_or_else(|| anyhow::anyhow!("File {:?} not found", file_id))?;
        anyhow::ensure!(
            file.channels.len() == 1,
            "expected exactly one channel, got {}",
            file.channels.len()
        );
        file.channels
            .values()
            .next()
            .map(|channel| channel.buffer_id)
            .ok_or_else(|| anyhow::anyhow!("file has no channels"))
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
        let generation = self.generation();
        jobs::spawn_load_wav_job(
            job_id,
            generation,
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
        let generation = self.generation();
        jobs::spawn_load_wav_path_job(
            job_id,
            generation,
            config,
            self.job_mgr.sender(),
            self.actions_tx.clone(),
        );
        job_id
    }

    pub fn start_compute_rms_job(&mut self, buffer_id: audio::BufferId) -> Result<jobs::JobId> {
        let buffer = self.audio.buffer_arc(buffer_id)?;
        let job_id = self
            .job_mgr
            .start_job(jobs::JobKind::ComputeRms, format!("RMS {buffer_id:?}"));
        jobs::spawn_compute_rms_job(
            job_id,
            buffer_id,
            buffer,
            self.job_mgr.sender(),
            self.actions_tx.clone(),
        );
        Ok(job_id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_load_diff_paths_job(
        &mut self,
        file_a: wav::ReadConfig,
        file_b: wav::ReadConfig,
    ) -> jobs::JobId {
        let label = format!(
            "Diff {} - {}",
            file_a
                .filepath
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("A"),
            file_b
                .filepath
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("B")
        );
        let job_id = self.job_mgr.start_job(jobs::JobKind::Diff, label);
        let generation = self.generation();
        jobs::spawn_load_diff_paths_job(
            job_id,
            generation,
            file_a,
            file_b,
            self.job_mgr.sender(),
            self.actions_tx.clone(),
        );
        job_id
    }

    pub fn start_diff_buffers_job(
        &mut self,
        buffer_id_a: audio::BufferId,
        buffer_id_b: audio::BufferId,
        sample_ix_offset_a: audio::sample::Ix,
        sample_ix_offset_b: audio::sample::Ix,
    ) -> Result<jobs::JobId> {
        let buffer_a = self.audio.buffer_arc(buffer_id_a)?;
        let buffer_b = self.audio.buffer_arc(buffer_id_b)?;
        anyhow::ensure!(
            buffer_a.sample_rate() == buffer_b.sample_rate(),
            "diff inputs must have the same sample rate ({} != {})",
            buffer_a.sample_rate(),
            buffer_b.sample_rate()
        );
        let job_id = self.job_mgr.start_job(
            jobs::JobKind::Diff,
            format!("Diff {buffer_id_a:?} - {buffer_id_b:?}"),
        );
        let generation = self.generation();
        jobs::spawn_diff_buffers_job(
            job_id,
            generation,
            jobs::diff::DiffBuffersJobInput {
                buffer_id_a,
                buffer_id_b,
                buffer_a,
                buffer_b,
                sample_ix_offset_a,
                sample_ix_offset_b,
            },
            self.job_mgr.sender(),
            self.actions_tx.clone(),
        );
        Ok(job_id)
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

    use super::{Action, FileVisibilityState, Model};
    use crate::{
        audio,
        audio::thumbnail::ThumbnailE,
        model::{jobs, track},
        wav::{self, file2},
    };

    fn add_buffer(model: &mut Model) -> audio::BufferId {
        model
            .audio
            .buffers
            .insert(std::sync::Arc::new(audio::buffer::BufferE::F32(
                audio::buffer::Buffer::with_size(48_000, 32, 16),
            )))
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
            sample_ix_offset: 0,
        }
    }

    fn loaded_file_with_one_buffer(
        buffer: audio::buffer::BufferE,
        sample_ix_offset: audio::sample::Ix,
    ) -> wav::read::LoadedFile {
        let mut channels = BTreeMap::new();
        let mut thumbnails = BTreeMap::new();
        channels.insert(0, buffer.clone());
        thumbnails.insert(0, ThumbnailE::from_buffer_e(&buffer, None));
        wav::read::LoadedFile {
            load_id: 0,
            channels,
            thumbnails,
            sample_type: audio::SampleType::Float,
            bit_depth: 32,
            sample_rate: 48_000,
            layout: None,
            path: None,
            nr_samples: buffer.nr_samples() as u64,
            sample_ix_offset,
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
        let file_id = model.insert_file(file);

        assert_eq!(
            model.file_visibility_state_for(file_id),
            Some(FileVisibilityState::AllVisible)
        );

        model.set_channel_visible(buffers[0], false);

        assert_eq!(
            model.file_visibility_state_for(file_id),
            Some(FileVisibilityState::PartiallyVisible)
        );

        model.set_file_visible_for(file_id, false);

        assert_eq!(
            model.file_visibility_state_for(file_id),
            Some(FileVisibilityState::NoneVisible)
        );
    }

    #[test]
    fn add_loaded_diff_creates_two_source_tracks_and_diff_track() {
        let mut model = Model::default();
        let buffer_a = audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(48_000, 32, 4));
        let buffer_b = audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(48_000, 32, 4));
        let diff_buffer =
            audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(48_000, 32, 4));
        let diff_thumbnail = ThumbnailE::from_buffer_e(&diff_buffer, None);

        model
            .add_loaded_diff(jobs::LoadedDiff {
                file_a: loaded_file_with_one_buffer(buffer_a, -2),
                file_b: loaded_file_with_one_buffer(buffer_b, 3),
                sample_ix_offset_a: -2,
                sample_ix_offset_b: 3,
                sample_ix_offset_diff: 0,
                diff_buffer,
                diff_thumbnail,
            })
            .unwrap();

        assert_eq!(model.tracks.tracks_order.len(), 3);
        let diff_track = model
            .tracks
            .get_track(*model.tracks.tracks_order.last().unwrap())
            .unwrap();
        assert!(diff_track.diff.is_some());
        assert_eq!(model.files_order.len(), 2);
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
        let file_id = model.insert_file(file.clone());

        assert!(model.remove_channel_track(buffers[0]));
        assert!(model.find_track_id_for_buffer(buffers[0]).is_none());
        assert_eq!(
            model.file_visibility_state_for(file_id),
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
        let file_id = model.insert_file(file);

        assert!(model.remove_channel_track(buffers[0]));
        assert!(model.restore_channel_track(buffers[0]).unwrap());
        let restored_track_id = model.find_track_id_for_buffer(buffers[0]).unwrap();

        assert_eq!(model.tracks.tracks_order.len(), 2);
        assert_eq!(model.tracks.tracks_order[0], restored_track_id);
        assert!(model.tracks.get_track(restored_track_id).unwrap().visible);
        assert_eq!(
            model.file_visibility_state_for(file_id),
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
        model.insert_file(file);

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
        model.insert_file(first_file);
        model.insert_file(second_file);

        assert!(model.remove_channel_track(first_file_buffers[1]));
        assert!(model.restore_channel_track(first_file_buffers[1]).unwrap());

        let ordered_buffers: Vec<_> = model
            .tracks
            .tracks_order
            .iter()
            .map(|track_id| model.tracks.get_track(*track_id).unwrap().single.buffer_id)
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
        model.insert_file(file);

        assert!(!model.restore_channel_track(buffers[0]).unwrap());
        assert_eq!(model.tracks.tracks_order.len(), 2);
    }

    #[test]
    fn close_all_clears_files_tracks_and_audio() {
        let mut model = Model::new();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let thumbnail =
            ThumbnailE::from_buffer_e(model.audio.get_buffer(buffers[0]).unwrap(), None);
        model.audio.thumbnails.insert(buffers[0], thumbnail);
        model.audio.rms_db.insert(buffers[1], -12.0);
        model.tracks.hover_info =
            crate::model::hover_info::HoverInfoE::IsHovered(Default::default());
        model.tracks.selection_info =
            crate::model::selection_info::SelectionInfoE::IsSelected(Default::default());
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        model.insert_file(file);

        model.close_all();

        assert!(model.files.is_empty());
        assert!(model.files_order.is_empty());
        assert!(model.tracks.tracks.is_empty());
        assert!(model.tracks.tracks_order.is_empty());
        assert_eq!(
            model.tracks.hover_info,
            crate::model::hover_info::HoverInfoE::NotHovered
        );
        assert_eq!(
            model.tracks.selection_info,
            crate::model::selection_info::SelectionInfoE::NotSelected
        );
        assert!(model.audio.buffers.is_empty());
        assert!(model.audio.thumbnails.is_empty());
        assert!(model.audio.rms_db.is_empty());
    }

    #[test]
    fn stale_loaded_file_integration_after_close_all_is_ignored() {
        let mut model = Model::new();
        let generation = model.generation();
        model.close_all();
        let buffer = audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(48_000, 32, 4));

        Action::IntegrateLoadedFile {
            generation,
            loaded: loaded_file_with_one_buffer(buffer, 0),
        }
        .process(&mut model)
        .unwrap();

        assert!(model.files.is_empty());
        assert!(model.tracks.tracks.is_empty());
        assert!(model.audio.buffers.is_empty());
    }

    #[test]
    fn stale_loaded_diff_integration_after_close_all_is_ignored() {
        let mut model = Model::new();
        let generation = model.generation();
        model.close_all();
        let buffer_a = audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(48_000, 32, 4));
        let buffer_b = audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(48_000, 32, 4));
        let diff_buffer =
            audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(48_000, 32, 4));
        let diff_thumbnail = ThumbnailE::from_buffer_e(&diff_buffer, None);

        Action::IntegrateLoadedDiff {
            generation,
            diff: jobs::LoadedDiff {
                file_a: loaded_file_with_one_buffer(buffer_a, 0),
                file_b: loaded_file_with_one_buffer(buffer_b, 0),
                sample_ix_offset_a: 0,
                sample_ix_offset_b: 0,
                sample_ix_offset_diff: 0,
                diff_buffer,
                diff_thumbnail,
            },
        }
        .process(&mut model)
        .unwrap();

        assert!(model.files.is_empty());
        assert!(model.tracks.tracks.is_empty());
        assert!(model.audio.buffers.is_empty());
    }
}
