//! Small native disk queries shared by file loading and reload validation.
//!
//! These helpers do not own model state. Keeping the same metadata comparison on the worker
//! and UI sides lets reload reject a file that changed between preparation and integration.

use anyhow::{Context, Result};
use std::{
    path::{Path, PathBuf},
    time::SystemTime,
};

/// File size and modification time recorded to detect writes during loading.
/// Checked together with watcher revisions; these metadata values are not a content checksum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Stamp {
    len: u64,
    modified: SystemTime,
}
/// Read metadata used to check for disk changes around loading and before integration.
pub(crate) fn stamp(path: &Path) -> Result<Stamp> {
    let metadata =
        std::fs::metadata(path).with_context(|| format!("Cannot read {}", path.display()))?;
    Ok(Stamp {
        len: metadata.len(),
        modified: metadata.modified()?,
    })
}

/// Resolve the directory, but keep the filename: replacement must continue watching the path.
pub(crate) fn absolute_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir().unwrap_or_default().join(path)
    };
    match (absolute.parent(), absolute.file_name()) {
        (Some(parent), Some(name)) => parent
            .canonicalize()
            .unwrap_or_else(|_| parent.to_owned())
            .join(name),
        _ => absolute,
    }
}
