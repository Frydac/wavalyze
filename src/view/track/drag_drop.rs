//! Drag source and drop target behavior for tracks in the central panel.
//!
//! Headers start `TrackId` drags. Each full track rectangle accepts drops: narrow edge bands
//! reorder, while the body opens diff confirmation.

use crate::model::{
    Action, Model, pending_track_diff::PendingTrackDiff, track::TrackId,
    track_selection::TrackSelectionMode,
};

/// Fixed edge band for normal tracks; short tracks scale this down in [`zone_for`].
const REORDER_EDGE_HEIGHT: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DropZone {
    Before,
    On,
    After,
}

/// Turn an eligible header drag into the shared `TrackId` payload used by both track views.
pub(super) fn handle_header_drag(
    ui: &egui::Ui,
    response: &egui::Response,
    model: &mut Model,
    track_id: TrackId,
) {
    let selected = model.tracks.track_selection.is_selected(track_id);
    let can_drag = can_start_drag(
        selected,
        model.tracks.track_selection.selected_ids().count(),
    );

    if response.drag_started() && can_drag {
        // Dragging an unselected track first makes it the sole selection. Selection remains queued
        // through the normal action path, but payload setup must happen now for this drag.
        if !selected {
            model.actions.push(Action::SelectTrack {
                track_id,
                mode: TrackSelectionMode::Replace,
            });
        }
        response.dnd_set_drag_payload(track_id);
    }

    if response.dragged() && can_drag {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if response.hovered() && can_drag {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
}

/// Register one full central track as a drop target, paint feedback, then resolve released drops.
pub(super) fn handle_track_drop(
    ui: &mut egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    track_rect: egui::Rect,
) {
    // Separate hover-only interaction gives the full track one target without stealing clicks
    // from headers, waveform controls, or resize handles.
    let response = ui.interact(
        track_rect,
        ui.id().with(("central_track_drop", track_id)),
        egui::Sense::hover(),
    );
    let pointer_pos = ui.input(|input| input.pointer.hover_pos());
    let pointer_y = pointer_pos.map_or(track_rect.center().y, |pos| pos.y);
    let zone = zone_for(pointer_y, track_rect);

    // Paint after track contents so the insertion line or outline stays visible over the waveform.
    if let Some(dragged) = response.dnd_hover_payload::<TrackId>() {
        let color = ui.visuals().selection.stroke.color;
        match zone {
            DropZone::Before => {
                ui.painter().hline(
                    track_rect.x_range(),
                    track_rect.top(),
                    egui::Stroke::new(3.0, color),
                );
            }
            DropZone::After => {
                ui.painter().hline(
                    track_rect.x_range(),
                    track_rect.bottom(),
                    egui::Stroke::new(3.0, color),
                );
            }
            DropZone::On if *dragged != track_id => {
                ui.painter().rect_stroke(
                    track_rect.shrink(1.0),
                    2.0,
                    egui::Stroke::new(2.0, color),
                    egui::epaint::StrokeKind::Inside,
                );
            }
            DropZone::On => {}
        }
    }

    // Reordering uses the existing action queue. Diffing pauses in model state until the shared
    // confirmation menu accepts or cancels it.
    let Some(dragged) = response.dnd_release_payload::<TrackId>() else {
        return;
    };
    let Some(track_ix) = model
        .tracks
        .tracks_order
        .iter()
        .position(|id| *id == track_id)
    else {
        return;
    };

    match zone {
        DropZone::Before => model.actions.push(Action::ReorderTrack {
            dragged: *dragged,
            to_gap_ix: track_ix,
        }),
        DropZone::After => model.actions.push(Action::ReorderTrack {
            dragged: *dragged,
            to_gap_ix: track_ix + 1,
        }),
        DropZone::On if *dragged != track_id => {
            let pos = pointer_pos.unwrap_or_else(|| track_rect.center());
            model.pending_track_diff = Some(PendingTrackDiff {
                dragged: *dragged,
                dropped_on: track_id,
                screen_pos: (pos.x, pos.y),
                armed: false,
            });
        }
        DropZone::On => {}
    }
}

/// Group dragging is deferred: a selected source is draggable only when selected alone.
fn can_start_drag(selected: bool, selected_count: usize) -> bool {
    !selected || selected_count == 1
}

/// Split a track into reorder edges and a diff body without consuming short tracks entirely.
fn zone_for(pointer_y: f32, track_rect: egui::Rect) -> DropZone {
    let edge_height = REORDER_EDGE_HEIGHT.min(track_rect.height() / 4.0);
    if pointer_y < track_rect.top() + edge_height {
        DropZone::Before
    } else if pointer_y > track_rect.bottom() - edge_height {
        DropZone::After
    } else {
        DropZone::On
    }
}

#[cfg(test)]
mod tests {
    use super::{DropZone, can_start_drag, zone_for};

    #[test]
    fn selected_track_requires_single_selection() {
        assert!(can_start_drag(true, 1));
        assert!(!can_start_drag(true, 2));
    }

    #[test]
    fn unselected_track_can_replace_any_selection() {
        assert!(can_start_drag(false, 0));
        assert!(can_start_drag(false, 1));
        assert!(can_start_drag(false, 2));
    }

    #[test]
    fn drop_zone_uses_narrow_edge_bands() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 100.0), egui::pos2(200.0, 300.0));

        assert_eq!(zone_for(105.0, rect), DropZone::Before);
        assert_eq!(zone_for(112.0, rect), DropZone::On);
        assert_eq!(zone_for(200.0, rect), DropZone::On);
        assert_eq!(zone_for(288.0, rect), DropZone::On);
        assert_eq!(zone_for(295.0, rect), DropZone::After);
    }

    #[test]
    fn drop_zone_scales_down_for_short_tracks() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 10.0), egui::pos2(100.0, 30.0));

        assert_eq!(zone_for(12.0, rect), DropZone::Before);
        assert_eq!(zone_for(15.0, rect), DropZone::On);
        assert_eq!(zone_for(25.0, rect), DropZone::On);
        assert_eq!(zone_for(28.0, rect), DropZone::After);
    }
}
