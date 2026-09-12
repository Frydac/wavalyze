//! Track selection state and click semantics.
//!
//! Views queue a selection mode; [`TrackSelection::apply`] resolves it against current visual
//! track order so Shift ranges keep working after reordering.

use std::collections::HashSet;

use crate::model::track::TrackId;

/// Meaning of a track click after keyboard modifiers are considered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackSelectionMode {
    Replace,
    Toggle,
    Range,
}

/// Selected track IDs plus the starting point used by Shift-range selection.
#[derive(Debug, Clone, Default)]
pub struct TrackSelection {
    selected: HashSet<TrackId>,
    /// Last plain/toggle click. Shift selects between this track and the clicked track.
    anchor: Option<TrackId>,
}

impl TrackSelection {
    /// Apply one click. Unknown IDs are ignored because queued actions may outlive removed tracks.
    pub fn apply(&mut self, track_id: TrackId, mode: TrackSelectionMode, track_order: &[TrackId]) {
        let Some(track_ix) = track_order.iter().position(|id| *id == track_id) else {
            return;
        };

        match mode {
            TrackSelectionMode::Replace => self.replace(track_id),
            TrackSelectionMode::Toggle => {
                if !self.selected.remove(&track_id) {
                    self.selected.insert(track_id);
                }
                self.anchor = Some(track_id);
            }
            TrackSelectionMode::Range => {
                let Some(anchor_ix) = self
                    .anchor
                    .and_then(|anchor| track_order.iter().position(|id| *id == anchor))
                else {
                    self.replace(track_id);
                    return;
                };
                let (start, end) = if anchor_ix <= track_ix {
                    (anchor_ix, track_ix)
                } else {
                    (track_ix, anchor_ix)
                };
                self.selected = track_order[start..=end].iter().copied().collect();
            }
        }
    }

    pub fn is_selected(&self, track_id: TrackId) -> bool {
        self.selected.contains(&track_id)
    }

    pub fn selected_ids(&self) -> impl Iterator<Item = TrackId> + '_ {
        self.selected.iter().copied()
    }

    pub fn remove(&mut self, track_id: TrackId) {
        self.selected.remove(&track_id);
        if self.anchor == Some(track_id) {
            self.anchor = None;
        }
    }

    pub fn clear(&mut self) {
        self.selected.clear();
        self.anchor = None;
    }

    fn replace(&mut self, track_id: TrackId) {
        self.selected.clear();
        self.selected.insert(track_id);
        self.anchor = Some(track_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    fn track_ids(count: usize) -> Vec<TrackId> {
        let mut tracks = SlotMap::<TrackId, ()>::with_key();
        (0..count).map(|_| tracks.insert(())).collect()
    }

    fn assert_selected(selection: &TrackSelection, expected: &[TrackId]) {
        assert_eq!(selection.selected_ids().count(), expected.len());
        for track_id in expected {
            assert!(selection.is_selected(*track_id));
        }
    }

    #[test]
    fn replace_keeps_only_clicked_track() {
        let order = track_ids(2);
        let mut selection = TrackSelection::default();

        selection.apply(order[0], TrackSelectionMode::Replace, &order);
        selection.apply(order[1], TrackSelectionMode::Replace, &order);

        assert_selected(&selection, &[order[1]]);
    }

    #[test]
    fn toggle_adds_and_removes_tracks() {
        let order = track_ids(3);
        let mut selection = TrackSelection::default();

        selection.apply(order[0], TrackSelectionMode::Replace, &order);
        selection.apply(order[2], TrackSelectionMode::Toggle, &order);
        assert_selected(&selection, &[order[0], order[2]]);

        selection.apply(order[0], TrackSelectionMode::Toggle, &order);
        assert_selected(&selection, &[order[2]]);
    }

    #[test]
    fn range_replaces_selection_between_anchor_and_clicked_track() {
        let order = track_ids(4);
        let mut selection = TrackSelection::default();

        selection.apply(order[1], TrackSelectionMode::Replace, &order);
        selection.apply(order[3], TrackSelectionMode::Range, &order);
        assert_selected(&selection, &order[1..=3]);

        selection.apply(order[0], TrackSelectionMode::Range, &order);
        assert_selected(&selection, &order[0..=1]);
    }

    #[test]
    fn range_uses_current_track_order() {
        let ids = track_ids(4);
        let mut selection = TrackSelection::default();
        selection.apply(ids[1], TrackSelectionMode::Replace, &ids);

        let reordered = [ids[3], ids[1], ids[0], ids[2]];
        selection.apply(ids[2], TrackSelectionMode::Range, &reordered);

        assert_selected(&selection, &[ids[1], ids[0], ids[2]]);
    }

    #[test]
    fn removing_anchor_makes_next_range_a_replacement() {
        let order = track_ids(3);
        let mut selection = TrackSelection::default();
        selection.apply(order[1], TrackSelectionMode::Replace, &order);

        selection.remove(order[1]);
        selection.apply(order[2], TrackSelectionMode::Range, &order);

        assert_selected(&selection, &[order[2]]);
    }

    #[test]
    fn clear_removes_selection_and_anchor() {
        let order = track_ids(2);
        let mut selection = TrackSelection::default();
        selection.apply(order[0], TrackSelectionMode::Replace, &order);

        selection.clear();
        selection.apply(order[1], TrackSelectionMode::Range, &order);

        assert_selected(&selection, &[order[1]]);
    }
}
