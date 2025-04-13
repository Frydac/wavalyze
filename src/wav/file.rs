use crate::audio;
use crate::wav;
use anyhow::{Context, Result};
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

/// Represents a wav File
/// * buffer
/// * file metadata
/// * maybe later:
///   * load in chunks
///   * request part of file
///   * async loading
///   * stream data from disk if too large
///
///   TODO: maybe don't use Rc<RefCell<Buffer<f32>>>, maybe also try with id's?
///   * like some kind of storage for the buffers, and the file objects also use id's to refer to
///     them?
#[derive(Debug)]
pub struct File {
    pub buffer: Rc<RefCell<audio::Buffer<f32>>>, //audio::Buffer<f32>,
    pub file_path: String,
}

fn make_absolute(path_str: &str) -> Result<PathBuf> {
    let path = Path::new(path_str);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let cwd = std::env::current_dir()?;
        Ok(cwd.join(path).canonicalize()?)
    }
}

impl File {
    pub fn from_wav(path: &str) -> Result<File> {
        let full_path = make_absolute(path)?.to_str().unwrap().to_string();
        dbg!(&full_path);
        let buffer = wav::read::read_wav_file_to_float(&full_path).context(format!("Failed to read wav file '{}'", full_path))?;
        Ok(File {
            buffer: Rc::new(RefCell::new(buffer)), //buffer,
            file_path: full_path,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.buffer.borrow().sample_rate
    }
    pub fn nr_channels(&self) -> usize {
        self.buffer.borrow().nr_channels()
    }
    pub fn nr_samples(&self) -> usize {
        self.buffer.borrow().nr_samples()
    }
    pub fn sample_type(&self) -> audio::sample::SampleType {
        // TODO: why clone here necessary? Because not primitive? doesn't implment Copy trait?
        self.buffer.borrow().sample_type.clone()
    }

    pub fn basename(&self) -> String {
        Path::new(&self.file_path).file_name().unwrap().to_str().unwrap().to_string()
    }
}

impl Deref for File {
    type Target = Rc<RefCell<audio::Buffer<f32>>>; //audio::Buffer<f32>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl DerefMut for File {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

mod tests {
    #[test]
    fn test_wav_file() {
        let file = crate::wav::file::File::from_wav("test.wav").unwrap();
        println!("{:#?}", file);
    }
}
