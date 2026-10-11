//! Ableton-style overview strip shown above the time ruler.
//!
//! The overview compresses the longest visible buffer into one horizontal bar and draws the
//! current time-camera viewport inside it. Interactions are translated back into existing model
//! actions (`PanX` and `ZoomX`) so the overview remains a view-layer widget rather than owning
//! separate navigation state.
//!
//! Layout contract: `overview_rect` and `ruler_rect` should have the same horizontal extent.
//! The overview maps mouse deltas through its own width, then applies `ZoomX` around anchors in
//! `ruler_rect`; if those widths diverge, edge-resizing will no longer visually track the pointer.

use crate::model::{self, Action, selection_info::SelectionInfoE, time_camera};
use crate::view::overview::{self, DragMotion};

pub(crate) const HEIGHT: f32 = 18.0;

/// Horizontal edge hit area for resizing either side of the viewport rect.
const EDGE_HIT_WIDTH: f32 = 6.0;
/// Minimum draggable center area, even when the viewport rect is visually very narrow.
const MIN_PAN_HIT_WIDTH: f32 = 10.0;

#[derive(Debug, Clone, Copy)]
struct Geometry {
    /// Inner drawing rect after border inset; all overview-time mapping uses this rect.
    inner_rect: egui::Rect,
    /// Actual time-ruler rect, used as the anchor space for `ZoomX` actions.
    ruler_rect: egui::Rect,
    /// Visible viewport indicator inside the overview.
    viewport_rect: egui::Rect,
    /// Non-overlapping interaction zones derived from `viewport_rect`.
    pan_rect: egui::Rect,
    left_resize_rect: egui::Rect,
    right_resize_rect: egui::Rect,
    /// Duration represented by the full overview width.
    total_duration_s: f64,
}

pub(crate) fn ui(
    ui: &mut egui::Ui,
    model: &mut model::Model,
    overview_rect: egui::Rect,
    ruler_rect: egui::Rect,
) {
    debug_assert!((overview_rect.left() - ruler_rect.left()).abs() < f32::EPSILON);
    debug_assert!((overview_rect.right() - ruler_rect.right()).abs() < f32::EPSILON);

    // Always draw the overview shell so the reserved strip is visible before/without buffers.
    let visuals = ui.visuals();
    let stroke = visuals.widgets.noninteractive.bg_stroke;
    ui.painter().rect(
        overview_rect,
        3.0,
        visuals.extreme_bg_color,
        stroke,
        egui::epaint::StrokeKind::Inside,
    );

    let Some(geometry) = geometry(model, overview_rect, ruler_rect) else {
        return;
    };

    // Double-clicking anywhere on the overview is a quick way back to the full visible duration.
    let overview_response = ui.interact(
        overview_rect,
        ui.id().with("overview_zoom_to_full"),
        egui::Sense::click(),
    );
    if overview_response.double_clicked() {
        model.actions.push(Action::ZoomToFull);
        return;
    }

    draw_selection(ui, model, geometry);
    handle_interaction(ui, model, geometry);
    draw_viewport(ui, model, geometry.viewport_rect);
}

fn geometry(
    model: &model::Model,
    overview_rect: egui::Rect,
    ruler_rect: egui::Rect,
) -> Option<Geometry> {
    // The overview represents the longest visible track; hidden tracks do not affect scale.
    let visible_width = ruler_rect.width();
    if overview_rect.width() <= 0.0 || overview_rect.height() <= 0.0 || visible_width <= 0.0 {
        return None;
    }

    let total_duration_s = visible_tracks_duration_s(model)?;
    if total_duration_s <= 0.0 {
        return None;
    }

    let inner_rect = overview_rect.shrink(1.0);
    if inner_rect.width() <= 0.0 || inner_rect.height() <= 0.0 {
        return None;
    }

    // Project the current time camera into overview coordinates and clamp to the loaded content.
    let visible_range = model.tracks.time_camera.time_range(visible_width as f64);
    let start = visible_range.start.clamp(0.0, total_duration_s);
    let end = visible_range.end.clamp(0.0, total_duration_s);
    let x_start = time_to_overview_x(start, total_duration_s, inner_rect);
    let x_end = time_to_overview_x(end, total_duration_s, inner_rect);
    let mut left = x_start.min(x_end);
    let mut right = x_start.max(x_end);
    // Keep a one-pixel edge marker visible when panned completely before/after the buffer.
    if right - left < 1.0 {
        if left >= inner_rect.right() {
            left = inner_rect.right() - 1.0;
            right = inner_rect.right();
        } else {
            right = (left + 1.0).min(inner_rect.right());
        }
    }
    let viewport_rect = egui::Rect::from_min_max(
        egui::pos2(left, inner_rect.top()),
        egui::pos2(right, inner_rect.bottom()),
    );
    let (pan_rect, left_resize_rect, right_resize_rect) =
        interaction_rects(inner_rect, viewport_rect, EDGE_HIT_WIDTH, MIN_PAN_HIT_WIDTH);

    Some(Geometry {
        inner_rect,
        ruler_rect,
        viewport_rect,
        pan_rect,
        left_resize_rect,
        right_resize_rect,
        total_duration_s,
    })
}

fn interaction_rects(
    inner_rect: egui::Rect,
    viewport_rect: egui::Rect,
    edge_width: f32,
    min_pan_width: f32,
) -> (egui::Rect, egui::Rect, egui::Rect) {
    let viewport_width = viewport_rect.width();
    let edge_width = edge_width.min(inner_rect.width() / 2.0).max(0.0);
    let min_pan_width = min_pan_width.min(inner_rect.width()).max(1.0);

    // Wide viewports get simple left-edge / center-pan / right-edge zones.
    if viewport_width >= edge_width * 2.0 + min_pan_width {
        let pan_rect = egui::Rect::from_min_max(
            egui::pos2(viewport_rect.left() + edge_width, viewport_rect.top()),
            egui::pos2(viewport_rect.right() - edge_width, viewport_rect.bottom()),
        );
        let left_resize_rect = egui::Rect::from_min_max(
            viewport_rect.left_top(),
            egui::pos2(pan_rect.left(), viewport_rect.bottom()),
        );
        let right_resize_rect = egui::Rect::from_min_max(
            egui::pos2(pan_rect.right(), viewport_rect.top()),
            viewport_rect.right_bottom(),
        );
        return (pan_rect, left_resize_rect, right_resize_rect);
    }

    // Tiny viewports still need an easy panning target, so reserve a centered pan zone and put
    // resize handles around it where there is space.
    let center_x = viewport_rect
        .center()
        .x
        .clamp(inner_rect.left(), inner_rect.right());
    let pan_left = (center_x - min_pan_width / 2.0)
        .clamp(inner_rect.left(), inner_rect.right() - min_pan_width);
    let pan_right = pan_left + min_pan_width;
    let pan_rect = egui::Rect::from_min_max(
        egui::pos2(pan_left, viewport_rect.top()),
        egui::pos2(pan_right, viewport_rect.bottom()),
    );
    let hit_left = (viewport_rect.left() - edge_width).max(inner_rect.left());
    let hit_right = (viewport_rect.right() + edge_width).min(inner_rect.right());
    let left_resize_rect = egui::Rect::from_min_max(
        egui::pos2(hit_left, viewport_rect.top()),
        egui::pos2(pan_rect.left(), viewport_rect.bottom()),
    );
    let right_resize_rect = egui::Rect::from_min_max(
        egui::pos2(pan_rect.right(), viewport_rect.top()),
        egui::pos2(hit_right, viewport_rect.bottom()),
    );
    (pan_rect, left_resize_rect, right_resize_rect)
}

fn handle_interaction(ui: &mut egui::Ui, model: &mut model::Model, geometry: Geometry) {
    // Register resize handles before the pan area so edge drags win if zones ever touch.
    let left_response = interact_if_positive(
        ui,
        geometry.left_resize_rect,
        "overview_viewport_left_resize",
        egui::CursorIcon::ResizeColumn,
    );
    let right_response = interact_if_positive(
        ui,
        geometry.right_resize_rect,
        "overview_viewport_right_resize",
        egui::CursorIcon::ResizeColumn,
    );
    let pan_response = interact_if_positive(
        ui,
        geometry.pan_rect,
        "overview_viewport_pan",
        egui::CursorIcon::PointingHand,
    );

    let active = [
        (DragAction::ResizeLeft, left_response.as_ref()),
        (DragAction::ResizeRight, right_response.as_ref()),
        (DragAction::Pan, pan_response.as_ref()),
    ]
    .into_iter()
    .find_map(|(action, response)| {
        response
            .filter(|response| response.dragged() || response.drag_stopped())
            .map(|response| (action, response))
    });
    let Some((action, response)) = active else {
        return;
    };

    let seconds_per_pixel = model.tracks.time_camera.seconds_per_pixel();
    if seconds_per_pixel <= 0.0 || geometry.inner_rect.width() <= 0.0 {
        return;
    }

    if response.drag_started()
        && let Some(origin) = ui.input(|input| input.pointer.press_origin())
    {
        let span = seconds_per_pixel * geometry.ruler_rect.width() as f64;
        let unit = geometry.total_duration_s / geometry.inner_rect.width() as f64;
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                response.id,
                DragMotion::new(unit, span, geometry.inner_rect.width(), origin.x),
            )
        });
    }
    let Some(mut motion) = ui
        .ctx()
        .data(|data| data.get_temp::<DragMotion>(response.id))
    else {
        return;
    };
    let Some(pos) = response.interact_pointer_pos() else {
        return;
    };
    let delta_s = motion.advance(pos.x, ui.input(|input| input.modifiers.shift));
    if response.drag_stopped() {
        ui.ctx()
            .data_mut(|data| data.remove::<DragMotion>(response.id));
    } else {
        ui.ctx()
            .data_mut(|data| data.insert_temp(response.id, motion));
    }
    if delta_s == 0.0 {
        return;
    }

    // Convert overview movement to the current ruler scale to reuse existing model actions.
    let nr_pixels = delta_s / seconds_per_pixel;
    match action {
        DragAction::Pan => {
            model.actions.push(Action::PanX {
                nr_pixels: nr_pixels as f32,
            });
        }
        DragAction::ResizeLeft => {
            model.actions.push(Action::ZoomX {
                nr_pixels: -nr_pixels as f32,
                center_x: geometry.ruler_rect.right(),
            });
        }
        DragAction::ResizeRight => {
            model.actions.push(Action::ZoomX {
                nr_pixels: nr_pixels as f32,
                center_x: geometry.ruler_rect.left(),
            });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragAction {
    Pan,
    ResizeLeft,
    ResizeRight,
}

fn interact_if_positive(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    id: &'static str,
    cursor_icon: egui::CursorIcon,
) -> Option<egui::Response> {
    rect.is_positive().then(|| {
        ui.interact(rect, ui.id().with(id), egui::Sense::click_and_drag())
            .on_hover_cursor(cursor_icon)
            .on_hover_text("Drag to adjust the visible time range; hold Shift for fine adjustment")
    })
}

fn draw_viewport(ui: &mut egui::Ui, model: &model::Model, viewport_rect: egui::Rect) {
    if viewport_rect.is_positive() {
        let accent = model.user_config.active_theme_colors(ui.visuals()).accent;
        overview::draw_viewport(ui.painter(), viewport_rect, 2, accent);
    }
}

fn draw_selection(ui: &mut egui::Ui, model: &model::Model, geometry: Geometry) {
    let SelectionInfoE::IsSelected(selection_info) = model.tracks.selection_info else {
        return;
    };
    if selection_info.ix_rng.end <= selection_info.ix_rng.start {
        return;
    }

    // Selection sample indices are global/ruler-facing values, so use the same reference sample
    // rate that the time ruler uses for sample-index display.
    let Some(sample_rate) = model.tracks.reference_sample_rate() else {
        return;
    };
    let start_s = time_camera::sample_ix_to_time(selection_info.ix_rng.start as f64, sample_rate)
        .clamp(0.0, geometry.total_duration_s);
    let end_s = time_camera::sample_ix_to_time(selection_info.ix_rng.end as f64, sample_rate)
        .clamp(0.0, geometry.total_duration_s);
    if start_s == end_s {
        return;
    }

    let x_start = time_to_overview_x(start_s, geometry.total_duration_s, geometry.inner_rect);
    let x_end = time_to_overview_x(end_s, geometry.total_duration_s, geometry.inner_rect);
    let mut left = x_start.min(x_end);
    let mut right = x_start.max(x_end);
    if right - left < 1.0 {
        right = (left + 1.0).min(geometry.inner_rect.right());
        if right == geometry.inner_rect.right() {
            left = (right - 1.0).max(geometry.inner_rect.left());
        }
    }

    let selection_rect = egui::Rect::from_min_max(
        egui::pos2(left, geometry.inner_rect.top()),
        egui::pos2(right, geometry.inner_rect.bottom()),
    );
    if !selection_rect.is_positive() {
        return;
    }

    let theme_colors = model.user_config.active_theme_colors(ui.visuals());
    ui.painter()
        .rect_filled(selection_rect, 1.0, theme_colors.waveform_selection_fill);

    let edge_stroke = egui::Stroke::new(1.0, theme_colors.accent.gamma_multiply(0.7));
    ui.painter().vline(
        selection_rect.left(),
        selection_rect.top()..=selection_rect.bottom(),
        edge_stroke,
    );
    ui.painter().vline(
        selection_rect.right(),
        selection_rect.top()..=selection_rect.bottom(),
        edge_stroke,
    );
}

fn visible_tracks_duration_s(model: &model::Model) -> Option<f64> {
    // Use model.audio as the source of truth for buffer lengths; tracks only tell us visibility
    // and which buffer they display.
    let mut longest = None;
    for track_id in &model.tracks.tracks_order {
        let Some(track) = model.tracks.get_track(*track_id) else {
            continue;
        };
        if !track.visible {
            continue;
        }
        let Ok(buffer) = model.audio.get_buffer(track.single.buffer_id) else {
            continue;
        };
        let sample_rate = buffer.sample_rate();
        if sample_rate == 0 {
            continue;
        }
        let duration_s = buffer.nr_samples() as f64 / sample_rate as f64;
        if longest.is_none_or(|longest| duration_s > longest) {
            longest = Some(duration_s);
        }
    }
    longest
}

fn time_to_overview_x(time_s: f64, total_duration_s: f64, overview_rect: egui::Rect) -> f32 {
    let t = (time_s / total_duration_s).clamp(0.0, 1.0) as f32;
    overview_rect.left() + t * overview_rect.width()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shift_slows_pan_and_both_edges_and_can_be_toggled_without_jumps() {
        for kind in [
            DragAction::Pan,
            DragAction::ResizeLeft,
            DragAction::ResizeRight,
        ] {
            for fine in [false, true] {
                let mut model = model::Model::new();
                #[cfg(not(target_arch = "wasm32"))]
                model.files.changes.disable_watcher();
                let buffer = crate::audio::buffer::BufferE::F32(
                    crate::audio::buffer::Buffer::<f32>::with_size(1000, 32, 16000),
                );
                let buffer_id = model.audio.buffers.insert(std::sync::Arc::new(buffer));
                model
                    .tracks
                    .add_track_to_end(buffer_id, 1000, &model.user_config.track)
                    .unwrap();
                let overview =
                    egui::Rect::from_min_size(egui::pos2(20.0, 20.0), egui::vec2(400.0, HEIGHT));
                let ruler =
                    egui::Rect::from_min_size(egui::pos2(20.0, 38.0), egui::vec2(400.0, 50.0));
                model.tracks.ruler.set_screen_rect(ruler.into());
                model.tracks.time_camera.time_start = 2.0;
                model.tracks.time_camera.set_seconds_per_pixel(0.01);
                let geo = geometry(&model, overview, ruler).unwrap();
                let origin = match kind {
                    DragAction::Pan => geo.pan_rect.center(),
                    DragAction::ResizeLeft => geo.left_resize_rect.center(),
                    DragAction::ResizeRight => geo.right_resize_rect.center(),
                };
                let ctx = egui::Context::default();
                let mut time = 0.0;
                let mut frame = |model: &mut model::Model, events, shift| {
                    time += 0.1;
                    let _ = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(600.0, 200.0),
                            )),
                            time: Some(time),
                            events,
                            modifiers: egui::Modifiers {
                                shift,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        |ctx| {
                            egui::CentralPanel::default()
                                .show(ctx, |ui| super::ui(ui, model, overview, ruler));
                        },
                    );
                    for action in std::mem::take(&mut model.actions) {
                        action.process(model).unwrap();
                    }
                    model.tracks.time_camera.time_range(ruler.width() as f64)
                };
                let button = |pos, pressed, shift| egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers {
                        shift,
                        ..Default::default()
                    },
                };
                frame(&mut model, vec![], fine);
                frame(
                    &mut model,
                    vec![
                        egui::Event::PointerMoved(origin),
                        button(origin, true, fine),
                    ],
                    fine,
                );
                let end = origin + egui::vec2(8.0, 0.0);
                let moved = frame(&mut model, vec![egui::Event::PointerMoved(end)], fine);
                let delta =
                    if fine { 4.0 * 0.1 } else { 16.0 } / geo.inner_rect.width() as f64 * 8.0;
                let expected_start = 2.0
                    + if kind == DragAction::ResizeRight {
                        0.0
                    } else {
                        delta
                    };
                let expected_end = 6.0
                    + if kind == DragAction::ResizeLeft {
                        0.0
                    } else {
                        delta
                    };
                assert!(
                    (moved.start - expected_start).abs() < 1e-6,
                    "{kind:?}, fine={fine}: {moved:?}"
                );
                assert!((moved.end - expected_end).abs() < 1e-6);
                assert_eq!(frame(&mut model, vec![], true), moved);
                let final_pos = end + egui::vec2(8.0, 0.0);
                let refined = frame(&mut model, vec![egui::Event::PointerMoved(final_pos)], true);
                let fine_delta = 4.0 * 0.1 / geo.inner_rect.width() as f64 * 8.0;
                assert!(
                    (refined.start
                        - moved.start
                        - if kind == DragAction::ResizeRight {
                            0.0
                        } else {
                            fine_delta
                        })
                    .abs()
                        < 1e-6
                );
                assert!(
                    (refined.end
                        - moved.end
                        - if kind == DragAction::ResizeLeft {
                            0.0
                        } else {
                            fine_delta
                        })
                    .abs()
                        < 1e-6
                );
                assert_eq!(
                    frame(&mut model, vec![button(final_pos, false, true)], true),
                    refined
                );
            }
        }
    }
}
