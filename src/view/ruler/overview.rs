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

use crate::model::{self, Action};

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

    let action = if left_response.as_ref().is_some_and(egui::Response::dragged) {
        Some(DragAction::ResizeLeft)
    } else if right_response.as_ref().is_some_and(egui::Response::dragged) {
        Some(DragAction::ResizeRight)
    } else if pan_response.as_ref().is_some_and(egui::Response::dragged) {
        Some(DragAction::Pan)
    } else {
        None
    };

    let Some(action) = action else {
        return;
    };

    let seconds_per_pixel = model.tracks.time_camera.seconds_per_pixel();
    if seconds_per_pixel <= 0.0 || geometry.inner_rect.width() <= 0.0 {
        return;
    }

    let delta_x = ui.input(|i| i.pointer.delta().x);
    if delta_x == 0.0 {
        return;
    }

    // Convert overview-pixels -> seconds -> ruler-pixels so existing navigation actions can be
    // reused. The sign/anchor choice makes the dragged overview edge follow the pointer.
    let delta_s = delta_x as f64 / geometry.inner_rect.width() as f64 * geometry.total_duration_s;
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
    })
}

fn draw_viewport(ui: &mut egui::Ui, model: &model::Model, viewport_rect: egui::Rect) {
    if viewport_rect.is_positive() {
        let accent = model.user_config.active_theme_colors(ui.visuals()).accent;
        ui.painter().rect(
            viewport_rect,
            2.0,
            accent.gamma_multiply(0.18),
            egui::Stroke::new(1.0, accent),
            egui::epaint::StrokeKind::Inside,
        );
    }
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
