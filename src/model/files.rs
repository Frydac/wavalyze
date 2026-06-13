//! File, channel, and track-presence operations on [`Model`]: resolving buffers to their
//! file/channel, per-file visibility state, and removing/restoring channel tracks.

use crate::audio;
use crate::model::{Model, track::TrackId};
use crate::wav::{self, file2::FileId};
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
    ) -> Option<(&wav::file2::File, &wav::file2::Channel)> {
        let track = self.tracks.get_track(track_id)?;
        self.get_file_channel_for_buffer(track.single.buffer_id)
    }

    /// Resolve a buffer to the file/channel it belongs to. Used by diff tracks, whose own
    /// `single.buffer_id` is the computed diff buffer (not in any file), to describe their sources.
    pub fn get_file_channel_for_buffer(
        &self,
        buffer_id: audio::BufferId,
    ) -> Option<(&wav::file2::File, &wav::file2::Channel)> {
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

    pub fn file_visibility_state(&self, file: &wav::file2::File) -> FileVisibilityState {
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

    pub fn set_file_visible(&mut self, file: &wav::file2::File, visible: bool) {
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
        let sample_rate = self.audio.get_buffer(buffer_id)?.sample_rate();
        let track_id =
            self.tracks
                .insert_track(buffer_id, sample_rate, insert_ix, &self.user_config.track)?;
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
    use crate::model::track;

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
        assert!(model.restore_channel_track(buffers[0]).unwrap());
        let restored_track_id = model.find_track_id_for_buffer(buffers[0]).unwrap();

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
