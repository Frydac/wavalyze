pub mod config;
pub mod track;
pub mod tracks;
pub mod view_buffer;

use crate::model;
pub use model::config::Config;
pub use model::track::Track;
pub use model::tracks::Tracks;
pub use model::view_buffer::ViewBuffer;

// NOTE: move all under this?

use crate::wav;
use anyhow::Result;
use std::cell::RefCell;
// use std::collections::VecDeque;
use std::rc::Rc;

use crate::audio::SampleIx;

#[derive(Default, Debug)]
pub struct Model {
    pub config: Config,
    pub tracks: Tracks,
    files: Vec<Rc<wav::File>>,
}

pub type SharedModel = Rc<RefCell<Model>>;

impl Model {
    pub fn default_shared() -> SharedModel {
        Rc::new(RefCell::new(Model::default()))
    }

    pub fn add_wav_file(&mut self, path: &str, channel: Option<u32>, offset: Option<u32>) -> Result<()> {
        println!("Adding wav file: {}", path);

        // Read file into float buffer
        let file = crate::wav::file::File::from_wav(path)?;

        println!("file.basename(): {}", file.basename());
        println!("file.nr_channels(): {}", file.nr_channels());

        if let Some(channel) = channel {
            if channel >= file.nr_channels() as u32 {
                return Err(anyhow::anyhow!("Channel {} out of range for file {}", channel, path));
            }

            let name = format!("{} - ch {}", file.basename(), channel);
            let track = model::track::Track::new(Rc::clone(&file.buffer), channel as usize, &name)?;
            self.tracks.push(track);
        } else {
            // For each channel create a model::track
            for (ix, ch) in file.buffer.borrow().channels().enumerate() {
                // let name = format!("{} - ch {}", file.basename(), ix);
                let name = format!("{} - ch {}", file.file_path, ix);
                let track = model::track::Track::new(Rc::clone(&file.buffer), ix, &name)?;
                self.tracks.push(track);
            }
        }

        // Store file
        self.files.push(Rc::new(file));

        Ok(())
    }
}

impl Model {}
