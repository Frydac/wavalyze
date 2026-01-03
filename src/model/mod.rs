pub mod action;
pub mod config;
pub mod hover_info;
pub mod ix_zoom_offset;
pub mod ruler;
pub mod track;
pub mod track2;
pub mod tracks;
pub mod tracks2;
pub mod types;
pub mod view_buffer;

use crate::audio::thumbnail::ThumbnailE;
use crate::model::track2::TrackId;
use crate::{audio, model};

pub use action::Action;
pub use model::config::Config;
pub use model::ix_zoom_offset::IxZoomOffset;
pub use model::track::Track;
pub use model::tracks::Tracks;
pub use model::types::PixelCoord;
pub use model::view_buffer::ViewBufferE;
use tracing::{info, trace};

// NOTE: move all under this?

use crate::wav;
use anyhow::Result;
use std::cell::RefCell;
// use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Default, Debug)]
pub struct Model {
    pub user_config: Config,
    pub tracks: Tracks,
    // files: Vec<Rc<wav::File>>,
    pub files2: Vec<wav::file2::File>,
    // buffers: audio::BufferPool,
    pub audio: audio::manager::AudioManager,
    pub tracks2: tracks2::Tracks,

    pub actions: Vec<Action>,
}

pub type SharedModel = Rc<RefCell<Model>>;

impl Model {
    pub fn split_mut(&mut self) -> (&mut model::ruler::Time, &mut Vec<model::action::Action>) {
        (&mut self.tracks2.ruler, &mut self.actions)
    }

    pub fn default_shared() -> SharedModel {
        Rc::new(RefCell::new(Model::default()))
    }
    // move self to shared model
    pub fn into_shared(self) -> SharedModel {
        Rc::new(RefCell::new(self))
    }
}

impl Model {
    pub fn load_wav(&mut self, wav_read_config: &wav::ReadConfig) -> Result<()> {
        trace!("Loading wav file: {wav_read_config:?}");

        // Load buffers and associate with buffer id's in a File instance
        let file = self.audio.load_file(wav_read_config)?;
        info!("Loaded file: {file}");

        // Add tracks for the loaded buffers in the file
        if let Err(e) = self.tracks2.add_tracks_from_file(&file, &self.user_config.track) {
            tracing::error!("Error adding tracks from file: {e}");
            return Err(e);
        }

        // Store the file instance itself
        self.files2.push(file);

        Ok(())
    }

    pub fn find_file_channel_for_track(&self, track_id: TrackId) -> Option<(&wav::file2::File, &wav::file2::Channel)> {
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
