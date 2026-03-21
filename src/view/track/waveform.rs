use crate::{
    audio::{
        self,
        sample::view::{SINGLE_SAMPLE_DRAW_MAX_SPP, ViewData},
    },
    model::{
        Action, Model,
        config::ThemeColors,
        ruler::{TickType, ValueLattice, sample_value_to_screen_y},
        track::TrackId,
    },
    rect::Rect,
    view::{
        track::{hover, selection},
        util::rpc,
        value_ruler2::NR_PIXELS_PER_VALUE_TICK,
    },
};
use anyhow::Result;

pub fn ui_waveform_canvas(
    ui: &mut egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    rect: egui::Rect,
    theme_colors: &ThemeColors,
) -> Result<()> {
    let size = ui.available_size();
    ui.set_max_size(size);
    ui.set_min_size(size);

    let bg_color = ui.visuals().extreme_bg_color;
    let stroke = ui.visuals().window_stroke();
    ui.painter().rect(rect, 0.0, bg_color, stroke);
    let waveform_response = ui.interact(
        rect,
        ui.id().with(("waveform_interaction", track_id)),
        egui::Sense::drag(),
    );
    handle_pan_drag(ui, model, track_id, &waveform_response);
    ui_waveform(ui, model, track_id, rect, theme_colors)?;
    hover::ui_hover(ui, model, track_id, theme_colors);
    selection::ui_selection(ui, model, &waveform_response, theme_colors);

    Ok(())
}

fn handle_pan_drag(
    ui: &mut egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    response: &egui::Response,
) {
    if response.dragged_by(egui::PointerButton::Secondary) {
        let (delta, modifiers) = ui.input(|i| (i.pointer.delta(), i.modifiers));
        if modifiers.ctrl {
            model.actions.push(Action::PanX {
                nr_pixels: -delta.x,
            });
            model.actions.push(Action::PanY {
                track_id,
                nr_pixels: delta.y,
            });
        } else if modifiers.shift {
            model.actions.push(Action::PanY {
                track_id,
                nr_pixels: delta.y,
            });
        } else {
            model.actions.push(Action::PanX {
                nr_pixels: -delta.x,
            });
        }
    }
}

fn ui_waveform(
    ui: &mut egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    rect: egui::Rect,
    theme_colors: &ThemeColors,
) -> Result<()> {
    let sample_ix_range = model
        .tracks
        .ruler
        .ix_range()
        .ok_or(anyhow::anyhow!("No time line"))?;
    let hover_info = model.tracks.hover_info;
    let display_scale = model.user_config.value_display_scale;
    let track = model
        .tracks
        .get_track_mut(track_id)
        .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;

    track.set_ix_range(sample_ix_range, &model.audio)?;
    track.set_screen_rect(rect.into());
    track.update_sample_view(&mut model.audio, display_scale)?;
    let sample_view = track.get_sample_view()?;

    let color = theme_colors.waveform;
    let line_color = color.linear_multiply(0.7);
    let screen_rect = track
        .screen_rect
        .ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
    let sample_rect = track
        .single
        .item
        .sample_rect()
        .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;
    draw_value_grid(ui, sample_rect, screen_rect, display_scale);

    match sample_view.data {
        ViewData::Single(ref single_view) => {
            if sample_view.samples_per_pixel < SINGLE_SAMPLE_DRAW_MAX_SPP {
                single_view.samples.iter().for_each(|pos| {
                    let Some(val_rng) = sample_rect.val_rng() else {
                        return;
                    };
                    let Some(y_mid) =
                        sample_value_to_screen_y(0.0, val_rng, screen_rect, display_scale)
                    else {
                        return;
                    };
                    let pos_mid = crate::Pos { x: pos.x, y: y_mid };
                    let is_hovered = hover_info.sample_pos_is_hovered(pos.x.into());
                    let stroke_width = if is_hovered { 2.0 } else { 1.0 };

                    let color = if is_hovered {
                        theme_colors.waveform_hovered_sample
                    } else {
                        theme_colors.waveform
                    };
                    let line_color = color.linear_multiply(0.7);

                    if pos.y < screen_rect.top() && pos_mid.y < screen_rect.top()
                        || pos.y > screen_rect.bottom() && pos_mid.y > screen_rect.bottom()
                    {
                        return;
                    }

                    let pos_mid = screen_rect.clip_pos(pos_mid);
                    let pos_mid = rpc(ui, pos_mid.into());

                    let mut pos = *pos;

                    if screen_rect.contains(pos) {
                        let circle_size = if sample_view.samples_per_pixel < 1.0 / 16.0 {
                            3.0
                        } else {
                            2.0
                        };
                        // Use pos centerd x so it aligns with the line that is drawn on a pixel
                        // column exactly.
                        let pos_centered = rpc(ui, pos.into());
                        let pos_centered_x = egui::pos2(pos_centered.x, pos.y);
                        ui.painter()
                            .circle_filled(pos_centered_x, circle_size, color);
                    } else {
                        pos = screen_rect.clip_pos(pos);
                    };

                    let pos = rpc(ui, pos.into());

                    ui.painter().line_segment(
                        [pos_mid, pos],
                        egui::Stroke::new(stroke_width, line_color.linear_multiply(0.5)),
                    );
                });
            } else {
                single_view.line_segments.iter().for_each(|segment| {
                    let positions = segment.iter().map(|pos| rpc(ui, pos.into())).collect();
                    ui.painter()
                        .line(positions, egui::Stroke::new(1.0, line_color));
                });
            }
        }
        ViewData::MinMax(ref mix_max_positions) => {
            mix_max_positions.iter().for_each(|pos| {
                let min = rpc(ui, (&pos.min).into());
                let max = rpc(ui, (&pos.max).into());
                if !screen_rect.contains(min.into()) && !screen_rect.contains(max.into()) {
                    return;
                }
                let color = theme_colors.waveform;
                ui.painter()
                    .line_segment([min, max], egui::Stroke::new(1.0, color));
            });
        }
    };

    Ok(())
}

fn draw_value_grid(
    ui: &mut egui::Ui,
    sample_rect: audio::SampleRect,
    screen_rect: Rect,
    display_scale: crate::model::ruler::ValueDisplayScale,
) {
    let Some(val_rng) = sample_rect.val_rng() else {
        return;
    };
    let mut lattice = ValueLattice::default();
    if lattice
        .compute_ticks(
            val_rng,
            screen_rect,
            NR_PIXELS_PER_VALUE_TICK,
            display_scale,
        )
        .is_err()
    {
        return;
    }

    let zero_stroke = egui::Stroke::new(1.0, ui.style().visuals.text_color().linear_multiply(0.55));
    let grid_stroke = egui::Stroke::new(
        1.0,
        ui.style()
            .visuals
            .widgets
            .noninteractive
            .bg_stroke
            .color
            .linear_multiply(0.7),
    );

    for tick in &lattice.ticks {
        // The waveform background uses a much coarser grid than the ruler on purpose.
        // We keep only the major lines plus zero so the waveform stays readable.
        if tick.sample_value != 0.0 && tick.tick_type != TickType::Big {
            continue;
        }
        let left = rpc(ui, egui::pos2(screen_rect.left(), tick.screen_y));
        let right = rpc(ui, egui::pos2(screen_rect.right(), tick.screen_y));
        // Zero is highlighted slightly more than the other guides because it carries semantic
        // meaning for audio signals, while the remaining grid lines should stay unobtrusive.
        let stroke = if tick.sample_value == 0.0 {
            zero_stroke
        } else {
            grid_stroke
        };
        ui.painter().line_segment([left, right], stroke);
    }
}
