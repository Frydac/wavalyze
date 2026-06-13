pub mod action;
pub mod config;
pub mod demo;
pub mod diff_pairing;
pub mod hover_info;
pub mod jobs;
pub mod pending_drop;
pub mod ruler;
pub mod selection_info;
pub mod shortcuts;
pub mod time_camera;
pub mod track;
pub mod tracks2;
pub mod types;
pub mod view_buffer;

// Domain modules: each holds an `impl Model` block (and its unit tests) for one area, keeping this
// file focused on the struct and core lifecycle. See also `test_support` for shared test fixtures.
mod diff;
mod files;
mod loading;
#[cfg(test)]
mod test_support;

pub use self::config::Config;
pub use self::jobs::JobManager;
pub use self::time_camera::TimeCamera;
pub use self::types::{BitDepth, PixelCoord, SampleRate};
pub use self::view_buffer::ViewBufferE;
pub use files::FileVisibilityState;
pub use jobs::FinishedJob;
// pub use self::hover_info::HoverInfo;
use crate::audio;
pub use action::Action;

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
    /// Diff request awaiting channel-pair selection; the view shows a matrix dialog while `Some`.
    pub pending_diff_pairing: Option<diff_pairing::PendingDiffPairing>,
    /// Two dropped files awaiting a Diff/Load decision; the view shows a chooser dialog while
    /// `Some`. Only ever set on native (diff is native-only).
    pub pending_drop_choice: Option<pending_drop::PendingDropChoice>,
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
            pending_diff_pairing: None,
            pending_drop_choice: None,
        }
    }
}

impl Model {
    pub fn new() -> Self {
        let mut res = Self::default();
        res.tracks.width_info = res.user_config.tracks_width_info;
        res
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

    pub fn zoom_to_full(&mut self) -> Result<()> {
        self.tracks.zoom_to_full(&self.audio)
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
    use super::Model;
    use crate::audio::thumbnail::ThumbnailE;
    use crate::model::test_support::{add_buffer, make_file};

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
}
