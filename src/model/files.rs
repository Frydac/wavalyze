//! File, channel, and track-presence operations on [`Model`]: resolving buffers to their
//! file/channel, per-file visibility state, and removing/restoring channel tracks.

use crate::audio;
use crate::model::{Model, track::TrackId};
use crate::wav::{self, file::FileId};
use anyhow::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileVisibilityState {
    NoneVisible,
    PartiallyVisible,
    AllVisible,
}

impl Model {
    pub fn get_file_channel_for_track(
        &self,
        track_id: TrackId,
    ) -> Option<(&wav::file::File, &wav::file::Channel)> {
        let track = self.tracks.get_track(track_id)?;
        self.get_file_channel_for_buffer(track.single.buffer_id)
    }

    /// Resolve a buffer to the file/channel it belongs to. Used by diff tracks, whose own
    /// `single.buffer_id` is the computed diff buffer (not in any file), to describe their sources.
    pub fn get_file_channel_for_buffer(
        &self,
        buffer_id: audio::BufferId,
    ) -> Option<(&wav::file::File, &wav::file::Channel)> {
        for file in self.files.values() {
            if let Some(channel) = file.get_channel(buffer_id) {
                return Some((file, channel));
            }
        }
        None
    }

    pub fn find_track_id_for_buffer(&self, buffer_id: audio::BufferId) -> Option<TrackId> {
        self.tracks
            .find_track(buffer_id)
            .map(|(track_id, _)| track_id)
    }

    pub fn file_visibility_state(&self, file: &wav::file::File) -> FileVisibilityState {
        let mut any_visible = false;
        let mut any_hidden = false;

        for channel in file.channels.values() {
            match self.find_track_id_for_buffer(channel.buffer_id) {
                Some(track_id) => {
                    let is_visible = self
                        .tracks
                        .get_track(track_id)
                        .is_some_and(|track| track.visible);
                    if is_visible {
                        any_visible = true;
                    } else {
                        any_hidden = true;
                    }
                }
                None => {
                    any_hidden = true;
                }
            }
        }

        match (any_visible, any_hidden) {
            (true, true) => FileVisibilityState::PartiallyVisible,
            (true, false) => FileVisibilityState::AllVisible,
            _ => FileVisibilityState::NoneVisible,
        }
    }

    pub fn set_channel_visible(&mut self, buffer_id: audio::BufferId, visible: bool) -> bool {
        let Some(track_id) = self.find_track_id_for_buffer(buffer_id) else {
            return false;
        };
        self.tracks.set_track_visibility(track_id, visible);
        true
    }

    pub fn set_file_visible(&mut self, file: &wav::file::File, visible: bool) {
        for channel in file.channels.values() {
            self.set_channel_visible(channel.buffer_id, visible);
        }
    }

    pub fn file_visibility_state_for(&self, file_id: FileId) -> Option<FileVisibilityState> {
        let file = self.files.get(file_id)?;
        Some(self.file_visibility_state(file))
    }

    pub fn set_file_visible_for(&mut self, file_id: FileId, visible: bool) -> bool {
        let Some(file) = self.files.get(file_id).cloned() else {
            return false;
        };
        self.set_file_visible(&file, visible);
        true
    }

    /// Set a file's absolute sample offset and update every channel track still inheriting it.
    pub fn set_file_sample_ix_offset(
        &mut self,
        file_id: FileId,
        sample_ix_offset: audio::sample::Ix,
    ) -> bool {
        let Some(file) = self.files.get_mut(file_id) else {
            return false;
        };
        file.sample_ix_offset = sample_ix_offset;
        let buffer_ids: Vec<_> = file
            .channels
            .values()
            .map(|channel| channel.buffer_id)
            .collect();

        for buffer_id in buffer_ids {
            let Some(track_id) = self.find_track_id_for_buffer(buffer_id) else {
                continue;
            };
            let Some(track) = self.tracks.get_track_mut(track_id) else {
                continue;
            };
            if track.use_file_offset && track.single.sample_ix_offset != sample_ix_offset as f64 {
                track.single.sample_ix_offset = sample_ix_offset as f64;
                track.single.mark_dirty();
            }
        }
        true
    }

    /// Toggle file-offset inheritance for a file-backed track. Disabling preserves its current
    /// absolute value; enabling immediately adopts the source file's current value.
    pub fn set_track_use_file_offset(&mut self, track_id: TrackId, use_file_offset: bool) -> bool {
        let file_offset = self
            .get_file_channel_for_track(track_id)
            .map(|(file, _)| file.sample_ix_offset as f64);
        if use_file_offset && file_offset.is_none() {
            return false;
        }
        let Some(track) = self.tracks.get_track_mut(track_id) else {
            return false;
        };
        track.use_file_offset = use_file_offset;
        if let Some(file_offset) = file_offset
            && use_file_offset
            && track.single.sample_ix_offset != file_offset
        {
            track.single.sample_ix_offset = file_offset;
            track.single.mark_dirty();
        }
        true
    }

    /// Set a track's custom absolute offset. Inherited tracks must be detached first.
    pub fn set_track_sample_ix_offset(&mut self, track_id: TrackId, sample_ix_offset: f64) -> bool {
        let Some(track) = self.tracks.get_track_mut(track_id) else {
            return false;
        };
        if track.use_file_offset {
            return false;
        }
        if track.single.sample_ix_offset != sample_ix_offset {
            track.single.sample_ix_offset = sample_ix_offset;
            track.single.mark_dirty();
        }
        true
    }

    /// Unload an entire file: remove all of its channel tracks, drop the underlying audio
    /// buffers, and discard the file spec itself.
    pub fn remove_file(&mut self, file_id: FileId) -> bool {
        let Some(file) = self.files.get(file_id).cloned() else {
            return false;
        };
        for channel in file.channels.values() {
            if let Some(track_id) = self.find_track_id_for_buffer(channel.buffer_id) {
                self.tracks.remove_track(track_id);
            }
        }
        self.audio.remove_buffers_from_file(&file);
        self.files.remove(file_id);
        self.files_order.retain(|id| *id != file_id);
        true
    }

    pub fn remove_channel_track(&mut self, buffer_id: audio::BufferId) -> bool {
        let Some(track_id) = self.find_track_id_for_buffer(buffer_id) else {
            return false;
        };
        self.tracks.remove_track(track_id);
        true
    }

    pub fn restore_channel_track(&mut self, buffer_id: audio::BufferId) -> Result<bool> {
        if self.find_track_id_for_buffer(buffer_id).is_some() {
            return Ok(false);
        }
        let Some(insert_ix) = self.track_insert_index_for_buffer(buffer_id) else {
            return Ok(false);
        };
        let Some(file_offset) = self
            .get_file_channel_for_buffer(buffer_id)
            .map(|(file, _)| file.sample_ix_offset)
        else {
            return Ok(false);
        };
        let sample_rate = self.audio.get_buffer(buffer_id)?.sample_rate();
        let track_id =
            self.tracks
                .insert_track(buffer_id, sample_rate, insert_ix, &self.user_config.track)?;
        if let Some(track) = self.tracks.get_track_mut(track_id) {
            track.single.sample_ix_offset = file_offset as f64;
            track.use_file_offset = true;
        }
        self.tracks.set_track_height(
            track_id,
            crate::model::track::min_total_height(&self.user_config.track),
        );
        Ok(true)
    }

    fn track_insert_index_for_buffer(&self, buffer_id: audio::BufferId) -> Option<usize> {
        let mut insert_ix = 0;
        for file in self.files_order.iter().filter_map(|id| self.files.get(*id)) {
            for channel in file.channels.values() {
                if channel.buffer_id == buffer_id {
                    return Some(insert_ix);
                }
                if self.find_track_id_for_buffer(channel.buffer_id).is_some() {
                    insert_ix += 1;
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_support::{add_buffer, make_file};
    use crate::model::{Action, track};

    #[test]
    fn file_visibility_state_tracks_partial_visibility() {
        let mut model = Model::new();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        let file_id = model.insert_file(file);

        assert_eq!(
            model.file_visibility_state_for(file_id),
            Some(FileVisibilityState::AllVisible)
        );

        model.set_channel_visible(buffers[0], false);

        assert_eq!(
            model.file_visibility_state_for(file_id),
            Some(FileVisibilityState::PartiallyVisible)
        );

        model.set_file_visible_for(file_id, false);

        assert_eq!(
            model.file_visibility_state_for(file_id),
            Some(FileVisibilityState::NoneVisible)
        );
    }

    #[test]
    fn file_offset_updates_only_inherited_tracks() {
        let mut model = Model::new();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let mut file = make_file(&buffers);
        file.sample_ix_offset = 4;
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        let file_id = model.insert_file(file);
        let track_ids = buffers.map(|buffer_id| model.find_track_id_for_buffer(buffer_id).unwrap());

        Action::SetTrackUseFileOffset {
            track_id: track_ids[0],
            use_file_offset: false,
        }
        .process(&mut model)
        .unwrap();
        Action::SetTrackSampleIxOffset {
            track_id: track_ids[0],
            sample_ix_offset: 12.5,
        }
        .process(&mut model)
        .unwrap();
        Action::SetFileSampleIxOffset {
            file_id,
            sample_ix_offset: 9,
        }
        .process(&mut model)
        .unwrap();

        let custom = model.tracks.get_track(track_ids[0]).unwrap();
        let inherited = model.tracks.get_track(track_ids[1]).unwrap();
        assert!(!custom.use_file_offset);
        assert_eq!(custom.single.sample_ix_offset, 12.5);
        assert!(inherited.use_file_offset);
        assert_eq!(inherited.single.sample_ix_offset, 9.0);
    }

    #[test]
    fn toggling_file_offset_preserves_then_replaces_absolute_value() {
        let mut model = Model::new();
        let buffer = add_buffer(&mut model);
        let mut file = make_file(&[buffer]);
        file.sample_ix_offset = 3;
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        let file_id = model.insert_file(file);
        let track_id = model.find_track_id_for_buffer(buffer).unwrap();

        assert!(model.set_track_use_file_offset(track_id, false));
        assert_eq!(
            model
                .tracks
                .get_track(track_id)
                .unwrap()
                .single
                .sample_ix_offset,
            3.0
        );
        assert!(model.set_file_sample_ix_offset(file_id, 7));
        assert_eq!(
            model
                .tracks
                .get_track(track_id)
                .unwrap()
                .single
                .sample_ix_offset,
            3.0
        );
        assert!(model.set_track_use_file_offset(track_id, true));
        let track = model.tracks.get_track(track_id).unwrap();
        assert!(track.use_file_offset);
        assert_eq!(track.single.sample_ix_offset, 7.0);
    }

    #[test]
    fn inherited_track_rejects_custom_offset() {
        let mut model = Model::new();
        let buffer = add_buffer(&mut model);
        let file = make_file(&[buffer]);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        model.insert_file(file);
        let track_id = model.find_track_id_for_buffer(buffer).unwrap();

        assert!(!model.set_track_sample_ix_offset(track_id, 5.0));
        assert_eq!(
            model
                .tracks
                .get_track(track_id)
                .unwrap()
                .single
                .sample_ix_offset,
            0.0
        );
    }

    #[test]
    fn remove_channel_track_marks_channel_missing() {
        let mut model = Model::new();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        let file_id = model.insert_file(file.clone());

        assert!(model.remove_channel_track(buffers[0]));
        assert!(model.find_track_id_for_buffer(buffers[0]).is_none());
        assert_eq!(
            model.file_visibility_state_for(file_id),
            Some(FileVisibilityState::PartiallyVisible)
        );
        assert!(
            model
                .get_file_channel_for_track(model.find_track_id_for_buffer(buffers[1]).unwrap())
                .is_some()
        );
    }

    #[test]
    fn restore_channel_track_recreates_missing_track() {
        let mut model = Model::new();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        let file_id = model.insert_file(file);

        assert!(model.remove_channel_track(buffers[0]));
        assert!(model.set_file_sample_ix_offset(file_id, 11));
        assert!(model.restore_channel_track(buffers[0]).unwrap());
        let restored_track_id = model.find_track_id_for_buffer(buffers[0]).unwrap();
        let restored_track = model.tracks.get_track(restored_track_id).unwrap();

        assert!(restored_track.use_file_offset);
        assert_eq!(restored_track.single.sample_ix_offset, 11.0);
        assert_eq!(model.tracks.tracks_order.len(), 2);
        assert_eq!(model.tracks.tracks_order[0], restored_track_id);
        assert!(model.tracks.get_track(restored_track_id).unwrap().visible);
        assert_eq!(
            model.file_visibility_state_for(file_id),
            Some(FileVisibilityState::AllVisible)
        );
    }

    #[test]
    fn restore_channel_track_uses_minimum_track_height() {
        let mut model = Model::new();
        model.user_config.track.min_height = 42.0;
        let buffers = [add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        model.insert_file(file);

        let track_id = model.find_track_id_for_buffer(buffers[0]).unwrap();
        model.tracks.set_track_height(track_id, 120.0);

        assert!(model.remove_channel_track(buffers[0]));
        assert!(model.restore_channel_track(buffers[0]).unwrap());

        let restored_track_id = model.find_track_id_for_buffer(buffers[0]).unwrap();
        assert_eq!(
            model.tracks.get_track(restored_track_id).unwrap().height,
            42.0 + track::HEADER_HEIGHT
        );
    }

    #[test]
    fn restore_channel_track_preserves_file_channel_order() {
        let mut model = Model::new();
        let first_file_buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let second_file_buffers = [add_buffer(&mut model)];
        let first_file = make_file(&first_file_buffers);
        let second_file = make_file(&second_file_buffers);

        model
            .tracks
            .add_tracks_from_file(&first_file, &model.user_config.track)
            .unwrap();
        model
            .tracks
            .add_tracks_from_file(&second_file, &model.user_config.track)
            .unwrap();
        model.insert_file(first_file);
        model.insert_file(second_file);

        assert!(model.remove_channel_track(first_file_buffers[1]));
        assert!(model.restore_channel_track(first_file_buffers[1]).unwrap());

        let ordered_buffers: Vec<_> = model
            .tracks
            .tracks_order
            .iter()
            .map(|track_id| model.tracks.get_track(*track_id).unwrap().single.buffer_id)
            .collect();
        assert_eq!(
            ordered_buffers,
            vec![
                first_file_buffers[0],
                first_file_buffers[1],
                second_file_buffers[0]
            ]
        );
    }

    #[test]
    fn restore_channel_track_is_noop_when_track_already_loaded() {
        let mut model = Model::new();
        let buffers = [add_buffer(&mut model), add_buffer(&mut model)];
        let file = make_file(&buffers);
        model
            .tracks
            .add_tracks_from_file(&file, &model.user_config.track)
            .unwrap();
        model.insert_file(file);

        assert!(!model.restore_channel_track(buffers[0]).unwrap());
        assert_eq!(model.tracks.tracks_order.len(), 2);
    }
}
