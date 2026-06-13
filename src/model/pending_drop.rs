//! Pending state for a drag-and-drop of exactly two files: the user is asked whether to diff the
//! two files or load them both as separate tracks. The view shows a small chooser dialog while
//! `model.pending_drop_choice` is `Some`.

use std::path::PathBuf;

/// Two dropped files awaiting a Diff/Load decision by the user.
#[derive(Debug)]
pub struct PendingDropChoice {
    pub path_a: PathBuf,
    pub path_b: PathBuf,
}
