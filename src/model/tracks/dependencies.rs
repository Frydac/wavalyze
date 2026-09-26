//! Dependency operations for diffs, independent of file paths and disk loading.
//!
//! Tracks can be reordered or removed while their audio remains an input to other diffs.
//! These helpers collect the recipes still needed by the workspace, order recomputations,
//! and redirect track references when a coordinator installs replacement buffers.

use super::Tracks;
use crate::{
    audio::{BufferId, manager::AudioManager},
    model::track::{TrackId, diff::Diff},
};
use std::collections::{HashMap, HashSet};

impl Tracks {
    /// Collect diff recipes needed by the current tracks, including removed intermediate tracks.
    /// Sorting by buffer ID keeps this snapshot independent of the user's track display order.
    pub(crate) fn diff_topology(&self) -> Vec<(BufferId, Diff)> {
        let mut definitions: std::collections::BTreeMap<_, _> = self
            .tracks
            .values()
            .filter_map(|t| t.diff.clone().map(|d| (d.buffer_id_diff, d)))
            .collect();
        // Only retain removed intermediate recipes reachable from a currently displayed diff.
        // Otherwise deleting a standalone diff would cause unnecessary work on every reload.
        loop {
            let missing: Vec<_> = definitions
                .values()
                .flat_map(|d| [d.buffer_id_a, d.buffer_id_b])
                .filter(|id| !definitions.contains_key(id))
                .filter_map(|id| self.retired_diffs.get(&id).cloned().map(|d| (id, d)))
                .collect();
            if missing.is_empty() {
                break;
            }
            definitions.extend(missing);
        }
        definitions.into_iter().collect()
    }

    /// Return source and dependent diff tracks in display order, including hidden tracks.
    pub(crate) fn affected_tracks_for_buffers(&self, changed: &HashSet<BufferId>) -> Vec<TrackId> {
        let affected = affected_diffs(&self.diff_topology(), changed);
        let affected_ids: HashSet<_> = affected.iter().map(|(id, _)| *id).collect();
        self.tracks_order
            .iter()
            .copied()
            .filter(|id| {
                self.get_track(*id)
                    .is_some_and(|t| affected_ids.contains(&t.single.buffer_id))
                    || self
                        .get_track(*id)
                        .is_some_and(|t| changed.contains(&t.single.buffer_id))
            })
            .collect()
    }

    /// Redirect displayed tracks and saved intermediate recipes to newly installed audio.
    /// Keep layout and selection, but invalidate waveform and hover data from the old buffers.
    pub(crate) fn apply_buffer_replacements(
        &mut self,
        replacements: &HashMap<BufferId, BufferId>,
        audio: &AudioManager,
    ) {
        // Update existing track records rather than recreating them: their IDs, visibility,
        // heights, offsets, and selections still describe the same user workspace.
        for track in self.tracks.values_mut() {
            if let Some(id) = replacements.get(&track.single.buffer_id) {
                track.single.buffer_id = *id;
                track.sample_rate = audio.buffers[*id].sample_rate();
                track.single.sample_view = None;
                track.single.mark_dirty();
            }
            if let Some(diff) = &mut track.diff {
                for id in [
                    &mut diff.buffer_id_a,
                    &mut diff.buffer_id_b,
                    &mut diff.buffer_id_diff,
                ] {
                    if let Some(new) = replacements.get(id) {
                        *id = *new;
                    }
                }
            }
        }
        // A removed intermediate track may still feed a visible diff. Its saved recipe must
        // point to the replacement buffers too, or the next reload would use missing inputs.
        let retired: Vec<_> = self
            .retired_diffs
            .drain()
            .map(|(old, mut diff)| {
                for id in [
                    &mut diff.buffer_id_a,
                    &mut diff.buffer_id_b,
                    &mut diff.buffer_id_diff,
                ] {
                    if let Some(new) = replacements.get(id) {
                        *id = *new;
                    }
                }
                (replacements.get(&old).copied().unwrap_or(old), diff)
            })
            .collect();
        self.retired_diffs.extend(retired);
        self.hover_info = Default::default();
    }
}

/// Find all diffs that use changed audio, directly or through another diff.
/// Then order them so an intermediate diff is recomputed before any diff that reads its output.
/// Track display order cannot determine this order because the user can rearrange tracks.
pub(crate) fn affected_diffs(
    topology: &[(BufferId, Diff)],
    changed: &HashSet<BufferId>,
) -> Vec<(BufferId, Diff)> {
    let mut dirty = changed.clone();
    loop {
        let before = dirty.len();
        for (_, diff) in topology {
            if dirty.contains(&diff.buffer_id_a) || dirty.contains(&diff.buffer_id_b) {
                dirty.insert(diff.buffer_id_diff);
            }
        }
        if before == dirty.len() {
            break;
        }
    }
    let mut remaining: Vec<_> = topology
        .iter()
        .filter(|(_, d)| dirty.contains(&d.buffer_id_diff))
        .cloned()
        .collect();
    let mut ordered = Vec::new();
    while !remaining.is_empty() {
        let outputs: HashSet<_> = remaining.iter().map(|(_, d)| d.buffer_id_diff).collect();
        let Some(index) = remaining.iter().position(|(_, d)| {
            !outputs.contains(&d.buffer_id_a) && !outputs.contains(&d.buffer_id_b)
        }) else {
            // Cycles cannot be created by the current UI (each diff allocates a fresh buffer).
            break;
        };
        ordered.push(remaining.remove(index));
    }
    ordered
}
