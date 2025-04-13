pub mod track;
pub mod tracks;
pub mod view_buffer;

use crate::model;
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
    pub tracks: Tracks,
    files: Vec<Rc<wav::File>>,
}

pub type SharedModel = Rc<RefCell<Model>>;

impl Model {
    pub fn default_shared() -> SharedModel {
        Rc::new(RefCell::new(Model::default()))
    }

    pub fn add_wav_file(&mut self, path: &str) -> Result<()> {
        println!("Adding wav file: {}", path);

        // Read file into float buffer
        let file = crate::wav::file::File::from_wav(path)?;

        println!("{}: {}", "file.basename()", file.basename());
        println!("{}: {}", "file.nr_channels()", file.nr_channels());

        // For each channel create a model::track
        for (ix, ch) in file.buffer.borrow().channels().enumerate() {
            let name = format!("{} Channel {}", file.basename(), ix);
            let track = model::track::Track::new(Rc::clone(&file.buffer), ix, &name)?;
            self.tracks.push(track);
        }

        // Store file
        self.files.push(Rc::new(file));

        Ok(())
    }
}

impl Model {}
