pub mod action;
pub mod config;
pub mod hover_info;
pub mod ruler;
pub mod timeline;
pub mod track;
pub mod track2;
pub mod tracks;
pub mod tracks2;
pub mod view_buffer;

use crate::wav::read::ChIx;
use crate::{audio, model};

pub use action::Action;
pub use model::config::Config;
pub use model::timeline::Timeline;
pub use model::track::Track;
pub use model::tracks::Tracks;
pub use model::view_buffer::ViewBufferE;
use tracing::info;

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
    files: Vec<Rc<wav::File>>,

    files2: Vec<wav::file2::File>,
    // buffers: audio::BufferPool,
    audio: audio::manager::AudioManager,
    pub tracks2: tracks2::Tracks,

    pub actions: Vec<Action>,
}

pub type SharedModel = Rc<RefCell<Model>>;

impl Model {
    pub fn default_shared() -> SharedModel {
        Rc::new(RefCell::new(Model::default()))
    }
    // move self to shared model
    pub fn into_shared(self) -> SharedModel {
        Rc::new(RefCell::new(self))
    }

    // old
    pub fn add_wav_file(&mut self, path: &str, ch_ix: Option<ChIx>, offset: Option<u32>) -> Result<()> {
        println!("Adding wav file: {}", path);

        // Read file into float buffer
        let file = crate::wav::file::File::from_wav(path)?;

        println!("file.basename(): {}", file.basename());
        println!("file.nr_channels(): {}", file.nr_channels());

        if let Some(ch_ix) = ch_ix {
            if ch_ix >= file.nr_channels() as ChIx {
                return Err(anyhow::anyhow!("Channel {} out of range for file {}", ch_ix, path));
            }

            let name = format!("{} - ch {}", file.basename(), ch_ix);
            let track = model::track::Track::new(Rc::clone(&file.buffer), ch_ix, &name)?;
            self.tracks.push(track);
        } else {
            // For each channel create a model::track
            for (ch_ix, ch) in file.buffer.borrow().channels().enumerate() {
                // let name = format!("{} - ch {}", file.basename(), ix);
                let name = format!("{} - ch {}", file.file_path, ch_ix);
                let track = model::track::Track::new(Rc::clone(&file.buffer), ch_ix, &name)?;
                self.tracks.push(track);
            }
        }

        // Store file
        self.files.push(Rc::new(file));

        // New buffer api
        // let read_config = crate::wav::read::ReadConfig {
        //     filepath: std::path::PathBuf::from(path),
        //     ch_ixs: ch_ix.is_some().then(|| vec![ch_ix.unwrap()]),
        //     sample_range: sample::OptIxRange::default(),
        // };
        // let file2 = crate::wav::read::read_to_file(read_config, &mut self.buffers)?;
        // dbg!(file2);

        Ok(())
    }
}

impl Model {
    pub fn load_wav(&mut self, wav_read_config: wav::ReadConfig) -> Result<()> {
        let file = self.audio.load_file(wav_read_config)?;
        info!("Loaded file: {file}");

        if let Err(e) = self.tracks2.add_tracks_from_file(&file) {
            tracing::error!("Error adding tracks from file: {e}");
        }

        self.files2.push(file);

        // TODO
        Ok(())
    }
}

impl Model {
    /// Process actions we want to happen in between interacting with and drawing the UI
    pub fn process_actions(&mut self) {
        let actions: Vec<_> = self.actions.drain(..).collect();

        for action in actions {
            match action {
                Action::RemoveTrackOld(track_id) => {
                    self.tracks.remove_track(track_id);
                }
                Action::RemoveTrack(track_id) => todo!(),
                Action::RemoveAllTracks => {
                    self.tracks.tracks.clear();
                }
                Action::OpenFile(read_config) => {
                    // TODO: give extra info about the file
                    let _ = self.load_wav(read_config);
                }
                Action::ZoomToFull => {
                    // self.tracks.zoom_to_full();
                    // todo!();
                }
            }
        }
    }
}
