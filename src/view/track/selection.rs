use crate::{
    audio::sample,
    model::{
        Action, Model,
        selection_info::{SelectionInfo, SelectionInfoE},
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
    model: &Model,
    rect: egui::Rect,
    pointer_x: f32,
) -> Option<(SelectionResizeEdge, sample::Ix)> {
    let (ix_rng, screen_x_rng) = selection_screen_x_range(model)?;
    let left_x = *screen_x_rng.start();
    let right_x = *screen_x_rng.end();
    let left_dist = if (rect.left()..=rect.right()).contains(&left_x) {
        (pointer_x - left_x).abs()
    } else {
        f32::INFINITY
    };
    let right_dist = if (rect.left()..=rect.right()).contains(&right_x) {
        (pointer_x - right_x).abs()
    } else {
        f32::INFINITY
    };

    if left_dist > SELECTION_EDGE_HIT_RADIUS_PX && right_dist > SELECTION_EDGE_HIT_RADIUS_PX {
        return None;
    }

    if left_dist <= right_dist {
        Some((SelectionResizeEdge::Left, ix_rng.end - 1))
    } else {
        Some((SelectionResizeEdge::Right, ix_rng.start))
    }
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

fn ui_selection_interaction(ui: &egui::Ui, model: &mut Model, response: &egui::Response) {
    let selection_resize_state_id = response.id.with("selection_resize_state");
    let modifiers = ui.input(|i| i.modifiers);
    let hover_pos = ui
        .ctx()
        .pointer_hover_pos()
        .filter(|&pos| ui.min_rect().contains(pos));
    let hover_edge = if modifiers.shift {
        hover_pos.and_then(|pos| hovered_selection_edge(model, ui.min_rect(), pos.x))
    } else {
        None
    };

    if hover_edge.is_some() {
        ui.ctx().set_cursor_icon(SELECTION_RESIZE_CURSOR);
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
                    modifiers
                        .shift
                        .then_some(press_origin)
                        .and_then(|origin| hovered_selection_edge(model, ui.min_rect(), origin.x))
                })
        {
            ui.data_mut(|data| {
                data.insert_temp(
                    selection_resize_state_id,
                    SelectionResizeState {
                        active_edge: edge,
                        anchor_sample_ix,
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

    let Some(current_sample_ix) = model.tracks.ruler.screen_x_to_sample_ix(current_pos.x) else {
        return;
    };
    let Some(start_sample_ix) = model.tracks.ruler.screen_x_to_sample_ix(press_origin.x) else {
        return;
    };

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
            .ruler
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

pub fn ui_selection(ui: &mut egui::Ui, model: &mut Model, response: &egui::Response) {
    ui_selection_interaction(ui, model, response);

    let Some((_sel_ix_rng, screen_x_rng)) = selection_screen_x_range(model) else {
        return;
    };

    let rect = ui.min_rect();
    let rect = egui::Rect::from_x_y_ranges(
        *screen_x_rng.start()..=(*screen_x_rng.end()).max(*screen_x_rng.start() + 1.0),
        (rect.top() + 1.0)..=rect.bottom(),
    );
    let rect = rect.intersect(ui.min_rect());
    ui.painter().rect(
        rect,
        0.0,
        egui::Color32::LIGHT_GRAY.linear_multiply(0.05),
        egui::Stroke::NONE,
    );
}
