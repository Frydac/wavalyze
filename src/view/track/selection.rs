use crate::{
    audio::sample,
    model::{
        Action, Model,
        config::ThemeColors,
        selection_info::{SelectionInfo, SelectionInfoE},
        track::TrackId,
    },
};

const SELECTION_EDGE_HIT_RADIUS_PX: f32 = 8.0;
const SELECTION_RESIZE_CURSOR: egui::CursorIcon = egui::CursorIcon::ResizeColumn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SelectionResizeEdge {
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, Default)]
struct SelectionResizeState {
    active_edge: SelectionResizeEdge,
    anchor_sample_ix: sample::Ix,
    anchor_boundary: sample::Ix,
}

use crate::model::selection_info::snapped_selection_range;

fn set_snapped_selection(model: &mut Model, anchor: f64, current: f64, toward_left: bool) {
    let ix_rng = snapped_selection_range(anchor, current, model.block_size, toward_left);
    let screen_x_start = model
        .tracks
        .sample_ix_to_screen_x(ix_rng.start as f64 - 0.1)
        .unwrap_or(0.0);
    let screen_x_end = model
        .tracks
        .sample_ix_to_screen_x(ix_rng.end as f64 - 0.1)
        .unwrap_or(screen_x_start);
    model
        .actions
        .push(Action::SetSelection(SelectionInfoE::IsSelected(
            SelectionInfo {
                ix_rng,
                screen_x_start,
                screen_x_end,
            },
        )));
}

fn selection_screen_x_range(
    model: &Model,
) -> Option<(sample::IxRange, std::ops::RangeInclusive<f32>)> {
    let SelectionInfoE::IsSelected(selection_info) = model.tracks.selection_info else {
        return None;
    };
    let ix_rng = selection_info.ix_rng;
    if ix_rng.end <= ix_rng.start {
        return None;
    }

    let left_x = model
        .tracks
        .sample_ix_to_screen_x((ix_rng.start as f64) - 0.1)?;
    let right_x = model
        .tracks
        .sample_ix_to_screen_x((ix_rng.end as f64) - 0.1)?;

    Some((ix_rng, left_x..=right_x))
}

fn hovered_selection_edge(
    ix_rng: sample::IxRange,
    screen_x_rng: std::ops::RangeInclusive<f32>,
    rect: egui::Rect,
    pointer_x: f32,
) -> Option<(SelectionResizeEdge, sample::Ix)> {
    let left_x = *screen_x_rng.start();
    let right_x = *screen_x_rng.end();
    let left_dist = selection_edge_hit_distance(rect, left_x, pointer_x);
    let right_dist = selection_edge_hit_distance(rect, right_x, pointer_x);

    if left_dist > SELECTION_EDGE_HIT_RADIUS_PX && right_dist > SELECTION_EDGE_HIT_RADIUS_PX {
        return None;
    }

    if left_dist <= right_dist {
        Some((SelectionResizeEdge::Left, ix_rng.end - 1))
    } else {
        Some((SelectionResizeEdge::Right, ix_rng.start))
    }
}

fn selection_edge_hit_distance(rect: egui::Rect, edge_x: f32, pointer_x: f32) -> f32 {
    if !(rect.left() - SELECTION_EDGE_HIT_RADIUS_PX..=rect.right() + SELECTION_EDGE_HIT_RADIUS_PX)
        .contains(&edge_x)
    {
        return f32::INFINITY;
    }

    (pointer_x - edge_x).abs()
}

fn hovered_selection_edge_for_model(
    model: &Model,
    rect: egui::Rect,
    pointer_x: f32,
) -> Option<(SelectionResizeEdge, sample::Ix)> {
    let (ix_rng, screen_x_rng) = selection_screen_x_range(model)?;
    hovered_selection_edge(ix_rng, screen_x_rng, rect, pointer_x)
}

fn set_selection_from_drag(
    model: &mut Model,
    start_sample_ix: sample::Ix,
    current_sample_ix: sample::Ix,
    screen_x_start: f32,
    screen_x_end: f32,
) {
    let ix_rng = if screen_x_end < screen_x_start {
        (current_sample_ix..start_sample_ix + 1).into()
    } else {
        (start_sample_ix..current_sample_ix + 1).into()
    };

    let selection_info = SelectionInfoE::IsSelected(SelectionInfo {
        ix_rng,
        screen_x_start,
        screen_x_end,
    });
    model.actions.push(Action::SetSelection(selection_info));
}

fn whole_track_selection_range(
    nr_samples: usize,
    sample_ix_offset: f64,
) -> Option<sample::IxRange> {
    if nr_samples == 0 {
        return None;
    }

    let offset = sample_ix_offset.round() as sample::Ix;
    let end = (nr_samples as sample::Ix).checked_sub(offset)?;
    Some((offset.checked_neg()?..end).into())
}

fn set_selection_to_whole_track(model: &mut Model, track_id: TrackId) {
    let Some(track) = model.tracks.get_track(track_id) else {
        return;
    };
    let Ok(buffer) = model.audio.get_buffer(track.single.buffer_id) else {
        return;
    };
    let Some(ix_rng) =
        whole_track_selection_range(buffer.nr_samples(), track.single.sample_ix_offset)
    else {
        return;
    };

    let screen_x_start = model
        .tracks
        .sample_ix_to_screen_x((ix_rng.start as f64) - 0.1)
        .unwrap_or(0.0);
    let screen_x_end = model
        .tracks
        .sample_ix_to_screen_x((ix_rng.end as f64) - 0.1)
        .unwrap_or(screen_x_start);
    model
        .actions
        .push(Action::SetSelection(SelectionInfoE::IsSelected(
            SelectionInfo {
                ix_rng,
                screen_x_start,
                screen_x_end,
            },
        )));
}

fn ui_selection_interaction(
    ui: &egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    waveform_rect: egui::Rect,
    response: &egui::Response,
) {
    let selection_resize_state_id = response.id.with("selection_resize_state");
    let modifiers = ui.input(|i| i.modifiers);
    let hover_pos = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|&pos| waveform_rect.contains(pos));
    let hover_edge = if modifiers.shift {
        hover_pos.and_then(|pos| hovered_selection_edge_for_model(model, waveform_rect, pos.x))
    } else {
        None
    };

    if hover_edge.is_some() {
        ui.ctx().set_cursor_icon(SELECTION_RESIZE_CURSOR);
    }

    if response.double_clicked() {
        ui.data_mut(|data| {
            data.remove_temp::<SelectionResizeState>(selection_resize_state_id);
        });
        set_selection_to_whole_track(model, track_id);
        return;
    }

    let primary_down = ui.input(|i| i.pointer.primary_down());
    if !primary_down {
        ui.data_mut(|data| {
            data.remove_temp::<SelectionResizeState>(selection_resize_state_id);
        });
        return;
    }

    let pressed_on_widget = response.is_pointer_button_down_on();
    let primary_pressed = ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary));
    if primary_pressed && pressed_on_widget {
        if let Some((edge, anchor_sample_ix)) =
            ui.input(|i| i.pointer.press_origin())
                .and_then(|press_origin| {
                    modifiers.shift.then_some(press_origin).and_then(|origin| {
                        hovered_selection_edge_for_model(model, waveform_rect, origin.x)
                    })
                })
        {
            ui.data_mut(|data| {
                data.insert_temp(
                    selection_resize_state_id,
                    SelectionResizeState {
                        active_edge: edge,
                        anchor_sample_ix,
                        anchor_boundary: if edge == SelectionResizeEdge::Left {
                            anchor_sample_ix.saturating_add(1)
                        } else {
                            anchor_sample_ix
                        },
                    },
                );
            });
        } else {
            ui.data_mut(|data| {
                data.remove_temp::<SelectionResizeState>(selection_resize_state_id);
            });
        }
    }

    if !pressed_on_widget {
        return;
    }

    let current_pos = response
        .interact_pointer_pos()
        .or_else(|| ui.input(|i| i.pointer.latest_pos()));
    let press_origin = ui.input(|i| i.pointer.press_origin());

    let (Some(current_pos), Some(press_origin)) = (current_pos, press_origin) else {
        return;
    };

    let Some(current_sample_ix) = model.tracks.screen_x_to_sample_ix(current_pos.x) else {
        return;
    };
    let Some(start_sample_ix) = model.tracks.screen_x_to_sample_ix(press_origin.x) else {
        return;
    };

    if model.user_config.blocks.enabled && model.user_config.blocks.snap_selection {
        if let Some(state) =
            ui.data(|data| data.get_temp::<SelectionResizeState>(selection_resize_state_id))
        {
            let anchor = state.anchor_boundary as f64;
            let toward_left = current_sample_ix < anchor
                || (current_sample_ix == anchor && state.active_edge == SelectionResizeEdge::Left);
            set_snapped_selection(model, anchor, current_sample_ix, toward_left);
            ui.ctx().set_cursor_icon(SELECTION_RESIZE_CURSOR);
        } else {
            set_snapped_selection(
                model,
                start_sample_ix,
                current_sample_ix,
                current_sample_ix < start_sample_ix,
            );
        }
        return;
    }

    let current_sample_ix = current_sample_ix.round() as sample::Ix;
    let start_sample_ix = start_sample_ix.round() as sample::Ix;

    if let Some(mut resize_state) =
        ui.data(|data| data.get_temp::<SelectionResizeState>(selection_resize_state_id))
    {
        resize_state.active_edge = if current_sample_ix < resize_state.anchor_sample_ix {
            SelectionResizeEdge::Left
        } else {
            SelectionResizeEdge::Right
        };
        ui.data_mut(|data| {
            data.insert_temp(selection_resize_state_id, resize_state);
        });

        let start_x = model
            .tracks
            .sample_ix_to_screen_x((resize_state.anchor_sample_ix as f64) - 0.1)
            .unwrap_or(press_origin.x);
        set_selection_from_drag(
            model,
            resize_state.anchor_sample_ix,
            current_sample_ix,
            start_x,
            current_pos.x,
        );
        ui.ctx().set_cursor_icon(SELECTION_RESIZE_CURSOR);
        return;
    }

    set_selection_from_drag(
        model,
        start_sample_ix,
        current_sample_ix,
        press_origin.x,
        current_pos.x,
    );
}

pub fn ui_selection(
    ui: &mut egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    waveform_rect: egui::Rect,
    response: &egui::Response,
    interaction_blocked: bool,
    theme_colors: &ThemeColors,
) {
    if !interaction_blocked {
        ui_selection_interaction(ui, model, track_id, waveform_rect, response);
    }

    let Some((_sel_ix_rng, screen_x_rng)) = selection_screen_x_range(model) else {
        return;
    };

    let fill_top = (waveform_rect.top() + 1.0).min(waveform_rect.bottom());
    let rect = egui::Rect::from_x_y_ranges(
        *screen_x_rng.start()..=(*screen_x_rng.end()).max(*screen_x_rng.start() + 1.0),
        fill_top..=waveform_rect.bottom(),
    );
    let rect = rect.intersect(waveform_rect);
    ui.painter().rect(
        rect,
        0.0,
        theme_colors.waveform_selection_fill,
        egui::Stroke::NONE,
        egui::epaint::StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::snapped_selection_range;
    use super::{SelectionResizeEdge, hovered_selection_edge, whole_track_selection_range};

    #[test]
    fn block_drag_rounds_both_boundaries_in_either_direction() {
        assert_eq!(
            snapped_selection_range(100.0, 1900.0, 1024, false),
            (0..2048).into()
        );
        assert_eq!(
            snapped_selection_range(1900.0, 100.0, 1024, true),
            (0..2048).into()
        );
        assert_eq!(
            snapped_selection_range(0.0, 1535.9, 1024, false),
            (0..1024).into()
        );
        assert_eq!(
            snapped_selection_range(0.0, 1536.0, 1024, false),
            (0..2048).into()
        );
        assert_eq!(
            snapped_selection_range(0.0, -1536.0, 1024, true),
            (-2048..0).into()
        );
    }

    #[test]
    fn block_drag_keeps_one_block_when_boundaries_coincide() {
        assert_eq!(
            snapped_selection_range(1024.0, 1100.0, 1024, false),
            (1024..2048).into()
        );
        assert_eq!(
            snapped_selection_range(1024.0, 1000.0, 1024, true),
            (0..1024).into()
        );
    }

    #[test]
    fn block_resize_preserves_anchor_and_allows_crossing_it() {
        // Left resize anchors at the exclusive end; right resize anchors at the start.
        assert_eq!(
            snapped_selection_range(2048.0, 900.0, 1024, true),
            (1024..2048).into()
        );
        assert_eq!(
            snapped_selection_range(1024.0, 2900.0, 1024, false),
            (1024..3072).into()
        );
        assert_eq!(
            snapped_selection_range(2048.0, 3100.0, 1024, false),
            (2048..3072).into()
        );
        assert_eq!(
            snapped_selection_range(1024.0, -100.0, 1024, true),
            (0..1024).into()
        );
    }

    #[test]
    fn block_drag_handles_extreme_sizes_and_positions() {
        for size in [0, 1, 1024, u64::MAX] {
            for position in [i64::MIN as f64, i64::MAX as f64] {
                let range = snapped_selection_range(position, position, size, false);
                assert!(range.start < range.end);
                let size = size.max(1).min(i64::MAX as u64) as i64;
                assert_eq!(range.start % size, 0);
                assert_eq!(range.end % size, 0);
            }
        }
    }

    #[test]
    fn hovered_selection_edge_keeps_visible_left_edge_draggable() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 20.0));

        let edge = hovered_selection_edge((10..20).into(), 4.0..=40.0, rect, 5.0);

        assert_eq!(edge, Some((SelectionResizeEdge::Left, 19)));
    }

    #[test]
    fn hovered_selection_edges_are_draggable_on_waveform_boundaries() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 20.0));

        let left = hovered_selection_edge((10..20).into(), 0.0..=40.0, rect, 0.0);
        let right = hovered_selection_edge((10..20).into(), 20.0..=100.0, rect, 100.0);

        assert_eq!(left, Some((SelectionResizeEdge::Left, 19)));
        assert_eq!(right, Some((SelectionResizeEdge::Right, 10)));
    }

    #[test]
    fn hovered_selection_edges_accept_the_exact_outside_hit_radius() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 20.0));

        let left = hovered_selection_edge((10..20).into(), -8.0..=40.0, rect, 0.0);
        let right = hovered_selection_edge((10..20).into(), 20.0..=108.0, rect, 100.0);

        assert_eq!(left, Some((SelectionResizeEdge::Left, 19)));
        assert_eq!(right, Some((SelectionResizeEdge::Right, 10)));
    }

    #[test]
    fn hovered_selection_edges_reject_just_beyond_outside_hit_radius() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 20.0));

        let left = hovered_selection_edge((10..20).into(), -8.01..=40.0, rect, 0.0);
        let right = hovered_selection_edge((10..20).into(), 20.0..=108.01, rect, 100.0);

        assert_eq!(left, None);
        assert_eq!(right, None);
    }

    #[test]
    fn hovered_selection_edge_keeps_left_edge_draggable_just_outside_rect() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 20.0));

        let edge = hovered_selection_edge((10..20).into(), -3.0..=40.0, rect, 0.0);

        assert_eq!(edge, Some((SelectionResizeEdge::Left, 19)));
    }

    #[test]
    fn hovered_selection_edge_keeps_right_edge_draggable_just_outside_rect() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 20.0));

        let edge = hovered_selection_edge((10..20).into(), 20.0..=103.0, rect, 100.0);

        assert_eq!(edge, Some((SelectionResizeEdge::Right, 10)));
    }

    #[test]
    fn hovered_selection_edge_rejects_edges_too_far_outside_rect() {
        let rect = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 20.0));

        let edge = hovered_selection_edge((10..20).into(), -9.0..=40.0, rect, 0.0);

        assert_eq!(edge, None);
    }

    #[test]
    fn whole_track_selection_range_uses_zero_offset() {
        assert_eq!(whole_track_selection_range(12, 0.0), Some((0..12).into()));
    }

    #[test]
    fn whole_track_selection_range_shifts_left_for_positive_offset() {
        assert_eq!(whole_track_selection_range(12, 3.0), Some((-3..9).into()));
    }

    #[test]
    fn whole_track_selection_range_shifts_right_for_negative_offset() {
        assert_eq!(whole_track_selection_range(12, -3.0), Some((3..15).into()));
    }

    #[test]
    fn whole_track_selection_range_rejects_empty_buffers() {
        assert_eq!(whole_track_selection_range(0, 0.0), None);
    }
}
