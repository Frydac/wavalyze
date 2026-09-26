//! Loaded file records, their sidebar order, and native source update state.
//!
//! `Files` keeps record insertion/removal and ordering together. It contains browser uploads
//! and generated audio too; only monitoring and reload state are native-only. Operations that
//! also change audio or tracks remain small `Model` coordinators in the child modules.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod disk;
mod loading;
#[cfg(not(target_arch = "wasm32"))]
pub mod monitoring;
mod operations;
#[cfg(not(target_arch = "wasm32"))]
pub mod reload;

use crate::{
    audio,
    wav::{self, file::FileId},
};
pub use operations::FileVisibilityState;
use slotmap::SlotMap;

/// Own file records and their display order so callers cannot update one without the other.
/// Native monitoring is shared by disk path; reload state tracks the single active update batch.
#[derive(Debug, Default)]
pub struct Files {
    entries: SlotMap<FileId, wav::file::File>,
    order: Vec<FileId>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) changes: monitoring::FileChanges,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) reload: reload::ReloadState,
}

impl Files {
    /// Register a file and append its stable ID to the sidebar order.
    pub fn insert(&mut self, file: wav::file::File) -> FileId {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(source) = &file.source {
            self.changes.watch(&source.filepath);
        }
        let id = self.entries.insert(file);
        self.order.push(id);
        id
    }

    /// Remove metadata and its order entry. The caller first removes associated tracks/audio.
    pub(crate) fn remove(&mut self, id: FileId) -> Option<wav::file::File> {
        let file = self.entries.remove(id)?;
        self.order.retain(|entry| *entry != id);
        Some(file)
    }

    /// Clear file records and ordering as part of clearing the surrounding workspace.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    /// Stable file IDs in their sidebar order; callers cannot mutate the order independently.
    pub fn order(&self) -> &[FileId] {
        &self.order
    }
    pub fn get(&self, id: FileId) -> Option<&wav::file::File> {
        self.entries.get(id)
    }
    pub fn get_mut(&mut self, id: FileId) -> Option<&mut wav::file::File> {
        self.entries.get_mut(id)
    }
    pub fn contains_key(&self, id: FileId) -> bool {
        self.entries.contains_key(id)
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn values(&self) -> impl Iterator<Item = &wav::file::File> {
        self.entries.values()
    }
    pub fn iter(&self) -> impl Iterator<Item = (FileId, &wav::file::File)> {
        self.entries.iter()
    }

    /// Resolve a buffer to the file/channel it belongs to. Used by diff tracks, whose own
    /// `single.buffer_id` is the computed diff buffer (not in any file), to describe their sources.
    pub fn channel_for_buffer(
        &self,
        buffer_id: audio::BufferId,
    ) -> Option<(&wav::file::File, &wav::file::Channel)> {
        for file in self.values() {
            if let Some(channel) = file.get_channel(buffer_id) {
                return Some((file, channel));
            }
        }
        None
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Files {
    /// Register a native source before its worker reads it, including pending file opens.
    pub(crate) fn watch_source(&mut self, path: &std::path::Path) -> std::path::PathBuf {
        self.changes.watch(path)
    }

    /// Disk change state shared by all entries opened from this file's path.
    pub(crate) fn change_for(&self, id: FileId) -> Option<&monitoring::Change> {
        let source = self.get(id)?.source.as_ref()?;
        self.changes
            .paths
            .get(&disk::absolute_path(&source.filepath))
    }

    /// Whether an update batch is still being prepared or integrated.
    pub(crate) fn reload_running(&self) -> bool {
        self.reload.active_job.is_some()
    }

    /// Watch setup/delivery failure for the Files sidebar, separate from reload errors.
    pub(crate) fn monitoring_error(&self) -> Option<&str> {
        self.changes.monitor_error.as_deref()
    }

    /// Record a successful update for one Files-list entry. Other entries from the same path
    /// remain pending until they have also applied this revision, possibly in a later batch.
    pub(crate) fn acknowledge_reload(&mut self, id: FileId, revision: u64) {
        let Some(source) = self.get(id).and_then(|file| file.source.as_ref()) else {
            return;
        };
        let path = disk::absolute_path(&source.filepath);
        self.changes.acknowledged.insert(id, revision);
        let all_updated = self
            .iter()
            .filter(|(_, file)| {
                file.source
                    .as_ref()
                    .is_some_and(|source| disk::absolute_path(&source.filepath) == path)
            })
            .all(|(id, _)| self.changes.acknowledged.get(&id).copied() == Some(revision));
        if all_updated && let Some(change) = self.changes.paths.get_mut(&path) {
            change.changed = false;
            change.error = None;
        }
    }

    /// Return Files-list entries that have not yet reloaded the latest observed disk change.
    pub fn changed_ids(&self) -> Vec<FileId> {
        self.order()
            .iter()
            .copied()
            .filter(|id| {
                self.change_for(*id).is_some_and(|change| {
                    change.changed
                        && self.changes.acknowledged.get(id).copied().unwrap_or(0) < change.revision
                })
            })
            .collect()
    }
    /// Process disk notifications, discard unused watches, and schedule a repaint after writes settle.
    /// Pending opens retain their watches because watching starts before the load worker reads.
    pub fn poll_changes(&mut self, ctx: &egui::Context, keep_pending_opens: bool) {
        self.changes.set_context(ctx);
        self.changes.drain();
        self.changes
            .acknowledged
            .retain(|id, _| self.entries.contains_key(*id));
        // Pending opens register before reading. Keep their watches until all open jobs finish.
        if !keep_pending_opens {
            let paths = self
                .values()
                .filter_map(|f| f.source.as_ref())
                .map(|s| crate::model::files::disk::absolute_path(&s.filepath))
                .collect();
            self.changes.retain(&paths);
        }
        if self.changes.paths.values().any(|c| c.changed && !c.ready()) {
            ctx.request_repaint_after(crate::model::files::monitoring::QUIET_PERIOD);
        }
    }
}

impl std::ops::Index<FileId> for Files {
    type Output = wav::file::File;
    fn index(&self, id: FileId) -> &Self::Output {
        &self.entries[id]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::test_support::make_file;

    #[test]
    fn replacing_a_removed_entry_keeps_new_file_at_end_and_rejects_old_id() {
        let mut files = Files::default();
        let first = files.insert(make_file(&[]));
        let middle = files.insert(make_file(&[]));
        let last = files.insert(make_file(&[]));
        assert!(files.remove(middle).is_some());
        let replacement = files.insert(make_file(&[]));
        assert_eq!(files.order(), &[first, last, replacement]);
        assert!(files.get(middle).is_none());
        assert!(files.remove(middle).is_none());
        assert_eq!(files.len(), files.order().len());
        files.clear();
        assert!(files.is_empty());
        assert!(files.order().is_empty());
    }
}
