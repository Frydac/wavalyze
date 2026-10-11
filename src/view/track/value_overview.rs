//! Full-scale reference for a track's current value window. Reads the existing range;
//! no separate navigation state is needed. The reference is linear in sample values,
//! including when the waveform uses a skewed display within its visible window.

use crate::audio::sample::ValRange;
use crate::model::{Action, track::TrackId};

const RELATIVE_RESIZE_THRESHOLD_HEIGHT: f32 = 18.0;
const MIN_VISIBLE_THUMB_HEIGHT: f32 = 2.0;
const MIN_THUMB_HIT_HEIGHT: f32 = 24.0;
const EDGE_HIT_HEIGHT: f32 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragKind {
    Pan,
    Top,
    Bottom,
}

impl DragKind {
    fn cursor(self, dragging: bool) -> egui::CursorIcon {
        match self {
            Self::Pan if dragging => egui::CursorIcon::Grabbing,
            Self::Pan => egui::CursorIcon::Grab,
            Self::Top | Self::Bottom => egui::CursorIcon::ResizeRow,
        }
    }
}

/// Reference range and scale remain fixed throughout a gesture. Accumulating movement
/// allows Shift to be toggled while dragging without moving the range suddenly.
#[derive(Debug, Clone, Copy)]
struct ValueDrag {
    range: ValRange<f64>,
    kind: DragKind,
    units_per_pixel: f64,
    fine_units_per_pixel: f64,
    previous_y: f32,
    delta: f64,
}

impl ValueDrag {
    fn new(range: ValRange<f64>, geometry: Geometry, kind: DragKind, origin_y: f32) -> Self {
        let span = range.max - range.min;
        let full_scale_unit = 2.0 / geometry.inner.height() as f64;
        let units_per_pixel = if kind != DragKind::Pan
            && geometry.viewport.height() < RELATIVE_RESIZE_THRESHOLD_HEIGHT
        {
            full_scale_unit.min(span / 40.0)
        } else {
            full_scale_unit
        };
        Self {
            range,
            kind,
            units_per_pixel,
            fine_units_per_pixel: units_per_pixel.min(span / geometry.inner.height() as f64) * 0.1,
            previous_y: origin_y,
            delta: 0.0,
        }
    }

    fn move_to(&mut self, y: f32, fine: bool) -> Option<ValRange<f64>> {
        let unit = if fine {
            self.fine_units_per_pixel
        } else {
            self.units_per_pixel
        };
        self.delta -= (y - self.previous_y) as f64 * unit;
        self.previous_y = y;
        let span = self.range.max - self.range.min;
        // Prevent inverted/collapsed windows, including rounding at large offsets.
        let minimum_span = (span * 1e-6).max(
            self.range
                .min
                .abs()
                .max(self.range.max.abs())
                .max(f64::MIN_POSITIVE)
                * f64::EPSILON
                * 8.0,
        );
        let range = match self.kind {
            DragKind::Pan => ValRange {
                min: self.range.min + self.delta,
                max: self.range.max + self.delta,
            },
            DragKind::Top => ValRange {
                min: self.range.min,
                max: (self.range.max + self.delta).max(self.range.min + minimum_span),
            },
            DragKind::Bottom => ValRange {
                min: (self.range.min + self.delta).min(self.range.max - minimum_span),
                max: self.range.max,
            },
        };
        valid_range(range).then_some(range)
    }
}

fn valid_range(range: ValRange<f64>) -> bool {
    range.min.is_finite()
        && range.max.is_finite()
        && range.max > range.min
        && (range.max - range.min).is_finite()
}

fn zoom_range(
    range: ValRange<f64>,
    scroll: f32,
    fine: bool,
    zoom_mult: f32,
) -> Option<ValRange<f64>> {
    let half_span = (range.max - range.min) * 0.5;
    let center = range.min + half_span;
    let exponent = (-scroll as f64 * zoom_mult as f64 * 0.00125 * if fine { 0.1 } else { 1.0 })
        .clamp(-20.0, 20.0);
    let half_span = half_span * exponent.exp();
    let zoomed = ValRange {
        min: center - half_span,
        max: center + half_span,
    };
    valid_range(zoomed).then_some(zoomed)
}

#[derive(Debug, Clone, Copy)]
struct Geometry {
    inner: egui::Rect,
    viewport: egui::Rect,
    /// A minimum-sized visual thumb; the numeric viewport remains true to scale.
    thumb: egui::Rect,
    overflow_top: bool,
    overflow_bottom: bool,
}

fn geometry(rect: egui::Rect, range: ValRange<f64>) -> Option<Geometry> {
    if rect.width() <= 2.0 || rect.height() <= 2.0 || !valid_range(range) {
        return None;
    }
    let inner = rect.shrink(1.0);
    let y =
        |value: f64| inner.top() + ((1.0 - value.clamp(-1.0, 1.0)) * 0.5) as f32 * inner.height();
    let viewport = egui::Rect::from_min_max(
        egui::pos2(inner.left(), y(range.max)),
        egui::pos2(inner.right(), y(range.min)),
    );
    let height = viewport
        .height()
        .max(MIN_VISIBLE_THUMB_HEIGHT)
        .min(inner.height());
    let top = (viewport.center().y - height * 0.5).clamp(inner.top(), inner.bottom() - height);
    let thumb = egui::Rect::from_min_size(
        egui::pos2(inner.left(), top),
        egui::vec2(inner.width(), height),
    );
    Some(Geometry {
        inner,
        viewport,
        thumb,
        overflow_top: range.max > 1.0,
        overflow_bottom: range.min < -1.0,
    })
}

pub(super) fn ui(
    ui: &egui::Ui,
    rect: egui::Rect,
    range: Option<ValRange<f64>>,
    accent: egui::Color32,
    track_id: TrackId,
    actions: &mut Vec<Action>,
    zoom_mult: f32,
) {
    if !rect.is_positive() {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let visuals = ui.visuals();
    painter.rect(
        rect,
        2.0,
        visuals.extreme_bg_color,
        visuals.widgets.noninteractive.bg_stroke,
        egui::epaint::StrokeKind::Inside,
    );
    let Some(geometry) = range.and_then(|range| geometry(rect, range)) else {
        return;
    };
    handle_interaction(
        ui,
        rect,
        geometry,
        range.unwrap(),
        track_id,
        actions,
        zoom_mult,
    );
    let color = accent.gamma_multiply(0.55);
    painter.rect(
        geometry.thumb,
        1.0,
        accent.gamma_multiply(0.12),
        egui::Stroke::new(1.0, color),
        egui::epaint::StrokeKind::Inside,
    );
    painter.hline(
        geometry.inner.x_range(),
        geometry.inner.center().y,
        visuals.widgets.noninteractive.bg_stroke,
    );
    let marker_size = 3.0_f32
        .min(geometry.inner.width() * 0.5)
        .min(geometry.inner.height() * 0.5);
    for (overflow, tip_y, direction) in [
        (geometry.overflow_top, geometry.inner.top(), 1.0),
        (geometry.overflow_bottom, geometry.inner.bottom(), -1.0),
    ] {
        if overflow {
            let x = geometry.inner.center().x;
            painter.add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(x, tip_y),
                    egui::pos2(x - marker_size, tip_y + direction * marker_size),
                    egui::pos2(x + marker_size, tip_y + direction * marker_size),
                ],
                color,
                egui::Stroke::NONE,
            ));
        }
    }
}

/// Invisible hit zones stay usable when the visible thumb is only two pixels tall.
/// Keep a center area for panning between the enlarged edge handles.
fn drag_kind_at(rect: egui::Rect, geometry: Geometry, pos: egui::Pos2) -> Option<DragKind> {
    let height = geometry
        .thumb
        .height()
        .max(MIN_THUMB_HIT_HEIGHT)
        .min(rect.height());
    let top = (geometry.thumb.center().y - height * 0.5).clamp(rect.top(), rect.bottom() - height);
    let hit = egui::Rect::from_min_size(
        egui::pos2(rect.left(), top),
        egui::vec2(rect.width(), height),
    );
    if !hit.contains(pos) {
        return None;
    }
    let edge = EDGE_HIT_HEIGHT.min(height / 3.0);
    Some(if pos.y < hit.top() + edge {
        DragKind::Top
    } else if pos.y > hit.bottom() - edge {
        DragKind::Bottom
    } else {
        DragKind::Pan
    })
}

fn handle_interaction(
    ui: &egui::Ui,
    rect: egui::Rect,
    geometry: Geometry,
    range: ValRange<f64>,
    track_id: TrackId,
    actions: &mut Vec<Action>,
    zoom_mult: f32,
) {
    let id = ui.id().with(("value_overview", track_id));
    let response = ui.interact(rect, id, egui::Sense::click_and_drag());
    let format_limit = |value: f64| {
        if value != 0.0 && value.abs() < 1e-5 {
            format!("{value:.6e}")
        } else {
            format!("{value:.8}")
        }
    };
    response.clone().on_hover_text(format!(
        "Value range: {} … {}\nZoom: {:.3}×\nDrag center to pan; drag edges to resize\nWheel to zoom; Shift for fine adjustment\nDouble-click to reset",
        format_limit(range.min), format_limit(range.max), 2.0 / (range.max - range.min),
    ));
    if response.double_clicked() {
        ui.ctx().data_mut(|data| data.remove::<ValueDrag>(id));
        actions.push(Action::RecenterY { track_id });
        return;
    }
    if response.drag_started() {
        ui.ctx().data_mut(|data| data.remove::<ValueDrag>(id));
        if let Some(origin) = ui.input(|input| input.pointer.press_origin())
            && let Some(kind) = drag_kind_at(rect, geometry, origin)
        {
            ui.ctx().data_mut(|data| {
                data.insert_temp(id, ValueDrag::new(range, geometry, kind, origin.y))
            });
        }
    }
    let drag = ui.ctx().data(|data| data.get_temp::<ValueDrag>(id));
    if (response.dragged() || response.drag_stopped())
        && let Some(mut drag) = drag
        && let Some(pos) = response.interact_pointer_pos()
    {
        ui.ctx().set_cursor_icon(drag.kind.cursor(true));
        if let Some(range) = drag.move_to(pos.y, ui.input(|input| input.modifiers.shift)) {
            actions.push(Action::SetTrackValueRange { track_id, range });
        }
        ui.ctx().data_mut(|data| data.insert_temp(id, drag));
    } else if response.hovered() {
        if let Some(kind) = response
            .hover_pos()
            .and_then(|pos| drag_kind_at(rect, geometry, pos))
        {
            ui.ctx().set_cursor_icon(kind.cursor(false));
        }
        // Consume the wheel so the enclosing track list does not scroll while zooming.
        let (scroll, fine) = ui.input_mut(|input| {
            let delta = input.smooth_scroll_delta;
            input.smooth_scroll_delta = egui::Vec2::ZERO;
            (
                if delta.y != 0.0 { delta.y } else { delta.x },
                input.modifiers.shift,
            )
        });
        if scroll != 0.0
            && let Some(range) = zoom_range(range, scroll, fine, zoom_mult)
        {
            actions.push(Action::SetTrackValueRange { track_id, range });
        }
    }
    if response.drag_stopped() || !ui.input(|input| input.pointer.primary_down()) {
        ui.ctx().data_mut(|data| data.remove::<ValueDrag>(id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pointer_drags_choose_the_correct_edge_and_keep_its_anchor_after_release() {
        for top in [true, false] {
            let ctx = egui::Context::default();
            let mut keys = slotmap::SlotMap::<TrackId, ()>::with_key();
            let track_id = keys.insert(());
            let mut range = ValRange {
                min: -1.0,
                max: 1.0,
            };
            let mut actions = Vec::new();
            let mut time = 0.0;
            let mut frame = |events| {
                time += 0.1;
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(300.0, 300.0),
                        )),
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            super::ui(
                                ui,
                                rect(),
                                Some(range),
                                egui::Color32::WHITE,
                                track_id,
                                &mut actions,
                                4.0,
                            );
                        });
                    },
                );
                for action in std::mem::take(&mut actions) {
                    match action {
                        Action::SetTrackValueRange {
                            track_id: id,
                            range: next,
                        } => {
                            assert_eq!(id, track_id);
                            range = next;
                        }
                        other => panic!("unexpected action: {other:?}"),
                    }
                }
                (range, output.platform_output.cursor_icon)
            };
            let start = egui::pos2(
                rect().center().x,
                if top {
                    rect().top() + 3.0
                } else {
                    rect().bottom() - 3.0
                },
            );
            let button = |pos, pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            };
            frame(vec![]);
            let (_, cursor) = frame(vec![egui::Event::PointerMoved(start)]);
            assert_eq!(cursor, egui::CursorIcon::ResizeRow);
            frame(vec![button(start, true)]);
            let end = start + egui::vec2(0.0, if top { 10.0 } else { -10.0 });
            let (resized, cursor) = frame(vec![egui::Event::PointerMoved(end)]);
            assert_eq!(cursor, egui::CursorIcon::ResizeRow);
            if top {
                assert_eq!(resized.min, -1.0);
                assert!((resized.max - 0.8).abs() < 1e-12);
            } else {
                assert_eq!(resized.max, 1.0);
                assert!((resized.min + 0.8).abs() < 1e-12);
            }
            let (released, _) = frame(vec![button(end, false)]);
            assert_eq!(released, resized);
        }
    }

    #[test]
    fn edge_resizing_keeps_the_opposite_limit_fixed_and_prevents_crossing() {
        let range = ValRange {
            min: -0.5,
            max: 0.5,
        };
        let geometry = geometry(rect(), range).unwrap();
        let mut top = ValueDrag::new(range, geometry, DragKind::Top, 0.0);
        let resized = top.move_to(-10.0, false).unwrap();
        assert_eq!(resized.min, range.min);
        assert!((resized.max - 0.7).abs() < 1e-12);
        // Modifier changes at the same pointer position must not move the window.
        assert_eq!(top.move_to(-10.0, true).unwrap(), resized);
        let fine = top.move_to(-20.0, true).unwrap();
        assert!((fine.max - 0.71).abs() < 1e-12);
        assert_eq!(fine.min, range.min);
        let crossed = top.move_to(1000.0, false).unwrap();
        assert_eq!(crossed.min, range.min);
        assert!(crossed.max > crossed.min);

        let mut bottom = ValueDrag::new(range, geometry, DragKind::Bottom, 0.0);
        let resized = bottom.move_to(10.0, false).unwrap();
        assert_eq!(resized.max, range.max);
        assert!((resized.min + 0.7).abs() < 1e-12);
        let crossed = bottom.move_to(-1000.0, false).unwrap();
        assert_eq!(crossed.max, range.max);
        assert!(crossed.max > crossed.min);
    }

    #[test]
    fn subpixel_windows_have_distinct_edges_and_center_and_resize_relatively() {
        let range = ValRange {
            min: -1e-10,
            max: 1e-10,
        };
        let geometry = geometry(rect(), range).unwrap();
        let center = geometry.viewport.center();
        assert_eq!(drag_kind_at(rect(), geometry, center), Some(DragKind::Pan));
        // There is a larger center dead zone before either resize handle takes over.
        for delta in [-5.0, 5.0] {
            assert_eq!(
                drag_kind_at(rect(), geometry, center + egui::vec2(0.0, delta)),
                Some(DragKind::Pan)
            );
        }
        assert_eq!(
            drag_kind_at(rect(), geometry, center - egui::vec2(0.0, 8.0)),
            Some(DragKind::Top)
        );
        assert_eq!(
            drag_kind_at(rect(), geometry, center + egui::vec2(0.0, 8.0)),
            Some(DragKind::Bottom)
        );
        let mut drag = ValueDrag::new(range, geometry, DragKind::Top, center.y);
        let resized = drag.move_to(center.y - 10.0, false).unwrap();
        assert_eq!(resized.min, range.min);
        assert!((resized.max - 1.5e-10).abs() < 1e-20);
        let mut fine = ValueDrag::new(range, geometry, DragKind::Pan, center.y);
        let panned = fine.move_to(center.y + 10.0, true).unwrap();
        assert!((panned.min - range.min).abs() < 1e-11);
        assert!(((panned.max - panned.min) - (range.max - range.min)).abs() < 1e-20);
    }

    #[test]
    fn wheel_zoom_preserves_center_and_supports_precision_and_inversion() {
        let range = ValRange {
            min: 0.25,
            max: 0.75,
        };
        let zoom = zoom_range(range, 50.0, false, 4.0).unwrap();
        let fine = zoom_range(range, 50.0, true, 4.0).unwrap();
        assert!((zoom.min + zoom.max - 1.0).abs() < 1e-12);
        assert!(zoom.min > range.min && zoom.max < range.max);
        assert!(fine.min < zoom.min && fine.min > range.min);
        let inverted = zoom_range(range, 50.0, false, -4.0).unwrap();
        assert!(inverted.min < range.min && inverted.max > range.max);
        let extreme = ValRange {
            min: 1.0,
            max: 1.0 + f64::EPSILON,
        };
        assert!(zoom_range(extreme, 1e6, false, 4.0).is_none());
    }

    #[test]
    fn wheel_over_the_strip_zooms_and_consumes_scrolling_only_while_hovered() {
        let ctx = egui::Context::default();
        let mut keys = slotmap::SlotMap::<TrackId, ()>::with_key();
        let track_id = keys.insert(());
        let mut range = ValRange {
            min: -0.5,
            max: 0.5,
        };
        let mut actions = Vec::new();
        let mut remaining = egui::Vec2::ZERO;
        let mut time = 0.0;
        let mut frame = |pos: egui::Pos2, fine: bool, wheel: bool| {
            time += 0.1;
            let modifiers = egui::Modifiers {
                shift: fine,
                ..Default::default()
            };
            let mut events = vec![egui::Event::PointerMoved(pos)];
            if wheel {
                events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    phase: egui::TouchPhase::Move,
                    delta: egui::vec2(0.0, 50.0),
                    modifiers,
                });
            }
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(300.0, 300.0),
                    )),
                    time: Some(time),
                    modifiers,
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        super::ui(
                            ui,
                            rect(),
                            Some(range),
                            egui::Color32::WHITE,
                            track_id,
                            &mut actions,
                            4.0,
                        );
                        remaining = ui.input(|input| input.smooth_scroll_delta);
                    });
                },
            );
            for action in std::mem::take(&mut actions) {
                match action {
                    Action::SetTrackValueRange {
                        track_id: id,
                        range: next,
                    } => {
                        assert_eq!(id, track_id);
                        range = next;
                    }
                    other => panic!("unexpected overview action: {other:?}"),
                }
            }
            (range, remaining)
        };
        frame(rect().center(), false, false);
        let (normal, remaining) = frame(rect().center(), false, true);
        assert!(normal.max < 0.5 && normal.min > -0.5);
        assert_eq!(remaining, egui::Vec2::ZERO);
        let (fine, remaining) = frame(rect().center(), true, true);
        assert!(fine.max < normal.max);
        assert!(normal.max - fine.max < 0.5 - normal.max);
        assert_eq!(remaining, egui::Vec2::ZERO);
        let (outside, remaining) = frame(egui::pos2(200.0, 200.0), false, true);
        assert_eq!(outside, fine);
        assert_ne!(remaining, egui::Vec2::ZERO);
    }

    #[test]
    fn gestures_pan_from_the_original_window_and_double_click_resets() {
        let mut model = crate::model::Model::new();
        model.user_config.value_display_scale.skew_factor = 2.0;
        #[cfg(not(target_arch = "wasm32"))]
        model.files.changes.disable_watcher();
        let config = crate::wav::ReadConfigBytes::new(
            Some("overview.wav".into()),
            crate::wav::channel_mask::tests::wav_bytes(None, 1, &[]),
        );
        model
            .add_loaded_file(
                crate::wav::read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap(),
            )
            .unwrap();
        let track_id = model.tracks.tracks_order[0];
        let buffer_id = model.tracks.get_track(track_id).unwrap().single.buffer_id;
        let sample_rect =
            crate::audio::SampleRect::from_buffere(model.audio.get_buffer(buffer_id).unwrap());
        model
            .tracks
            .get_track_mut(track_id)
            .unwrap()
            .single
            .set_sample_rect(sample_rect);
        let initial = ValRange {
            min: -0.25,
            max: 0.25,
        };
        model
            .tracks
            .set_track_value_range(track_id, initial)
            .unwrap();
        let ctx = egui::Context::default();
        let mut time = 0.0;
        let mut frame = |model: &mut crate::model::Model, events| {
            time += 0.1;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(300.0, 300.0),
                    )),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let range = model
                            .tracks
                            .get_track(track_id)
                            .unwrap()
                            .single
                            .sample_rect_raw()
                            .unwrap()
                            .val_rng();
                        super::ui(
                            ui,
                            rect(),
                            range,
                            egui::Color32::WHITE,
                            track_id,
                            &mut model.actions,
                            model.user_config.navigation.zoom_y_mult(),
                        );
                    });
                },
            );
            for action in std::mem::take(&mut model.actions) {
                action.process(model).unwrap();
            }
        };
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let range = |model: &crate::model::Model| {
            model
                .tracks
                .get_track(track_id)
                .unwrap()
                .single
                .sample_rect_raw()
                .unwrap()
                .val_rng()
                .unwrap()
        };
        frame(&mut model, vec![]);
        let start = rect().center();
        frame(
            &mut model,
            vec![egui::Event::PointerMoved(start), button(start, true)],
        );
        let first = start + egui::vec2(0.0, 10.0);
        frame(&mut model, vec![egui::Event::PointerMoved(first)]);
        assert!((range(&model).min + 0.45).abs() < 1e-12);
        assert!((range(&model).max - 0.05).abs() < 1e-12);
        // Range now moved: the next frame must still measure from the initial window.
        let second = start + egui::vec2(0.0, 20.0);
        frame(&mut model, vec![egui::Event::PointerMoved(second)]);
        assert!((range(&model).min + 0.65).abs() < 1e-12);
        assert!((range(&model).max + 0.15).abs() < 1e-12);
        frame(&mut model, vec![button(second, false)]);
        assert!((range(&model).min + 0.65).abs() < 1e-12);
        assert!((range(&model).max + 0.15).abs() < 1e-12);
        let panned = range(&model);
        // Single click/drag outside the thumb must leave the range alone.
        let outside = egui::pos2(start.x, rect().top() + 5.0);
        frame(
            &mut model,
            vec![egui::Event::PointerMoved(outside), button(outside, true)],
        );
        frame(
            &mut model,
            vec![egui::Event::PointerMoved(outside + egui::vec2(0.0, 10.0))],
        );
        frame(
            &mut model,
            vec![button(outside + egui::vec2(0.0, 10.0), false)],
        );
        assert_eq!(range(&model), panned);
        // Double-click anywhere on the strip resets to full scale.
        for _ in 0..2 {
            frame(
                &mut model,
                vec![egui::Event::PointerMoved(outside), button(outside, true)],
            );
            frame(&mut model, vec![button(outside, false)]);
        }
        assert_eq!(
            range(&model),
            ValRange {
                min: -1.0,
                max: 1.0
            }
        );
        assert_eq!(model.user_config.value_display_scale.skew_factor, 2.0);
    }

    fn rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(14.0, 102.0))
    }

    #[test]
    fn full_scale_and_zoomed_windows_map_to_the_full_scale_reference() {
        let full = geometry(
            rect(),
            ValRange {
                min: -1.0,
                max: 1.0,
            },
        )
        .unwrap();
        assert_eq!(full.viewport, full.inner);
        assert!(!full.overflow_top && !full.overflow_bottom);
        let zoom = geometry(
            rect(),
            ValRange {
                min: -0.25,
                max: 0.25,
            },
        )
        .unwrap();
        assert_eq!(zoom.viewport.height(), 25.0);
        assert_eq!(zoom.viewport.center(), full.inner.center());
        let panned = geometry(rect(), ValRange { min: 0.0, max: 0.5 }).unwrap();
        assert_eq!(panned.viewport.top(), 46.0);
        assert_eq!(panned.viewport.bottom(), full.inner.center().y);
    }

    #[test]
    fn outside_full_scale_is_clipped_and_indicated() {
        let above = geometry(rect(), ValRange { min: 1.2, max: 1.5 }).unwrap();
        assert_eq!(above.viewport.top(), above.inner.top());
        assert_eq!(above.viewport.height(), 0.0);
        assert!(above.overflow_top && !above.overflow_bottom);
        let below = geometry(
            rect(),
            ValRange {
                min: -2.0,
                max: -1.1,
            },
        )
        .unwrap();
        assert_eq!(below.viewport.bottom(), below.inner.bottom());
        assert!(below.overflow_bottom && !below.overflow_top);
        let both = geometry(
            rect(),
            ValRange {
                min: -2.0,
                max: 2.0,
            },
        )
        .unwrap();
        assert_eq!(both.viewport, both.inner);
        assert!(both.overflow_top && both.overflow_bottom);
    }

    #[test]
    fn tiny_ranges_keep_their_actual_extent() {
        let tiny = geometry(
            rect(),
            ValRange {
                min: 0.0,
                max: 1e-6,
            },
        )
        .unwrap();
        assert!(tiny.viewport.height() < 1.0);
        assert_eq!(tiny.thumb.height(), MIN_VISIBLE_THUMB_HEIGHT);
        assert_eq!(tiny.thumb.center(), tiny.viewport.center());
        assert_eq!(tiny.viewport.bottom(), tiny.inner.center().y);
    }

    #[test]
    fn minimum_visible_thumb_stays_inside_the_strip_at_extremes_and_on_short_tracks() {
        for range in [
            ValRange {
                min: 0.999999,
                max: 1.0,
            },
            ValRange {
                min: -1.0,
                max: -0.999999,
            },
            ValRange { min: 1.2, max: 1.5 },
            ValRange {
                min: -2.0,
                max: -1.1,
            },
        ] {
            let geometry = geometry(rect(), range).unwrap();
            assert_eq!(geometry.thumb.height(), MIN_VISIBLE_THUMB_HEIGHT);
            assert!(geometry.inner.contains_rect(geometry.thumb));
        }
        let short = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(14.0, 8.0));
        let geometry = geometry(
            short,
            ValRange {
                min: -1e-12,
                max: 1e-12,
            },
        )
        .unwrap();
        assert_eq!(geometry.thumb.height(), MIN_VISIBLE_THUMB_HEIGHT);
        assert!(geometry.inner.contains_rect(geometry.thumb));
    }

    #[test]
    fn invalid_ranges_and_tiny_rectangles_have_no_viewport() {
        for (min, max) in [
            (0.0, 0.0),
            (1.0, -1.0),
            (f64::NAN, 1.0),
            (-1.0, f64::INFINITY),
        ] {
            assert!(geometry(rect(), ValRange { min, max }).is_none());
        }
        let tiny = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(14.0, 2.0));
        assert!(
            geometry(
                tiny,
                ValRange {
                    min: -1.0,
                    max: 1.0
                }
            )
            .is_none()
        );
    }
}
