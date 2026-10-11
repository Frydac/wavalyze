//! Diff orchestration on [`Model`]: integrating loaded/computed diffs into tracks, opening the
//! channel-pairing dialog, and spawning the diff jobs.
//!
//! Distinct from [`crate::model::jobs::diff`] (the background workers) and
//! [`crate::model::diff_pairing`] (the pending pairing dialog state).

use crate::audio;
use crate::model::{Model, jobs, track};
use crate::wav::{self, file::FileId};
use anyhow::Result;

impl Model {
    pub fn add_loaded_diff(&mut self, diff: jobs::LoadedDiff) -> Result<()> {
        // The diff job loads both source files (restricted to the channels referenced by the pairs)
        // and computes one diff per pair on a worker. Register both files without auto-creating
        // tracks, then walk the pairs in order appending `source_a, source_b, diff` per pair. A
        // source track is created only on first appearance, so a channel reused across pairs shows
        // its source once. Result order: a0, b0, diff0, a1, b1, diff1, ...
        let file_a_id = self.register_loaded_file(diff.file_a)?;
        let file_b_id = self.register_loaded_file(diff.file_b)?;

        for pair in diff.pairs {
            let buffer_id_a = self.channel_buffer_id(file_a_id, pair.ch_a)?;
            let buffer_id_b = self.channel_buffer_id(file_b_id, pair.ch_b)?;
            let sample_rate = self.audio.get_buffer(buffer_id_a)?.sample_rate();
            anyhow::ensure!(
                sample_rate == self.audio.get_buffer(buffer_id_b)?.sample_rate(),
                "diff source sample rates differ"
            );

            self.ensure_source_track(buffer_id_a, sample_rate, diff.sample_ix_offset_a)?;
            self.ensure_source_track(buffer_id_b, sample_rate, diff.sample_ix_offset_b)?;

            let buffer_id_diff = self
                .audio
                .buffers
                .insert(std::sync::Arc::new(pair.diff_buffer));
            self.audio
                .thumbnails
                .insert(buffer_id_diff, pair.diff_thumbnail);
            self.tracks.add_diff_track_to_end(
                track::diff::Diff {
                    buffer_id_diff,
                    buffer_id_a,
                    buffer_id_b,
                    sample_ix_offset_a: diff.sample_ix_offset_a,
                    sample_ix_offset_b: diff.sample_ix_offset_b,
                    sample_ix_offset_diff: pair.sample_ix_offset_diff,
                },
                sample_rate,
                &self.user_config.track,
            )?;
        }

        Ok(())
    }

    /// Append a source track for `buffer_id` if one does not already exist (a channel may appear in
    /// multiple diff pairs, but only one track per buffer is allowed).
    fn ensure_source_track(
        &mut self,
        buffer_id: audio::BufferId,
        sample_rate: u32,
        sample_ix_offset: audio::sample::Ix,
    ) -> Result<()> {
        if self.tracks.find_track(buffer_id).is_some() {
            return Ok(());
        }
        let track_id =
            self.tracks
                .add_track_to_end(buffer_id, sample_rate, &self.user_config.track)?;
        if let Some(track) = self.tracks.tracks.get_mut(track_id) {
            track.single.sample_ix_offset = sample_ix_offset as f64;
            track.use_file_offset = true;
        }
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
        // Insert directly after the dropped-on track if it still exists, else append to the end.
        let insert_ix = diff
            .insert_after
            .and_then(|id| self.tracks.tracks_order.iter().position(|t| *t == id))
            .map(|pos| pos + 1)
            .unwrap_or(self.tracks.tracks_order.len());
        self.tracks.insert_diff_track(
            track::diff::Diff {
                buffer_id_diff,
                buffer_id_a: diff.buffer_id_a,
                buffer_id_b: diff.buffer_id_b,
                sample_ix_offset_a: diff.sample_ix_offset_a,
                sample_ix_offset_b: diff.sample_ix_offset_b,
                sample_ix_offset_diff: diff.sample_ix_offset_diff,
            },
            sample_rate,
            insert_ix,
            &self.user_config.track,
        )?;

        Ok(())
    }

    fn channel_buffer_id(
        &self,
        file_id: FileId,
        ch_ix: wav::read::ChIx,
    ) -> Result<audio::BufferId> {
        let file = self
            .files
            .get(file_id)
            .ok_or_else(|| anyhow::anyhow!("File {:?} not found", file_id))?;
        file.channels
            .get(&ch_ix)
            .map(|channel| channel.buffer_id)
            .ok_or_else(|| anyhow::anyhow!("file {:?} has no channel {ch_ix}", file_id))
    }

    /// Open the channel-pairing matrix dialog for diffing two files. Channel counts and masks are peeked
    /// from the WAV headers without loading samples.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_diff_pairing_dialog(
        &mut self,
        file_a: wav::ReadConfig,
        file_b: wav::ReadConfig,
    ) -> Result<()> {
        use anyhow::Context;
        let metadata_a =
            wav::peek_channel_metadata(&file_a.filepath).context("invalid first diff input")?;
        let metadata_b =
            wav::peek_channel_metadata(&file_b.filepath).context("invalid second diff input")?;
        let effective_ch_ixs = |config: &wav::ReadConfig,
                                metadata: wav::WavChannelMetadata|
         -> Result<Vec<wav::read::ChIx>> {
            let channels = config
                .ch_ixs
                .clone()
                .unwrap_or_else(|| (0..metadata.total_nr_channels).collect());
            anyhow::ensure!(
                channels.iter().all(|ix| *ix < metadata.total_nr_channels),
                "selected channel is outside the file's {} channels",
                metadata.total_nr_channels
            );
            Ok(channels)
        };
        let ch_ixs_a = effective_ch_ixs(&file_a, metadata_a).context("invalid first diff input")?;
        let ch_ixs_b =
            effective_ch_ixs(&file_b, metadata_b).context("invalid second diff input")?;
        self.pending_diff_pairing = Some(crate::model::diff_pairing::PendingDiffPairing::new(
            file_a, file_b, metadata_a, metadata_b, ch_ixs_a, ch_ixs_b,
        ));
        Ok(())
    }

    /// Spawn a worker that loads both files (restricted to the channels referenced by `pairs`) and
    /// computes one diff per pair, integrated together via `Action::IntegrateLoadedDiff`. A single
    /// pair is the single-channel diff special case. No-op on an empty selection.
    pub fn start_diff_pairs(
        &mut self,
        file_a: wav::ReadConfig,
        file_b: wav::ReadConfig,
        pairs: Vec<(wav::read::ChIx, wav::read::ChIx)>,
        offset_detection_a: Option<jobs::OffsetDetectionMode>,
        offset_detection_b: Option<jobs::OffsetDetectionMode>,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if pairs.is_empty() {
                return;
            }
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
                jobs::diff::LoadDiffPathsJobInput {
                    generation,
                    file_a,
                    file_b,
                    pairs,
                    offset_detection_a,
                    offset_detection_b,
                },
                self.job_mgr.sender(),
                self.actions_tx.clone(),
            );
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (
                file_a,
                file_b,
                pairs,
                offset_detection_a,
                offset_detection_b,
            );
            tracing::warn!("start_diff_pairs ignored on wasm");
        }
    }

    pub fn start_diff_buffers_job(
        &mut self,
        buffer_id_a: audio::BufferId,
        buffer_id_b: audio::BufferId,
        sample_ix_offset_a: audio::sample::Ix,
        sample_ix_offset_b: audio::sample::Ix,
        insert_after: Option<track::TrackId>,
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
                insert_after,
            },
            self.job_mgr.sender(),
            self.actions_tx.clone(),
        );
        Ok(job_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{self, thumbnail::ThumbnailE};
    use crate::model::Action;
    use crate::model::test_support::{
        loaded_file_with_buffers, loaded_file_with_one_buffer, write_test_wav,
    };

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
                pairs: vec![jobs::LoadedDiffPair {
                    ch_a: 0,
                    ch_b: 0,
                    sample_ix_offset_diff: 0,
                    diff_buffer,
                    diff_thumbnail,
                }],
            })
            .unwrap();

        assert_eq!(model.tracks.tracks_order.len(), 3);
        let source_a = model
            .tracks
            .get_track(model.tracks.tracks_order[0])
            .unwrap();
        let source_b = model
            .tracks
            .get_track(model.tracks.tracks_order[1])
            .unwrap();
        let diff_track = model
            .tracks
            .get_track(*model.tracks.tracks_order.last().unwrap())
            .unwrap();
        assert!(source_a.use_file_offset);
        assert!(source_b.use_file_offset);
        assert!(diff_track.diff.is_some());
        assert!(!diff_track.use_file_offset);
        assert_eq!(model.files.order().len(), 2);
    }

    #[test]
    fn add_diff_buffer_inserts_after_target_track() {
        let mut model = Model::default();
        let buf = || audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(48_000, 32, 4));
        let id0 = model.audio.buffers.insert(std::sync::Arc::new(buf()));
        let id1 = model.audio.buffers.insert(std::sync::Arc::new(buf()));
        let id2 = model.audio.buffers.insert(std::sync::Arc::new(buf()));
        let cfg = model.user_config.track.clone();
        let t0 = model.tracks.add_track_to_end(id0, 48_000, &cfg).unwrap();
        let t1 = model.tracks.add_track_to_end(id1, 48_000, &cfg).unwrap();
        let t2 = model.tracks.add_track_to_end(id2, 48_000, &cfg).unwrap();

        let computed = |insert_after| {
            let diff_buffer = buf();
            let diff_thumbnail = ThumbnailE::from_buffer_e(&diff_buffer, None);
            jobs::ComputedDiff {
                buffer_id_a: id0,
                buffer_id_b: id2,
                sample_ix_offset_a: 0,
                sample_ix_offset_b: 0,
                sample_ix_offset_diff: 0,
                diff_buffer,
                diff_thumbnail,
                insert_after,
            }
        };

        // Insert directly after the middle track.
        model.add_diff_buffer(computed(Some(t1))).unwrap();
        let diff_id = model.tracks.tracks_order[2];
        assert_eq!(model.tracks.tracks_order, vec![t0, t1, diff_id, t2]);
        assert!(model.tracks.get_track(diff_id).unwrap().diff.is_some());

        // `None` appends to the end.
        model.add_diff_buffer(computed(None)).unwrap();
        let last = *model.tracks.tracks_order.last().unwrap();
        assert_eq!(model.tracks.tracks_order.len(), 5);
        assert!(model.tracks.get_track(last).unwrap().diff.is_some());
    }

    #[test]
    fn add_loaded_diff_interleaves_pairs_and_skips_unselected_channels() {
        let mut model = Model::default();
        let buf = || audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(48_000, 32, 4));
        let diff_a = buf();
        let diff_b = buf();

        model
            .add_loaded_diff(jobs::LoadedDiff {
                // file_a has channels 0,1,2 loaded; file_b has 0,1,2 loaded.
                file_a: loaded_file_with_buffers(&[buf(), buf(), buf()], 0),
                file_b: loaded_file_with_buffers(&[buf(), buf(), buf()], 0),
                sample_ix_offset_a: 0,
                sample_ix_offset_b: 0,
                pairs: vec![
                    jobs::LoadedDiffPair {
                        ch_a: 0,
                        ch_b: 0,
                        sample_ix_offset_diff: 0,
                        diff_buffer: diff_a,
                        diff_thumbnail: ThumbnailE::from_buffer_e(&buf(), None),
                    },
                    jobs::LoadedDiffPair {
                        ch_a: 1,
                        ch_b: 2,
                        sample_ix_offset_diff: 0,
                        diff_buffer: diff_b,
                        diff_thumbnail: ThumbnailE::from_buffer_e(&buf(), None),
                    },
                ],
            })
            .unwrap();

        // Order: a0, b0, diff0, a1, b2, diff1 — source channels not in any pair (a2, b1) get no
        // track.
        let order: Vec<bool> = model
            .tracks
            .tracks_order
            .iter()
            .map(|id| model.tracks.get_track(*id).unwrap().diff.is_some())
            .collect();
        assert_eq!(
            order,
            vec![false, false, true, false, false, true],
            "expected a0, b0, diff, a1, b2, diff"
        );
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
                pairs: vec![jobs::LoadedDiffPair {
                    ch_a: 0,
                    ch_b: 0,
                    sample_ix_offset_diff: 0,
                    diff_buffer,
                    diff_thumbnail,
                }],
            },
        }
        .process(&mut model)
        .unwrap();

        assert!(model.files.is_empty());
        assert!(model.tracks.tracks.is_empty());
        assert!(model.audio.buffers.is_empty());
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn open_diff_multichannel_files_opens_pairing_dialog() {
        let path_a = write_test_wav("pairing_a_2ch.wav", 2);
        let path_b = write_test_wav("pairing_b_3ch.wav", 3);
        let mut model = Model::new();

        Action::OpenDiffFilePaths {
            file_a: wav::ReadConfig::new(path_a),
            file_b: wav::ReadConfig::new(path_b),
        }
        .process(&mut model)
        .unwrap();

        let pending = model.pending_diff_pairing.as_ref().unwrap();
        assert_eq!(pending.ch_ixs_a, vec![0, 1]);
        assert_eq!(pending.ch_ixs_b, vec![0, 1, 2]);
        assert_eq!(pending.selected_pairs(), vec![(0, 0), (1, 1)]);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn open_diff_single_channel_inputs_bypasses_pairing_dialog() {
        let mono = write_test_wav("pairing_mono.wav", 1);
        let stereo = write_test_wav("pairing_stereo.wav", 2);
        let mut model = Model::new();

        Action::OpenDiffFilePaths {
            file_a: wav::ReadConfig::new(mono),
            file_b: wav::ReadConfig::new(stereo).with_ch_ixs([1]),
        }
        .process(&mut model)
        .unwrap();

        assert!(model.pending_diff_pairing.is_none());
        assert_eq!(model.job_mgr.pending(), 1);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn confirm_and_cancel_clear_pending_diff_pairing() {
        let path_a = write_test_wav("pairing_confirm_a.wav", 2);
        let path_b = write_test_wav("pairing_confirm_b.wav", 2);
        let mut model = Model::new();
        model
            .open_diff_pairing_dialog(wav::ReadConfig::new(&path_a), wav::ReadConfig::new(&path_b))
            .unwrap();
        assert!(model.pending_diff_pairing.is_some());

        Action::ConfirmDiffPairing.process(&mut model).unwrap();
        assert!(model.pending_diff_pairing.is_none());

        model
            .open_diff_pairing_dialog(wav::ReadConfig::new(&path_a), wav::ReadConfig::new(&path_b))
            .unwrap();
        Action::CancelDiffPairing.process(&mut model).unwrap();
        assert!(model.pending_diff_pairing.is_none());
    }
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn diff_dialog_presets_survive_confirmation_and_reload() {
        use std::time::Duration;
        use wav::channel_interpretation::{ChannelInterpretation, ChannelOrderPreset};
        let bytes = wav::channel_mask::tests::wav_bytes;
        for preset_b in [
            ChannelOrderPreset::Automatic,
            ChannelOrderPreset::AuroIdOrder,
            ChannelOrderPreset::Unassigned,
        ] {
            let path_a = write_test_wav(&format!("preset_a_{preset_b:?}.wav"), 3);
            let path_b = write_test_wav(&format!("preset_b_{preset_b:?}.wav"), 6);
            std::fs::write(&path_a, bytes(None, 3, &[])).unwrap();
            std::fs::write(&path_b, bytes(Some(0x3f), 6, &[])).unwrap();
            let mut model = Model::new();
            model.files.changes.disable_watcher();
            model
                .open_diff_pairing_dialog(
                    wav::ReadConfig::new(&path_a).with_ch_ixs([0, 2]),
                    wav::ReadConfig::new(&path_b).with_ch_ixs([4, 5]),
                )
                .unwrap();
            let pending = model.pending_diff_pairing.as_mut().unwrap();
            assert_eq!(
                pending.metadata_a,
                wav::WavChannelMetadata {
                    total_nr_channels: 3,
                    channel_mask: None
                }
            );
            assert_eq!(
                pending.metadata_b,
                wav::WavChannelMetadata {
                    total_nr_channels: 6,
                    channel_mask: Some(0x3f)
                }
            );
            assert_eq!(pending.ch_ixs_a, [0, 2]);
            assert_eq!(pending.ch_ixs_b, [4, 5]);
            let interpretation_a = ChannelInterpretation {
                preset: ChannelOrderPreset::Automatic,
                active_layout: Some(audio::Layout::LAYOUT_1_0),
            };
            let interpretation_b = ChannelInterpretation {
                preset: preset_b,
                active_layout: (preset_b == ChannelOrderPreset::AuroIdOrder)
                    .then_some(audio::Layout::LAYOUT_5_1),
            };
            pending.clear();
            pending.checked[1][0] = true;
            pending.file_a.channel_interpretation = interpretation_a.clone();
            pending.file_b.channel_interpretation = interpretation_b.clone();
            assert_eq!(pending.selected_pairs(), [(2, 4)]);
            Action::ConfirmDiffPairing.process(&mut model).unwrap();
            let action = model
                .actions_rx
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            action.process(&mut model).unwrap();
            model.drain_job_events();
            assert!(model.pending_diff_pairing.is_none());
            assert_eq!(model.files.len(), 2);
            let id_a = model.files.order()[0];
            let id_b = model.files.order()[1];
            let file_a = &model.files[id_a];
            let file_b = &model.files[id_b];
            assert_eq!(file_a.channel_interpretation, interpretation_a);
            assert_eq!(file_b.channel_interpretation, interpretation_b);
            assert_eq!(file_a.total_nr_channels, 3);
            assert_eq!(file_b.total_nr_channels, 6);
            assert_eq!(file_a.channels.keys().copied().collect::<Vec<_>>(), [2]);
            assert_eq!(file_b.channels.keys().copied().collect::<Vec<_>>(), [4]);
            assert_eq!(file_a.channels[&2].channel_id, Some(audio::Id::Center));
            let expected_b = match preset_b {
                ChannelOrderPreset::Automatic => Some(audio::Id::LeftBack),
                ChannelOrderPreset::AuroIdOrder => Some(audio::Id::LeftSurround),
                ChannelOrderPreset::Unassigned => None,
            };
            assert_eq!(file_b.channels[&4].channel_id, expected_b);
            let diff = model
                .tracks
                .tracks_order
                .iter()
                .find_map(|id| model.tracks.get_track(*id).unwrap().diff.as_ref())
                .unwrap();
            assert_eq!(diff.buffer_id_a, file_a.channels[&2].buffer_id);
            assert_eq!(diff.buffer_id_b, file_b.channels[&4].buffer_id);
            match model.audio.get_buffer(diff.buffer_id_diff).unwrap() {
                audio::buffer::BufferE::I16(buffer) => assert_eq!(buffer.data, [-2, -5]),
                _ => panic!("expected PCM16 diff"),
            }

            for id in [id_a, id_b] {
                assert_eq!(
                    model.files[id]
                        .source
                        .as_ref()
                        .unwrap()
                        .channel_interpretation,
                    model.files[id].channel_interpretation
                );
            }
            let order = model.tracks.tracks_order.clone();
            let track_b = model
                .find_track_id_for_buffer(file_b.channels[&4].buffer_id)
                .unwrap();
            // Reload B with an unspecified mask. Explicit choices remain; Automatic re-resolves.
            std::fs::write(&path_b, bytes(Some(0), 6, &[])).unwrap();
            let path = crate::model::files::disk::absolute_path(
                &model.files[id_b].source.as_ref().unwrap().filepath,
            );
            let change = model.files.changes.paths.get_mut(&path).unwrap();
            change.changed = true;
            change.revision += 1;
            change.last_event_time = None;
            Action::ReloadFiles(vec![id_b]).process(&mut model).unwrap();
            model
                .actions_rx
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .process(&mut model)
                .unwrap();
            model.drain_job_events();
            assert!(model.files.reload.active_job.is_none());
            assert_eq!(model.files[id_a].channel_interpretation, interpretation_a);
            assert_eq!(model.files[id_b].channel_interpretation, interpretation_b);
            assert_eq!(model.files[id_b].channel_mask, Some(0));
            assert_eq!(
                model.files[id_b].channels[&4].channel_id,
                (preset_b != ChannelOrderPreset::Unassigned).then_some(audio::Id::LeftSurround)
            );
            assert_eq!(
                model.find_track_id_for_buffer(model.files[id_b].channels[&4].buffer_id),
                Some(track_b)
            );
            assert_eq!(model.tracks.tracks_order, order);
            assert_eq!(
                model.files[id_b]
                    .source
                    .as_ref()
                    .unwrap()
                    .channel_interpretation,
                interpretation_b
            );
        }
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn diff_dialog_rejects_selected_indices_outside_the_peeked_header() {
        let a = write_test_wav("preset_invalid_a.wav", 3);
        let b = write_test_wav("preset_invalid_b.wav", 3);
        let mut model = Model::new();
        let error = model
            .open_diff_pairing_dialog(
                wav::ReadConfig::new(a).with_ch_ixs([3]),
                wav::ReadConfig::new(b),
            )
            .unwrap_err();
        assert!(format!("{error:#}").contains("selected channel is outside"));
        assert!(model.pending_diff_pairing.is_none());
    }
}
