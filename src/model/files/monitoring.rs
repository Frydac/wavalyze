//! Track which disk sources may have changed without reloading audio automatically.
//!
//! OS callbacks enqueue events and wake egui.
//! The UI thread turns them into path revisions and pending notifications.
//! Non-recursive path ancestor watches survive deleting parent directories of watched paths.
//! The reload module uses these revisions to reject stale work and acknowledge successful updates
//! per loaded file.
//!
//! This module is native-only: browser uploads do not retain usable filesystem paths.
use super::disk::absolute_path;
use notify::{EventKind, RecursiveMode, Watcher};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

/// Wait after the latest event before offering reload, to coalesce a generator's write burst.
pub const QUIET_PERIOD: Duration = Duration::from_millis(300);

/// Pending disk state for one normalized path, shared by all Files-list entries opened from that
/// path.
/// Revisions count filesystem notifications, not content changes.
/// Reload separately checks whether the file changed during loading and rejects the result if it
/// did.
#[derive(Debug, Default)]
pub struct Change {
    /// Incremented by relevant events.
    pub revision: u64,
    /// Remains set until every instance of the laoded file has acknowledged the current revision.
    pub changed: bool,
    /// Timepoint of latest relevant event, used to wait for a quiet period before enabling update
    /// buttons.
    pub last_event_time: Option<Instant>,
    /// Last reload failure shown with the pending notification; a new event clears it.
    pub error: Option<String>,
}
impl Change {
    /// Whether the write burst has been quiet long enough to offer a reload.
    pub fn ready(&self) -> bool {
        self.last_event_time
            .is_none_or(|at| at.elapsed() >= QUIET_PERIOD)
    }
}

/// Detect changes to files on disk and track which entries in the Files list need reloading.
/// If the same disk file is opened twice, both entries receive change notifications from the
/// same watcher. Reloading one entry clears its notification; the other still needs reloading.
pub struct FileChanges {
    watcher: Option<notify::RecommendedWatcher>,
    rx: mpsc::Receiver<notify::Result<notify::Event>>,
    context: Arc<Mutex<Option<egui::Context>>>,
    directories: BTreeSet<PathBuf>,
    /// Source paths, including pending opens registered before decoding starts.
    pub paths: BTreeMap<PathBuf, Change>,
    /// Last successfully applied revision for each loaded file instance.
    pub acknowledged: HashMap<crate::wav::file::FileId, u64>,
    /// Watch setup or delivery failure, displayed separately from reload errors.
    pub monitor_error: Option<String>,
}
impl std::fmt::Debug for FileChanges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileChanges")
            .field("paths", &self.paths)
            .finish_non_exhaustive()
    }
}
impl Default for FileChanges {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let context: Arc<Mutex<Option<egui::Context>>> = Arc::default();
        let wake = context.clone();
        let result = notify::recommended_watcher(move |event| {
            let _ = tx.send(event);
            if let Some(ctx) = wake.lock().unwrap().as_ref() {
                ctx.request_repaint();
            }
        });
        let (watcher, monitor_error) = match result {
            Ok(watcher) => (Some(watcher), None),
            Err(error) => (None, Some(format!("File monitoring unavailable: {error}"))),
        };
        Self {
            acknowledged: HashMap::new(),
            watcher,
            rx,
            context,
            directories: BTreeSet::new(),
            paths: BTreeMap::new(),
            monitor_error,
        }
    }
}

impl FileChanges {
    #[cfg(test)]
    pub(crate) fn disable_watcher(&mut self) {
        self.watcher = None;
    }
    /// Attach the repaint handle so filesystem events also wake an idle window.
    pub fn set_context(&mut self, ctx: &egui::Context) {
        *self.context.lock().unwrap() = Some(ctx.clone());
    }
    /// Register a source before reading it and return the normalized path used for filesystem event
    /// matching.
    pub fn watch(&mut self, path: &Path) -> PathBuf {
        let path = absolute_path(path);
        self.paths.entry(path.clone()).or_default();
        self.restore_directory_watches();
        path
    }
    /// Apply queued events on the UI thread, then restore any invalidated directory watches.
    pub fn drain(&mut self) {
        let mut had_events = false;
        while let Ok(event) = self.rx.try_recv() {
            had_events = true;
            match event {
                Ok(event) => self.record_event(event, Instant::now()),
                Err(error) => self.monitor_error = Some(format!("File monitoring failed: {error}")),
            }
        }
        if had_events {
            self.restore_directory_watches();
        }
    }
    fn record_event(&mut self, event: notify::Event, now: Instant) {
        let rescan = event.need_rescan();
        if !rescan && matches!(event.kind, EventKind::Access(_) | EventKind::Other) {
            return;
        }
        let structural = rescan
            || matches!(
                event.kind,
                EventKind::Create(_)
                    | EventKind::Remove(_)
                    | EventKind::Modify(notify::event::ModifyKind::Name(_))
                    | EventKind::Any
            );
        let event_paths: Vec<_> = event.paths.iter().map(|path| absolute_path(path)).collect();
        for (path, change) in &mut self.paths {
            // Directory removal/recreation affects every loaded descendant, even when the
            // generator finishes writing before we can install watches on the new tree.
            if rescan
                || event_paths.iter().any(|event_path| {
                    path == event_path || (structural && path.starts_with(event_path))
                })
            {
                change.revision += 1;
                change.changed = true;
                change.last_event_time = Some(now);
                change.error = None;
            }
        }
        if structural {
            // A cached path is not proof that the OS still watches its current directory:
            // deletion or rename may have detached the watch from that path's new instance.
            let invalidated: Vec<_> = self
                .directories
                .iter()
                .filter(|directory| {
                    rescan || event_paths.iter().any(|path| directory.starts_with(path))
                })
                .cloned()
                .collect();
            for directory in invalidated {
                if let Some(watcher) = &mut self.watcher {
                    let _ = watcher.unwatch(&directory);
                }
                self.directories.remove(&directory);
            }
        }
    }
    fn needed_directories(&self) -> BTreeSet<PathBuf> {
        // Non-recursive ancestor watches survive removal of any scenario parent without
        // recursively watching unrelated trees. Shared ancestors only need one registration.
        self.paths
            .keys()
            .flat_map(|path| path.ancestors().skip(1).map(Path::to_owned))
            .collect()
    }
    /// Start watching any required directories that are not currently being watched.
    /// Used both when opening a file and after filesystem events: deleting a scenario directory
    /// removes its OS watch, so recreating that directory requires registering a new watch.
    /// Missing directories are skipped until a watched parent reports their recreation.
    fn restore_directory_watches(&mut self) {
        let needed = self.needed_directories();
        let Some(watcher) = &mut self.watcher else {
            return;
        };
        for directory in needed {
            if self.directories.contains(&directory) || !directory.is_dir() {
                continue;
            }
            match watcher.watch(&directory, RecursiveMode::NonRecursive) {
                Ok(()) => {
                    self.directories.insert(directory);
                }
                // Disappearing directories are normal during regeneration. A surviving
                // ancestor will notify us when to retry; other errors remain visible.
                Err(_) if !directory.is_dir() => {}
                Err(error) => {
                    self.monitor_error =
                        Some(format!("Cannot monitor {}: {error}", directory.display()));
                }
            }
        }
    }
    /// Release registrations no longer needed by loaded files or pending opens.
    pub fn retain(&mut self, paths: &BTreeSet<PathBuf>) {
        self.paths.retain(|path, _| paths.contains(path));
        let needed = self.needed_directories();
        self.directories.retain(|directory| {
            if needed.contains(directory) {
                return true;
            }
            if let Some(watcher) = &mut self.watcher {
                let _ = watcher.unwatch(directory);
            }
            false
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, ModifyKind, RenameMode};

    #[test]
    fn coalesces_writes_and_ignores_access_and_unrelated_paths() {
        let mut changes = FileChanges::default();
        changes.disable_watcher();
        let path = changes.watch(Path::new("target/change-events.wav"));
        let now = Instant::now();
        changes.record_event(
            notify::Event::new(EventKind::Access(AccessKind::Read)).add_path(path.clone()),
            now,
        );
        changes.record_event(
            notify::Event::new(EventKind::Modify(ModifyKind::Any))
                .add_path(PathBuf::from("unrelated.wav")),
            now,
        );
        assert!(!changes.paths[&path].changed);
        for _ in 0..3 {
            changes.record_event(
                notify::Event::new(EventKind::Modify(ModifyKind::Any)).add_path(path.clone()),
                now,
            );
        }
        assert_eq!(changes.paths.len(), 1);
        assert_eq!(changes.paths[&path].revision, 3);
        assert!(!changes.paths[&path].ready());
        changes.paths.get_mut(&path).unwrap().last_event_time = Some(now - QUIET_PERIOD);
        assert!(changes.paths[&path].ready());
    }

    #[test]
    fn replacement_and_removal_remain_pending_until_acknowledged() {
        let mut changes = FileChanges::default();
        changes.disable_watcher();
        let path = changes.watch(Path::new("target/replaced.wav"));
        let now = Instant::now();
        changes.record_event(
            notify::Event::new(EventKind::Modify(ModifyKind::Name(RenameMode::Both)))
                .add_path(PathBuf::from("temporary.wav"))
                .add_path(path.clone()),
            now,
        );
        changes.record_event(
            notify::Event::new(EventKind::Remove(notify::event::RemoveKind::File))
                .add_path(path.clone()),
            now,
        );
        changes.record_event(
            notify::Event::new(EventKind::Create(notify::event::CreateKind::File))
                .add_path(path.clone()),
            now,
        );
        assert!(changes.paths[&path].changed);
        assert_eq!(changes.paths[&path].revision, 3);
        changes.retain(&BTreeSet::new());
        assert!(changes.paths.is_empty());
    }
    #[test]
    fn native_watcher_detects_overwrite_replacement_and_recreation() {
        let dir = std::env::current_dir().unwrap().join(format!(
            "target/test_output/native-watch-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("output.wav");
        std::fs::write(&path, b"original").unwrap();
        let mut changes = FileChanges::default();
        changes.watch(&path);
        assert!(
            changes.monitor_error.is_none(),
            "{:?}",
            changes.monitor_error
        );
        let wait_for_change = |changes: &mut FileChanges| {
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                changes.drain();
                if changes.paths[&path].changed && changes.paths[&path].ready() {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "No settled filesystem notification"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            changes.paths.get_mut(&path).unwrap().changed = false;
        };
        std::fs::write(&path, b"overwritten").unwrap();
        wait_for_change(&mut changes);
        let temporary = dir.join("replacement.tmp");
        std::fs::write(&temporary, b"replacement").unwrap();
        // Windows does not replace an existing destination with std::fs::rename.
        #[cfg(target_os = "windows")]
        std::fs::remove_file(&path).unwrap();
        std::fs::rename(&temporary, &path).unwrap();
        wait_for_change(&mut changes);
        std::fs::remove_file(&path).unwrap();
        wait_for_change(&mut changes);
        std::fs::write(&path, b"recreated").unwrap();
        wait_for_change(&mut changes);
    }
    #[test]
    fn directory_events_affect_only_descendants_and_release_stale_watches() {
        let mut changes = FileChanges::default();
        changes.disable_watcher();
        let base = absolute_path(Path::new("target/ancestor-events"));
        let scenario = base.join("scenario");
        let a = changes.watch(&scenario.join("run/a.wav"));
        let b = changes.watch(&scenario.join("run/b.wav"));
        let unrelated = changes.watch(&base.join("scenario-other/c.wav"));
        changes.directories = changes.needed_directories();
        changes.record_event(
            notify::Event::new(EventKind::Remove(notify::event::RemoveKind::Folder))
                .add_path(scenario.clone()),
            Instant::now(),
        );
        assert!(changes.paths[&a].changed);
        assert!(changes.paths[&b].changed);
        assert!(!changes.paths[&unrelated].changed);
        assert!(!changes.directories.contains(&scenario));
        assert!(!changes.directories.contains(&scenario.join("run")));
        assert!(changes.directories.contains(&base));
        // Closing one scenario must retain shared ancestors for other loaded files.
        changes.retain(&BTreeSet::from([unrelated]));
        assert!(changes.directories.contains(&base));
        changes.retain(&BTreeSet::new());
        assert!(changes.directories.is_empty());
    }

    #[test]
    fn native_watcher_recovers_after_scenario_ancestor_is_removed_or_renamed() {
        let root = std::env::current_dir().unwrap().join(format!(
            "target/test_output/scenario-watch-{}",
            std::process::id()
        ));
        let scenario = root.join("scenario");
        let directory = scenario.join("run/audio");
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("output.wav");
        std::fs::write(&path, b"original").unwrap();
        let mut changes = FileChanges::default();
        changes.watch(&path);
        assert!(
            changes.monitor_error.is_none(),
            "{:?}",
            changes.monitor_error
        );
        let wait_for_change = |changes: &mut FileChanges| {
            let deadline = Instant::now() + Duration::from_secs(3);
            loop {
                changes.drain();
                if changes.paths[&path].changed && changes.paths[&path].ready() {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "No settled scenario notification"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            changes.paths.get_mut(&path).unwrap().changed = false;
            assert!(
                changes.monitor_error.is_none(),
                "{:?}",
                changes.monitor_error
            );
        };
        // Process deletion while the tree is absent, then recover when each parent returns.
        std::fs::remove_dir_all(&scenario).unwrap();
        wait_for_change(&mut changes);
        assert!(!changes.directories.contains(&directory));
        assert!(changes.directories.contains(&root));
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(&path, b"first regeneration").unwrap();
        wait_for_change(&mut changes);
        assert!(changes.directories.contains(&directory));
        std::fs::write(&path, b"write after recreation").unwrap();
        wait_for_change(&mut changes);

        // A fast QC run may remove, recreate and populate the tree before the UI drains
        // even one event. Reattach to the new directories, not their cached old instances.
        std::fs::remove_dir_all(&scenario).unwrap();
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(&path, b"fast regeneration").unwrap();
        wait_for_change(&mut changes);
        std::fs::write(&path, b"write after fast regeneration").unwrap();
        wait_for_change(&mut changes);

        let archived = root.join(format!(
            "archived-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::rename(&scenario, &archived).unwrap();
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(&path, b"replacement scenario").unwrap();
        wait_for_change(&mut changes);
        std::fs::write(&path, b"write after rename").unwrap();
        wait_for_change(&mut changes);
    }
}
