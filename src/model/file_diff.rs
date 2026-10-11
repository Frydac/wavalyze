//! Pair loaded files using buffer snapshots, then integrate and group their tracks together.

use std::{
    collections::{BTreeMap, HashSet},
    sync::Arc,
};

use anyhow::{Context, Result};

use super::{Model, diff_pairing::PendingDiffPairing, jobs, track};
use crate::{
    audio::{buffer::BufferE, sample},
    wav::{
        self,
        channel_interpretation::ChannelInterpretation,
        file::{File, FileId},
    },
};

#[derive(Debug, Clone)]
pub(crate) struct FileDiffSource {
    pub id: FileId,
    pub file: File,
}

#[derive(Debug)]
pub(crate) struct FileDiffRequest {
    pub sources: [FileDiffSource; 2],
    pub interpretations: [ChannelInterpretation; 2],
    pub offsets: [sample::Ix; 2],
    pub detection: [Option<jobs::OffsetDetectionMode>; 2],
    pub pairs: Vec<(wav::read::ChIx, wav::read::ChIx)>,
}

pub(crate) struct FileDiffJobInput {
    pub request: FileDiffRequest,
    pub buffers: [BTreeMap<wav::read::ChIx, Arc<BufferE>>; 2],
}

#[derive(Debug)]
pub struct FileDiffResult {
    pub(crate) request: FileDiffRequest,
    pub(crate) offsets: [sample::Ix; 2],
    pub(crate) pairs: Vec<jobs::LoadedDiffPair>,
}

impl Model {
    pub(crate) fn start_dropped_file_diff(
        &mut self,
        target: FileId,
        input: jobs::load_wav::WavLoadInput,
    ) -> Result<()> {
        anyhow::ensure!(
            self.files.contains_key(target),
            "drop target is no longer open"
        );
        #[cfg(not(target_arch = "wasm32"))]
        if let jobs::load_wav::WavLoadInput::Path(config) = &input {
            self.files.watch_source(&config.filepath);
        }
        let job_id = self
            .job_mgr
            .start_job(jobs::JobKind::LoadWav, "Load dropped WAV for diff");
        jobs::load_wav::spawn_load_file_for_diff(
            job_id,
            self.generation(),
            target,
            input,
            self.job_mgr.sender(),
            self.actions_tx.clone(),
        );
        Ok(())
    }

    pub(crate) fn integrate_dropped_file_for_diff(
        &mut self,
        target: FileId,
        loaded: wav::read::LoadedFile,
    ) -> Result<()> {
        let file = self
            .files
            .get(target)
            .context("drop target is no longer open")?;
        anyhow::ensure!(
            file.sample_rate == loaded.sample_rate,
            "diff inputs must have the same sample rate"
        );
        anyhow::ensure!(
            !file.channels.is_empty() && !loaded.channels.is_empty(),
            "both files need loaded channels"
        );
        let source = self.add_loaded_file(loaded)?;
        self.open_existing_file_pairing(source, target)
    }

    pub(crate) fn open_existing_file_pairing(&mut self, a: FileId, b: FileId) -> Result<()> {
        anyhow::ensure!(a != b, "choose two different files");
        let sources = [a, b].map(|id| {
            self.files
                .get(id)
                .cloned()
                .map(|file| FileDiffSource { id, file })
                .context("file is no longer open")
        });
        let [a, b] = sources;
        let sources = [a?, b?];
        anyhow::ensure!(
            sources
                .iter()
                .all(|source| !source.file.channels.is_empty()),
            "both files need loaded channels"
        );
        anyhow::ensure!(
            sources[0].file.sample_rate == sources[1].file.sample_rate,
            "diff inputs must have the same sample rate"
        );
        let configs = sources.each_ref().map(|source| {
            wav::ReadConfig::new(source.file.path.clone().unwrap_or_else(|| "Demo".into()))
                .with_ch_ixs(source.file.channels.keys().copied().collect::<Vec<_>>())
                .with_sample_ix_offset(source.file.sample_ix_offset)
                .with_channel_interpretation(source.file.channel_interpretation.clone())
        });
        let metadata = sources.each_ref().map(|source| wav::WavChannelMetadata {
            total_nr_channels: source.file.total_nr_channels,
            channel_mask: source.file.channel_mask,
        });
        let indices = sources
            .each_ref()
            .map(|source| source.file.channels.keys().copied().collect());
        let [a, b] = configs;
        let [indices_a, indices_b] = indices;
        let mut pending =
            PendingDiffPairing::new(a, b, metadata[0], metadata[1], indices_a, indices_b);
        pending.existing_files = Some(sources);
        self.pending_diff_pairing = Some(pending);
        Ok(())
    }

    fn validate_file_diff_sources(&self, sources: &[FileDiffSource; 2]) -> Result<()> {
        for source in sources {
            anyhow::ensure!(
                self.files.get(source.id) == Some(&source.file),
                "a diff input changed or closed; reopen channel pairing and retry"
            );
        }
        Ok(())
    }

    fn prepare_existing_file_diff(
        &self,
        pending: PendingDiffPairing,
    ) -> Result<Option<FileDiffJobInput>> {
        let pairs = pending.selected_pairs();
        if pairs.is_empty() {
            return Ok(None);
        }
        let sources = pending
            .existing_files
            .context("loaded-file sources missing")?;
        self.validate_file_diff_sources(&sources)?;
        let mut buffers = [BTreeMap::new(), BTreeMap::new()];
        for &(a, b) in &pairs {
            for (side, index) in [a, b].into_iter().enumerate() {
                let channel = sources[side]
                    .file
                    .channels
                    .get(&index)
                    .context("selected channel is not loaded")?;
                buffers[side].insert(index, self.audio.buffer_arc(channel.buffer_id)?);
            }
        }
        let request = FileDiffRequest {
            sources,
            interpretations: [
                pending.file_a.channel_interpretation,
                pending.file_b.channel_interpretation,
            ],
            offsets: [
                pending.file_a.sample_ix_offset,
                pending.file_b.sample_ix_offset,
            ],
            detection: [pending.offset_detection_a, pending.offset_detection_b],
            pairs,
        };
        Ok(Some(FileDiffJobInput { request, buffers }))
    }

    pub(crate) fn start_existing_file_diff(&mut self, pending: PendingDiffPairing) -> Result<()> {
        let Some(input) = self.prepare_existing_file_diff(pending)? else {
            return Ok(());
        };
        let job_id = self
            .job_mgr
            .start_job(jobs::JobKind::Diff, "Compare loaded files");
        jobs::diff::spawn_file_diff_job(
            job_id,
            self.generation(),
            input,
            self.job_mgr.sender(),
            self.actions_tx.clone(),
        );
        Ok(())
    }

    pub(crate) fn integrate_existing_file_diff(&mut self, result: FileDiffResult) -> Result<()> {
        self.validate_file_diff_sources(&result.request.sources)?;
        // Find the source group before creating tracks. Unrelated tracks retain their relative
        // order, and the new group occupies the first source track's original position.
        let old_order = self.tracks.tracks_order.clone();
        let buffers: HashSet<_> = result
            .request
            .sources
            .iter()
            .flat_map(|source| source.file.channels.values().map(|ch| ch.buffer_id))
            .collect();
        let affected: HashSet<_> = old_order
            .iter()
            .copied()
            .filter(|id| {
                self.tracks.get_track(*id).is_some_and(|track| {
                    track.diff.is_none() && buffers.contains(&track.single.buffer_id)
                })
            })
            .collect();
        let anchor = old_order
            .iter()
            .position(|id| affected.contains(id))
            .unwrap_or(old_order.len());
        let mut group = Vec::new();
        let mut grouped = HashSet::new();

        for (side, source) in result.request.sources.iter().enumerate() {
            self.files
                .get_mut(source.id)
                .unwrap()
                .set_channel_interpretation(result.request.interpretations[side].clone());
            self.set_file_sample_ix_offset(source.id, result.offsets[side]);
        }
        for pair in result.pairs {
            let mut source_buffers = Vec::new();
            for (side, index) in [pair.ch_a, pair.ch_b].into_iter().enumerate() {
                let source = &result.request.sources[side];
                let buffer_id = source.file.channels[&index].buffer_id;
                let track_id = if let Some(id) = self.find_track_id_for_buffer(buffer_id) {
                    id
                } else {
                    // A closed source track can be restored without loading its file again.
                    let id = self.tracks.add_track_to_end(
                        buffer_id,
                        source.file.sample_rate,
                        &self.user_config.track,
                    )?;
                    let track = self.tracks.get_track_mut(id).unwrap();
                    track.single.sample_ix_offset = result.offsets[side] as f64;
                    track.use_file_offset = true;
                    id
                };
                if grouped.insert(track_id) {
                    group.push(track_id);
                }
                source_buffers.push(buffer_id);
            }
            let buffer_id_diff = self.audio.buffers.insert(Arc::new(pair.diff_buffer));
            self.audio
                .thumbnails
                .insert(buffer_id_diff, pair.diff_thumbnail);
            let id = self.tracks.add_diff_track_to_end(
                track::diff::Diff {
                    buffer_id_diff,
                    buffer_id_a: source_buffers[0],
                    buffer_id_b: source_buffers[1],
                    sample_ix_offset_a: result.offsets[0],
                    sample_ix_offset_b: result.offsets[1],
                    sample_ix_offset_diff: pair.sample_ix_offset_diff,
                },
                result.request.sources[0].file.sample_rate,
                &self.user_config.track,
            )?;
            grouped.insert(id);
            group.push(id);
        }
        // Closing unpaired source tracks keeps their buffers available in Files and for
        // existing diff dependencies, while removing their presentation and track selection.
        for id in affected.iter().filter(|id| !grouped.contains(id)) {
            self.tracks.remove_track(*id);
        }
        let remaining: Vec<_> = old_order
            .iter()
            .copied()
            .filter(|id| !affected.contains(id))
            .collect();
        self.tracks.tracks_order = remaining[..anchor]
            .iter()
            .copied()
            .chain(group)
            .chain(remaining[anchor..].iter().copied())
            .collect();
        Ok(())
    }
}

#[cfg(test)]
#[path = "file_diff_tests.rs"]
mod tests;
