pub mod action;
pub mod config;
pub mod demo;
pub mod hover_info;
pub mod load_manager;
pub mod ruler;
pub mod sample_ix_zoom;
pub mod track;
pub mod track2;
pub mod tracks;
pub mod tracks2;
pub mod types;
pub mod view_buffer;

pub use self::config::Config;
pub use self::sample_ix_zoom::SampleIxZoom;
pub use self::track::Track;
pub use self::tracks::Tracks;
pub use self::types::{BitDepth, PixelCoord, SampleRate};
pub use self::view_buffer::ViewBufferE;
pub use load_manager::{LoadManager, LoadProgressEntry};
// pub use self::hover_info::HoverInfo;
use crate::audio;
use crate::audio::thumbnail::ThumbnailE;
use crate::model::track2::TrackId;
pub use action::Action;
use tracing::{info, trace};

// NOTE: move all under this?

use crate::wav;
use anyhow::Result;
// use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Model {
    pub user_config: Config,
    pub tracks: Tracks,
    // files: Vec<Rc<wav::File>>,
    pub files2: Vec<wav::file2::File>,
    // buffers: audio::BufferPool,
    pub audio: audio::manager::AudioManager,
    pub tracks2: tracks2::Tracks,

    pub actions: Vec<Action>,

    pub load_mgr: LoadManager,
}

impl Model {
    pub fn new() -> Self {
        let mut res = Self::default();
        res.tracks2.width_info = res.user_config.tracks_width_info;
        res
    }

    pub fn load_wav(&mut self, wav_read_config: &wav::ReadConfig) -> Result<()> {
        trace!("Loading wav file: {wav_read_config:?}");

        // Load buffers and associate with buffer id's in a File instance
        let file = self.audio.load_file(wav_read_config)?;
        info!("Loaded file: {file}");

        // Add tracks for the loaded buffers in the file
        if let Err(e) = self
            .tracks2
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
        let track = self.tracks2.get_track(track_id)?;
        let buffer_id = track.single.item.buffer_id;
        for file in self.files2.iter() {
            if let Some(channel) = file.get_channel(buffer_id) {
                return Some((file, channel));
            }
        }
        None
    }

    pub fn zoom_to_full(&mut self) -> Result<()> {
        self.tracks2.zoom_to_full(&self.audio)
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

        self.tracks2
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
