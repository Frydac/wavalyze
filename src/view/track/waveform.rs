use crate::{
    audio::{
        self,
        sample::{
            self,
            view::{SINGLE_SAMPLE_DRAW_MAX_SPP, ViewData},
        },
    },
    model::{
        Action, Model,
        config::ThemeColors,
        hover_info::HoverInfoE,
        ruler::{ValueLattice, sample_value_to_screen_y},
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
    ui.painter().rect(
        rect,
        0.0,
        bg_color,
        stroke,
        egui::epaint::StrokeKind::Inside,
    );
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
    let screen_width = model.tracks.ruler.screen_rect().width() as f64;
    anyhow::ensure!(screen_width > 0.0, "Ruler screen rect width is zero");
    let time_range = model.tracks.time_camera.time_range(screen_width);
    let hover_info = model.tracks.hover_info;
    let display_scale = model.user_config.value_display_scale;
    let round_minmax_to_pixel_center = model.user_config.round_minmax_waveform_to_pixel_center;
    let track = model
        .tracks
        .get_track_mut(track_id)
        .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;
    let sample_ix_range = audio::sample::FracIxRange {
        start: crate::model::time_camera::time_to_sample_ix(time_range.start, track.sample_rate),
        end: crate::model::time_camera::time_to_sample_ix(time_range.end, track.sample_rate),
    };

    track.single.set_ix_range(sample_ix_range, &model.audio)?;
    track.set_screen_rect(rect.into());
    track.single.set_display_scale(display_scale);
    track.update(&mut model.audio)?;
    let sample_view = track.single.get_sample_view()?;

    let color = theme_colors.waveform;
    let line_color = color.linear_multiply(0.7);
    let screen_rect = track
        .screen_rect
        .ok_or_else(|| anyhow::anyhow!("screen_rect is missing"))?;
    let sample_rect = track
        .single
        .sample_rect()
        .ok_or_else(|| anyhow::anyhow!("sample_rect is missing"))?;

    draw_waveform(
        ui,
        WaveformDrawParams {
            sample_view,
            sample_rect,
            screen_rect,
            display_scale,
            round_minmax_to_pixel_center,
            hover_info: &hover_info,
            theme_colors,
        },
    );

    draw_value_grid(ui, sample_rect, screen_rect, display_scale);

    Ok(())
}

struct WaveformDrawParams<'a> {
    sample_view: &'a sample::View,
    sample_rect: audio::SampleRect,
    screen_rect: Rect,
    display_scale: crate::model::ruler::ValueDisplayScale,
    round_minmax_to_pixel_center: bool,
    hover_info: &'a HoverInfoE,
    theme_colors: &'a ThemeColors,
}

fn draw_waveform(ui: &mut egui::Ui, params: WaveformDrawParams<'_>) {
    match params.sample_view.data {
        ViewData::Single(ref single_view) => {
            if params.sample_view.samples_per_pixel < SINGLE_SAMPLE_DRAW_MAX_SPP {
                single_view.samples.iter().for_each(|pos| {
                    let Some(val_rng) = params.sample_rect.val_rng() else {
                        return;
                    };
                    let Some(y_mid) = sample_value_to_screen_y(
                        0.0,
                        val_rng,
                        params.screen_rect,
                        params.display_scale,
                    ) else {
                        return;
                    };
                    let pos_mid = crate::Pos { x: pos.x, y: y_mid };
                    let is_hovered = params.hover_info.sample_pos_is_hovered(
                        pos.x.into(),
                        params.sample_view.samples_per_pixel as f64,
                    );
                    let stroke_width = if is_hovered { 2.0 } else { 1.0 };

                    let color = if is_hovered {
                        params.theme_colors.waveform_hovered_sample
                    } else {
                        params.theme_colors.waveform
                    };
                    let line_color = color.linear_multiply(0.7);

                    if pos.y < params.screen_rect.top() && pos_mid.y < params.screen_rect.top()
                        || pos.y > params.screen_rect.bottom()
                            && pos_mid.y > params.screen_rect.bottom()
                    {
                        return;
                    }

                    let pos_mid = params.screen_rect.clip_pos(pos_mid);
                    let pos_mid = rpc(ui, pos_mid.into());

                    let mut pos = *pos;

                    if params.screen_rect.contains(pos) {
                        let circle_size = if params.sample_view.samples_per_pixel < 1.0 / 16.0 {
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
                        pos = params.screen_rect.clip_pos(pos);
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
                    let color = params.theme_colors.waveform;
                    let line_color = color.linear_multiply(0.7);
                    ui.painter()
                        .line(positions, egui::Stroke::new(1.0, line_color));
                });
            }
        }
        ViewData::MinMax(ref mix_max_positions) => {
            let stroke_width = minmax_column_stroke_width(ui.ctx().pixels_per_point());
            mix_max_positions.iter().for_each(|pos| {
                let min = minmax_column_pos(ui, &pos.min, params.round_minmax_to_pixel_center);
                let max = minmax_column_pos(ui, &pos.max, params.round_minmax_to_pixel_center);
                if !params.screen_rect.contains(min.into())
                    && !params.screen_rect.contains(max.into())
                {
                    return;
                }
                let color = params.theme_colors.waveform;
                ui.painter()
                    .line_segment([min, max], egui::Stroke::new(stroke_width, color));
            });
        }
    };
}

fn minmax_column_pos(ui: &egui::Ui, pos: &crate::Pos, round_to_pixel_center: bool) -> egui::Pos2 {
    let pos = egui::Pos2::from(pos);
    if round_to_pixel_center {
        rpc(ui, pos)
    } else {
        pos
    }
}

fn minmax_column_stroke_width(pixels_per_point: f32) -> f32 {
    if pixels_per_point <= 0.0 {
        return 1.0;
    }

    // Min/max data is one vertical column per egui point. On fractional display scales
    // (e.g. 1.25 or 1.5), adjacent point-centered strokes can round to physical pixel
    // centers that are two pixels apart. Widen to a whole number of physical pixels so
    // zoomed-out waveforms remain continuous instead of developing alternating gaps.
    (pixels_per_point.ceil() / pixels_per_point).max(1.0)
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

    // let zero_stroke = egui::Stroke::new(1.0, ui.style().visuals.text_color().linear_multiply(0.55));
    // let zero_stroke = egui::Stroke::new(1.0, ui.style().visuals.text_color());
    // let grid_stroke = egui::Stroke::new(
    //     1.0,
    //     ui.style()
    //         .visuals
    //         .widgets
    //         .noninteractive
    //         .bg_stroke
    //         .color
    //         .linear_multiply(0.7),
    // );
    let zero_stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(128));
    let grid_stroke = egui::Stroke::new(1.0, egui::Color32::from_white_alpha(35));

    for tick in &lattice.ticks {
        if tick.sample_value != 0.0 && tick.tick_type != crate::model::ruler::TickType::Big {
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

#[cfg(test)]
mod tests {
    use super::minmax_column_stroke_width;

    #[test]
    fn minmax_column_stroke_width_covers_fractional_scale_pixels() {
        assert_eq!(minmax_column_stroke_width(1.0), 1.0);
        assert_eq!(minmax_column_stroke_width(2.0), 1.0);
        assert!((minmax_column_stroke_width(1.25) - 1.6).abs() < f32::EPSILON);
        assert!((minmax_column_stroke_width(1.5) - (2.0 / 1.5)).abs() < f32::EPSILON);
    }

    #[test]
    fn minmax_column_stroke_width_handles_invalid_scale() {
        assert_eq!(minmax_column_stroke_width(0.0), 1.0);
        assert_eq!(minmax_column_stroke_width(-1.0), 1.0);
    }
}
