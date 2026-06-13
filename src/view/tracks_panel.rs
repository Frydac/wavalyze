use crate::{
    model::{Action, Model, pending_track_diff::PendingTrackDiff, track::TrackId},
    view::{track, util::add_row_label},
};

/// Which part of a target row the pointer is over while dragging: the middle (diff) or near an
/// edge (reorder into the gap above/below).
#[derive(Clone, Copy, PartialEq)]
enum DropZone {
    Before,
    On,
    After,
}

/// Classify a pointer Y within a row rect: top/bottom quarters are "between" (reorder), the middle
/// half is "on" (diff).
fn zone_for(pointer_y: f32, row_rect: egui::Rect) -> DropZone {
    let t = (pointer_y - row_rect.top()) / row_rect.height().max(1.0);
    if t < 0.25 {
        DropZone::Before
    } else if t > 0.75 {
        DropZone::After
    } else {
        DropZone::On
    }
}

/// What a completed drop resolves to. Reorders apply immediately; an "on" drop opens a
/// confirmation menu instead so reorder drags can't trigger accidental diffs.
enum DropResolution {
    Reorder {
        dragged: TrackId,
        to_gap_ix: usize,
    },
    OpenDiffMenu {
        dragged: TrackId,
        dropped_on: TrackId,
        pos: egui::Pos2,
    },
}

/// Left-panel "Tracks" tab: lists every track in track order with a visibility checkbox and a
/// hover popup matching the central-panel track header. Track-centric counterpart to the
/// file-centric [`super::file::ui`].
///
/// Dragging a row's label onto another row reorders (drop near a row edge) or opens a diff
/// confirmation menu (drop on the middle), with a highlight / insertion-line drawn under the
/// pointer while dragging.
pub fn ui(ui: &mut egui::Ui, model: &mut Model) {
    ui.add_space(5.0);
    ui.heading("Tracks");
    ui.add_space(5.0);

    if model.tracks.tracks_order.is_empty() {
        ui.label("No tracks");
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Clone the small id list so we can mutate the model (toggle visibility) inside the loop.
        let track_ids = model.tracks.tracks_order.clone();
        // Resolve at most one drop per frame, applied after the loop to avoid borrowing `model`.
        let mut pending: Option<DropResolution> = None;
        for (ix, track_id) in track_ids.into_iter().enumerate() {
            let Some(visible) = model.tracks.get_track(track_id).map(|track| track.visible) else {
                continue;
            };
            let label = format!("{ix}  {}", track::track_label(model, track_id));
            let hover_info = track::header_hover_info(model, track_id);

            let row = ui.horizontal(|ui| {
                let mut checked = visible;
                let response = ui.add(egui::Checkbox::without_text(&mut checked));
                if response.changed() {
                    model.tracks.set_track_visibility(track_id, checked);
                }
                // The label is the drag handle: dragging one row's label onto another reorders or
                // opens a diff menu depending on the drop zone (resolved on the row-wide response).
                let dnd_id = egui::Id::new(("track_row_dnd", track_id));
                let label_response = ui
                    .dnd_drag_source(dnd_id, track_id, |ui| add_row_label(ui, label))
                    .inner;
                if let Some(hover_info) = hover_info {
                    label_response.on_hover_ui(move |ui| hover_info.show(ui, track_id));
                }
            });

            // Row-wide drop target: an explicit hover interaction guarantees `contains_pointer`
            // across the full row rect (the label alone is too narrow).
            let row_rect = row.response.rect;
            let drop_resp = ui.interact(
                row_rect,
                ui.id().with(("track_drop", track_id)),
                egui::Sense::hover(),
            );
            let pointer_pos = ui.input(|i| i.pointer.hover_pos());
            let pointer_y = pointer_pos.map_or(row_rect.center().y, |p| p.y);
            let zone = zone_for(pointer_y, row_rect);

            if drop_resp.dnd_hover_payload::<TrackId>().is_some() {
                let painter = ui.painter();
                match zone {
                    DropZone::On => {
                        painter.rect_filled(row_rect, 2.0, ui.visuals().selection.bg_fill);
                    }
                    DropZone::Before => {
                        painter.hline(
                            row_rect.x_range(),
                            row_rect.top(),
                            ui.visuals().selection.stroke,
                        );
                    }
                    DropZone::After => {
                        painter.hline(
                            row_rect.x_range(),
                            row_rect.bottom(),
                            ui.visuals().selection.stroke,
                        );
                    }
                }
            }

            if let Some(dragged) = drop_resp.dnd_release_payload::<TrackId>() {
                pending = match zone {
                    DropZone::On if *dragged != track_id => Some(DropResolution::OpenDiffMenu {
                        dragged: *dragged,
                        dropped_on: track_id,
                        pos: pointer_pos.unwrap_or_else(|| row_rect.center()),
                    }),
                    DropZone::On => None,
                    DropZone::Before => Some(DropResolution::Reorder {
                        dragged: *dragged,
                        to_gap_ix: ix,
                    }),
                    DropZone::After => Some(DropResolution::Reorder {
                        dragged: *dragged,
                        to_gap_ix: ix + 1,
                    }),
                };
            }

            ui.add_space(2.0);
        }
        match pending {
            Some(DropResolution::Reorder { dragged, to_gap_ix }) => model
                .actions
                .push(Action::ReorderTrack { dragged, to_gap_ix }),
            Some(DropResolution::OpenDiffMenu {
                dragged,
                dropped_on,
                pos,
            }) => {
                model.pending_track_diff = Some(PendingTrackDiff {
                    dragged,
                    dropped_on,
                    screen_pos: (pos.x, pos.y),
                    armed: false,
                });
            }
            None => {}
        }
    });

    show_diff_menu(ui, model);
}

/// Confirmation context menu shown after a track is dropped *on* another, while
/// `model.pending_track_diff` is `Some`. Closes on Diff/Cancel, Escape, or a click outside.
fn show_diff_menu(ui: &mut egui::Ui, model: &mut Model) {
    let Some(pending) = model.pending_track_diff.clone() else {
        return;
    };
    let label_a = track::track_label(model, pending.dragged);
    let label_b = track::track_label(model, pending.dropped_on);
    let pos = egui::pos2(pending.screen_pos.0, pending.screen_pos.1);

    let mut do_diff = false;
    let mut cancel = false;
    let area = egui::Area::new(egui::Id::new("track_diff_context_menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(pos)
        .constrain(true)
        .show(ui.ctx(), |ui| {
            egui::Frame::menu(ui.style()).show(ui, |ui| {
                ui.label(format!("{label_a}  −  {label_b}"));
                ui.separator();
                if ui.button("Diff").clicked() {
                    do_diff = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });

    let escaped = ui.input(|i| i.key_pressed(egui::Key::Escape));
    // Don't let the same pointer release that opened the menu dismiss it via click-outside; only
    // arm the click-outside check once the menu has survived a frame.
    let clicked_outside = pending.armed && area.response.clicked_elsewhere();

    if do_diff {
        model.actions.push(Action::DiffTracks {
            dragged: pending.dragged,
            dropped_on: pending.dropped_on,
        });
        model.pending_track_diff = None;
    } else if cancel || escaped || clicked_outside {
        model.pending_track_diff = None;
    } else if !pending.armed {
        if let Some(p) = model.pending_track_diff.as_mut() {
            p.armed = true;
        }
        ui.ctx().request_repaint();
    }
}
