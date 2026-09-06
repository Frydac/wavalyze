use crate::{
    model::{
        Action, Model,
        config::ThemeColors,
        hover_info::{HoverInfo, HoverInfoE},
        track::TrackId,
    },
    view::util::{rpc, zoom_delta_to_scroll_delta},
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum TrackScrollAction {
    PanX { nr_pixels: f32 },
    PanY { nr_pixels: f32 },
    ZoomX { nr_pixels: f32, center_x: f32 },
    ZoomY { nr_pixels: f32, center_y: f32 },
}

fn waveform_contains_pointer(waveform_rect: egui::Rect, pointer: egui::Pos2) -> bool {
    waveform_rect.contains(pointer)
}

pub fn ui_hover(
    ui: &mut egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    waveform_rect: egui::Rect,
    theme_colors: &ThemeColors,
) {
    match &model.tracks.hover_info {
        HoverInfoE::NotHovered => {}
        HoverInfoE::IsHovered(hover_info) => {
            {
                let pos_y_min = waveform_rect.top();
                let pos_y_max = waveform_rect.bottom();
                let pos_x = hover_info.screen_pos.x;
                let pos_min = rpc(ui, egui::pos2(pos_x, pos_y_min));
                let pos_max = rpc(ui, egui::pos2(pos_x, pos_y_max));
                ui.painter().line_segment(
                    [pos_min, pos_max],
                    egui::Stroke::new(1.0, theme_colors.accent),
                );
            }

            {
                if waveform_contains_pointer(waveform_rect, (&hover_info.screen_pos).into()) {
                    let pos_x_min = waveform_rect.left();
                    let pos_x_max = waveform_rect.right();
                    let pos_y = hover_info.screen_pos.y;
                    let pos_min = rpc(ui, egui::pos2(pos_x_min, pos_y));
                    let pos_max = rpc(ui, egui::pos2(pos_x_max, pos_y));
                    ui.painter().line_segment(
                        [pos_min, pos_max],
                        egui::Stroke::new(1.0, theme_colors.accent),
                    );
                }
            }
        }
    }

    let _hover_response = ui
        .interact(waveform_rect, egui::Id::new(track_id), egui::Sense::hover())
        .on_hover_cursor(egui::CursorIcon::None);

    if let Some(pos) = ui.ctx().pointer_hover_pos()
        && waveform_contains_pointer(waveform_rect, pos)
    {
        let sample_ix = model.tracks.screen_x_to_sample_ix(pos.x).unwrap_or(0.0);
        let sample_pos_x = model
            .tracks
            .sample_ix_to_screen_x(sample_ix.round())
            .map(|x| x.floor() as f64);
        model
            .actions
            .push(Action::SetHoverInfo(HoverInfoE::IsHovered(HoverInfo {
                screen_pos: pos.into(),
                sample_ix,
                sample_pos_x,
                track_id: Some(track_id),
            })));
        let scroll_zoom_speed = ui.ctx().options(|o| o.input_options.scroll_zoom_speed);
        ui.ctx().input(|i| {
            if let Some(action) = track_scroll_action(
                i.modifiers,
                i.smooth_scroll_delta,
                zoom_delta_to_scroll_delta(i.zoom_delta(), scroll_zoom_speed),
                pos,
                &model.user_config.navigation,
            ) {
                match action {
                    TrackScrollAction::PanX { nr_pixels } => {
                        model.actions.push(Action::PanX { nr_pixels });
                    }
                    TrackScrollAction::PanY { nr_pixels } => {
                        model.actions.push(Action::PanY {
                            track_id,
                            nr_pixels,
                        });
                    }
                    TrackScrollAction::ZoomX {
                        nr_pixels,
                        center_x,
                    } => {
                        model.actions.push(Action::ZoomX {
                            nr_pixels,
                            center_x,
                        });
                    }
                    TrackScrollAction::ZoomY {
                        nr_pixels,
                        center_y,
                    } => {
                        model.actions.push(Action::ZoomY {
                            track_id,
                            nr_pixels,
                            center_y,
                        });
                    }
                }
            }
        });
    }
}

fn track_scroll_action(
    modifiers: egui::Modifiers,
    scroll: egui::Vec2,
    zoom_scroll_delta: f32,
    pos: egui::Pos2,
    navigation: &crate::model::config::NavigationConfig,
) -> Option<TrackScrollAction> {
    let scroll_y = effective_scroll_delta(scroll);
    let zoom_delta = effective_zoom_delta(scroll_y, zoom_scroll_delta);
    if modifiers.alt {
        if modifiers.shift && !modifiers.ctrl && scroll_y != 0.0 {
            Some(TrackScrollAction::PanY {
                nr_pixels: scroll_y * navigation.pan_y_mult(),
            })
        } else if modifiers.ctrl && zoom_delta != 0.0 {
            Some(TrackScrollAction::ZoomY {
                nr_pixels: zoom_delta * navigation.zoom_y_mult(),
                center_y: pos.y,
            })
        } else {
            None
        }
    } else if modifiers.shift && !modifiers.ctrl {
        if scroll.x != 0.0 {
            Some(TrackScrollAction::PanX {
                nr_pixels: scroll.x * navigation.pan_x_mult(),
            })
        } else {
            None
        }
    } else if modifiers.ctrl && zoom_delta != 0.0 {
        Some(TrackScrollAction::ZoomX {
            nr_pixels: zoom_delta * navigation.zoom_x_mult(),
            center_x: pos.x,
        })
    } else {
        None
    }
}

fn effective_scroll_delta(scroll: egui::Vec2) -> f32 {
    if scroll.y != 0.0 { scroll.y } else { scroll.x }
}

fn effective_zoom_delta(scroll_y: f32, zoom_scroll_delta: f32) -> f32 {
    if scroll_y != 0.0 {
        scroll_y
    } else {
        zoom_scroll_delta
    }
}

#[cfg(test)]
mod tests {
    use super::{TrackScrollAction, track_scroll_action, waveform_contains_pointer};
    use crate::model::config::NavigationConfig;
    use egui::{Modifiers, Pos2, Vec2};

    #[test]
    fn hover_containment_includes_waveform_boundaries() {
        let rect = egui::Rect::from_min_max(Pos2::new(10.0, 20.0), Pos2::new(110.0, 70.0));

        for pointer in [
            rect.left_top(),
            rect.right_top(),
            rect.left_bottom(),
            rect.right_bottom(),
            rect.center(),
        ] {
            assert!(waveform_contains_pointer(rect, pointer));
        }
    }

    #[test]
    fn hover_containment_rejects_points_just_outside_waveform() {
        let rect = egui::Rect::from_min_max(Pos2::new(10.0, 20.0), Pos2::new(110.0, 70.0));

        for pointer in [
            Pos2::new(rect.left() - 0.01, rect.center().y),
            Pos2::new(rect.right() + 0.01, rect.center().y),
            Pos2::new(rect.center().x, rect.top() - 0.01),
            Pos2::new(rect.center().x, rect.bottom() + 0.01),
        ] {
            assert!(!waveform_contains_pointer(rect, pointer));
        }
    }

    #[test]
    fn ctrl_vertical_scroll_zooms_x() {
        let action = track_scroll_action(
            Modifiers {
                ctrl: true,
                ..Modifiers::NONE
            },
            Vec2::new(0.0, 3.0),
            0.0,
            Pos2::new(10.0, 20.0),
            &NavigationConfig::default(),
        );

        assert_eq!(
            action,
            Some(TrackScrollAction::ZoomX {
                nr_pixels: 12.0,
                center_x: 10.0,
            })
        );
    }

    #[test]
    fn ctrl_horizontal_only_scroll_also_zooms_x() {
        let action = track_scroll_action(
            Modifiers {
                ctrl: true,
                ..Modifiers::NONE
            },
            Vec2::new(3.0, 0.0),
            0.0,
            Pos2::new(10.0, 20.0),
            &NavigationConfig::default(),
        );

        assert_eq!(
            action,
            Some(TrackScrollAction::ZoomX {
                nr_pixels: 12.0,
                center_x: 10.0,
            })
        );
    }

    #[test]
    fn alt_ctrl_horizontal_only_scroll_zooms_y() {
        let action = track_scroll_action(
            Modifiers {
                alt: true,
                ctrl: true,
                ..Modifiers::NONE
            },
            Vec2::new(3.0, 0.0),
            0.0,
            Pos2::new(10.0, 20.0),
            &NavigationConfig::default(),
        );

        assert_eq!(
            action,
            Some(TrackScrollAction::ZoomY {
                nr_pixels: 12.0,
                center_y: 20.0,
            })
        );
    }

    #[test]
    fn shift_scroll_pans_x_without_zooming() {
        let action = track_scroll_action(
            Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
            Vec2::new(3.0, 5.0),
            0.0,
            Pos2::new(10.0, 20.0),
            &NavigationConfig::default(),
        );

        assert_eq!(action, Some(TrackScrollAction::PanX { nr_pixels: 3.0 }));
    }

    #[test]
    fn plain_scroll_does_not_emit_track_action() {
        let action = track_scroll_action(
            Modifiers::NONE,
            Vec2::new(3.0, 5.0),
            0.0,
            Pos2::new(10.0, 20.0),
            &NavigationConfig::default(),
        );

        assert_eq!(action, None);
    }

    #[test]
    fn invert_zoom_x_flips_zoom_direction() {
        let navigation = NavigationConfig {
            invert_zoom_x: true,
            ..NavigationConfig::default()
        };
        let action = track_scroll_action(
            Modifiers {
                ctrl: true,
                ..Modifiers::NONE
            },
            Vec2::new(0.0, 3.0),
            0.0,
            Pos2::new(10.0, 20.0),
            &navigation,
        );

        assert_eq!(
            action,
            Some(TrackScrollAction::ZoomX {
                nr_pixels: -12.0,
                center_x: 10.0,
            })
        );
    }

    #[test]
    fn pan_x_factor_scales_pan() {
        let navigation = NavigationConfig {
            pan_x_factor: 2.0,
            ..NavigationConfig::default()
        };
        let action = track_scroll_action(
            Modifiers {
                shift: true,
                ..Modifiers::NONE
            },
            Vec2::new(3.0, 5.0),
            0.0,
            Pos2::new(10.0, 20.0),
            &navigation,
        );

        assert_eq!(action, Some(TrackScrollAction::PanX { nr_pixels: 6.0 }));
    }

    #[test]
    fn ctrl_zoom_delta_zooms_x_when_scroll_delta_is_zero() {
        let action = track_scroll_action(
            Modifiers {
                ctrl: true,
                ..Modifiers::NONE
            },
            Vec2::ZERO,
            3.0,
            Pos2::new(10.0, 20.0),
            &NavigationConfig::default(),
        );

        assert_eq!(
            action,
            Some(TrackScrollAction::ZoomX {
                nr_pixels: 12.0,
                center_x: 10.0,
            })
        );
    }
}
