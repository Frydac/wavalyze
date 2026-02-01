use crate::wav;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub struct LoadProgressEntry {
    pub path: PathBuf,
    pub handle: wav::read::LoadProgressHandle,
}

#[derive(Debug)]
/// Tracks in-flight loads and their progress handles.
///
/// This keeps async-ish load plumbing out of `Model` and provides a single
/// place to register loads and drain finished results.
pub struct LoadManager {
    tx: Sender<wav::read::LoadResult>,
    rx: Receiver<wav::read::LoadResult>,
    pending: usize,
    next_id: wav::read::LoadId,
    progress: HashMap<wav::read::LoadId, LoadProgressEntry>,
}

impl LoadManager {
    /// Create a fresh manager with a new result channel.
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            tx,
            rx,
            pending: 0,
            next_id: 1,
            progress: HashMap::new(),
        }
    }

    /// Returns a sender for background loaders to report results.
    pub fn sender(&self) -> Sender<wav::read::LoadResult> {
        self.tx.clone()
    }

    /// Number of loads still in progress.
    pub fn pending(&self) -> usize {
        self.pending
    }

    /// Register a new load and return its id so results can be matched later.
    pub fn start_load(
        &mut self,
        path: PathBuf,
        handle: wav::read::LoadProgressHandle,
    ) -> wav::read::LoadId {
        // Register a new load so the UI can show progress while results arrive.
        self.pending = self.pending.saturating_add(1);
        let load_id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.progress
            .insert(load_id, LoadProgressEntry { path, handle });
        load_id
    }

    /// Progress handle for a given load id, if still tracked.
    pub fn progress_handle(
        &self,
        load_id: wav::read::LoadId,
    ) -> Option<wav::read::LoadProgressHandle> {
        self.progress
            .get(&load_id)
            .map(|entry| entry.handle.clone())
    }

    /// First progress entry, used by the UI to display a single load.
    pub fn any_progress_entry(&self) -> Option<&LoadProgressEntry> {
        self.progress.values().next()
    }

    /// Drain finished loads; returns results paired with their progress handle.
    pub fn drain_results(
        &mut self,
    ) -> Vec<(wav::read::LoadResult, Option<wav::read::LoadProgressHandle>)> {
        // Drain any finished loads, returning results with their progress handle.
        let mut results = Vec::new();
        loop {
            match self.rx.try_recv() {
                Ok(result) => {
                    self.pending = self.pending.saturating_sub(1);
                    let load_id = match &result {
                        wav::read::LoadResult::Ok(loaded) => loaded.load_id,
                        wav::read::LoadResult::Err { load_id, .. } => *load_id,
                    };
                    let handle = self.progress_handle(load_id);
                    results.push((result, handle));
                    self.progress.remove(&load_id);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    tracing::error!("Load results channel disconnected");
                    break;
                }
            }
        }
        results
    }
}

impl Default for LoadManager {
    fn default() -> Self {
        Self::new()
    }
}
