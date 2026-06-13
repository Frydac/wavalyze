//! WAV loading and integration on [`Model`]: synchronous loads, integrating a decoded
//! [`wav::read::LoadedFile`] into buffers/tracks/files, and spawning background load jobs.

use crate::audio::thumbnail::ThumbnailE;
use crate::model::{Model, jobs};
use crate::wav::{self, file2::FileId};
use anyhow::Result;
use tracing::{info, trace};

impl Model {
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

    pub fn add_loaded_file(&mut self, loaded: wav::read::LoadedFile) -> Result<FileId> {
        let file_id = self.register_loaded_file(loaded)?;
        let file = self
            .files
            .get(file_id)
            .ok_or_else(|| anyhow::anyhow!("File {:?} not found", file_id))?
            .clone();
        self.tracks
            .add_tracks_from_file(&file, &self.user_config.track)?;
        Ok(file_id)
    }

    /// Insert a loaded file's buffers, thumbnails, and metadata into the model without creating any
    /// tracks. Callers that want the default one-track-per-channel layout use [`Self::add_loaded_file`];
    /// the diff path uses this to control track order itself.
    pub(crate) fn register_loaded_file(&mut self, loaded: wav::read::LoadedFile) -> Result<FileId> {
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

        Ok(self.insert_file(file))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio;
    use crate::model::Action;
    use crate::model::test_support::loaded_file_with_one_buffer;

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
}
