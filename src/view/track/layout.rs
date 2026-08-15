use crate::model::{config::RULER_SLOT_WIDTH, track::HEADER_HEIGHT};

const RESIZE_HANDLE_HEIGHT: f32 = 3.0;

/// The shared horizontal split between a track's sidebar and its waveform content.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::view) struct TrackColumns {
    pub sidebar: egui::Rect,
    pub content: egui::Rect,
}

impl TrackColumns {
    pub(in crate::view) fn new(rect: egui::Rect, sidebar_width: f32) -> Self {
        let rect = non_negative_rect(rect);
        let sidebar_width = if sidebar_width.is_nan() {
            0.0
        } else {
            sidebar_width.clamp(0.0, rect.width())
        };
        let split_x = rect.left() + sidebar_width;

        Self {
            sidebar: egui::Rect::from_min_max(rect.min, egui::pos2(split_x, rect.bottom())),
            content: egui::Rect::from_min_max(egui::pos2(split_x, rect.top()), rect.right_bottom()),
        }
    }
}

/// All fixed geometry for one track.
///
/// Component renderers should treat these rectangles as authoritative. The resize handle is the
/// only intentional overlap: it sits over the bottom edge of the track without reducing the
/// waveform or sidebar body height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TrackLayout {
    pub track: egui::Rect,
    pub columns: TrackColumns,
    pub sidebar_header_controls: egui::Rect,
    pub reset_y_button: Option<egui::Rect>,
    pub stats_viewport: egui::Rect,
    pub db_ruler: Option<egui::Rect>,
    pub amplitude_ruler: Option<egui::Rect>,
    pub waveform_header: egui::Rect,
    pub waveform_canvas: egui::Rect,
    pub resize_handle: egui::Rect,
}

impl TrackLayout {
    pub fn new(
        track: egui::Rect,
        sidebar_width: f32,
        show_amplitude_ruler: bool,
        show_db_ruler: bool,
    ) -> Self {
        let track = non_negative_rect(track);
        let columns = TrackColumns::new(track, sidebar_width);
        let header_height = HEADER_HEIGHT.min(track.height());
        let body_top = track.top() + header_height;

        let sidebar_header = egui::Rect::from_min_max(
            columns.sidebar.min,
            egui::pos2(columns.sidebar.right(), body_top),
        );
        let sidebar_body = egui::Rect::from_min_max(
            egui::pos2(columns.sidebar.left(), body_top),
            columns.sidebar.right_bottom(),
        );

        // Rulers are assigned from the waveform edge leftward. Amplitude remains right-most,
        // matching the existing per-track presentation.
        let mut ruler_cursor_x = sidebar_body.right();
        let amplitude_ruler = show_amplitude_ruler
            .then(|| take_ruler_slot(sidebar_body, &mut ruler_cursor_x))
            .flatten();
        let db_ruler = show_db_ruler
            .then(|| take_ruler_slot(sidebar_body, &mut ruler_cursor_x))
            .flatten();

        let stats_viewport = egui::Rect::from_min_max(
            sidebar_body.min,
            egui::pos2(ruler_cursor_x, sidebar_body.bottom()),
        );

        let rightmost_ruler = amplitude_ruler.or(db_ruler);
        let reset_y_button = rightmost_ruler.map(|ruler| {
            egui::Rect::from_min_max(
                egui::pos2(ruler.left(), sidebar_header.top()),
                egui::pos2(ruler.right(), sidebar_header.bottom()),
            )
        });
        let controls_right = reset_y_button
            .map(|rect| rect.left())
            .unwrap_or(sidebar_header.right());
        let sidebar_header_controls = egui::Rect::from_min_max(
            sidebar_header.min,
            egui::pos2(controls_right, sidebar_header.bottom()),
        );

        let waveform_header = egui::Rect::from_min_max(
            columns.content.min,
            egui::pos2(columns.content.right(), body_top),
        );
        let waveform_canvas = egui::Rect::from_min_max(
            egui::pos2(columns.content.left(), body_top),
            columns.content.right_bottom(),
        );

        let resize_height = RESIZE_HANDLE_HEIGHT.min(track.height());
        let resize_handle = egui::Rect::from_min_max(
            egui::pos2(track.left(), track.bottom() - resize_height),
            track.right_bottom(),
        );

        Self {
            track,
            columns,
            sidebar_header_controls,
            reset_y_button,
            stats_viewport,
            db_ruler,
            amplitude_ruler,
            waveform_header,
            waveform_canvas,
            resize_handle,
        }
    }
}

/// Collapse an inverted input rectangle at its minimum edge. Normal egui layout rectangles are
/// already valid, but doing this at the geometry boundary guarantees that all derived regions
/// have non-negative dimensions.
fn non_negative_rect(rect: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.right().max(rect.left()), rect.bottom().max(rect.top())),
    )
}

fn take_ruler_slot(body: egui::Rect, cursor_x: &mut f32) -> Option<egui::Rect> {
    let available_width = (*cursor_x - body.left()).max(0.0);
    let width = RULER_SLOT_WIDTH.min(available_width);
    if width <= 0.0 || body.height() <= 0.0 {
        return None;
    }

    let rect = egui::Rect::from_min_max(
        egui::pos2(*cursor_x - width, body.top()),
        egui::pos2(*cursor_x, body.bottom()),
    );
    *cursor_x -= width;
    Some(rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(width: f32, height: f32) -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(width, height))
    }

    #[test]
    fn columns_clamp_sidebar_to_available_width() {
        let columns = TrackColumns::new(rect(100.0, 50.0), 140.0);

        assert_eq!(columns.sidebar.width(), 100.0);
        assert_eq!(columns.content.width(), 0.0);
        assert_eq!(columns.sidebar.right(), columns.content.left());
    }

    #[test]
    fn both_rulers_are_right_aligned_with_amplitude_rightmost() {
        let layout = TrackLayout::new(rect(500.0, 100.0), 250.0, true, true);
        let amplitude = layout.amplitude_ruler.unwrap();
        let db = layout.db_ruler.unwrap();

        assert_eq!(amplitude.width(), RULER_SLOT_WIDTH);
        assert_eq!(db.width(), RULER_SLOT_WIDTH);
        assert_eq!(amplitude.right(), layout.columns.sidebar.right());
        assert_eq!(db.right(), amplitude.left());
        assert_eq!(layout.stats_viewport.right(), db.left());
        assert_eq!(
            layout.reset_y_button.unwrap().x_range(),
            amplitude.x_range()
        );
    }

    #[test]
    fn one_ruler_occupies_the_rightmost_sidebar_slot() {
        for (show_amplitude, show_db) in [(true, false), (false, true)] {
            let layout = TrackLayout::new(rect(500.0, 100.0), 250.0, show_amplitude, show_db);
            let ruler = layout.amplitude_ruler.or(layout.db_ruler).unwrap();

            assert_eq!(ruler.width(), RULER_SLOT_WIDTH);
            assert_eq!(ruler.right(), layout.columns.sidebar.right());
            assert_eq!(layout.stats_viewport.right(), ruler.left());
            assert_eq!(layout.reset_y_button.unwrap().x_range(), ruler.x_range());
            assert_eq!(layout.amplitude_ruler.is_some(), show_amplitude);
            assert_eq!(layout.db_ruler.is_some(), show_db);
        }
    }

    #[test]
    fn no_rulers_gives_the_stats_viewport_the_full_sidebar_body() {
        let layout = TrackLayout::new(rect(500.0, 100.0), 250.0, false, false);

        assert_eq!(layout.stats_viewport.left(), layout.columns.sidebar.left());
        assert_eq!(
            layout.stats_viewport.right(),
            layout.columns.sidebar.right()
        );
        assert_eq!(layout.sidebar_header_controls.width(), 250.0);
        assert!(layout.reset_y_button.is_none());
    }

    #[test]
    fn compact_track_keeps_all_vertical_geometry_inside_track() {
        let track = rect(500.0, 25.0);
        let layout = TrackLayout::new(track, 250.0, true, false);

        for component in [
            layout.sidebar_header_controls,
            layout.reset_y_button.unwrap(),
            layout.stats_viewport,
            layout.amplitude_ruler.unwrap(),
            layout.waveform_header,
            layout.waveform_canvas,
            layout.resize_handle,
        ] {
            assert!(track.contains_rect(component));
        }
        assert_eq!(layout.waveform_header.height(), HEADER_HEIGHT);
        assert_eq!(layout.waveform_canvas.height(), 25.0 - HEADER_HEIGHT);
    }

    #[test]
    fn compact_body_heights_are_preserved_exactly() {
        for body_height in [10.0, 25.0] {
            let track = rect(500.0, HEADER_HEIGHT + body_height);
            let layout = TrackLayout::new(track, 250.0, true, true);

            assert_eq!(layout.waveform_header.height(), HEADER_HEIGHT);
            assert_eq!(layout.waveform_canvas.height(), body_height);
            assert_eq!(layout.stats_viewport.height(), body_height);
            assert_eq!(layout.amplitude_ruler.unwrap().height(), body_height);
            assert_eq!(layout.db_ruler.unwrap().height(), body_height);
        }
    }

    #[test]
    fn header_and_body_regions_share_boundaries_without_gaps() {
        let layout = TrackLayout::new(rect(500.0, 100.0), 250.0, true, true);
        let body_top = layout.track.top() + HEADER_HEIGHT;

        assert_eq!(layout.sidebar_header_controls.top(), layout.track.top());
        assert_eq!(layout.sidebar_header_controls.bottom(), body_top);
        assert_eq!(layout.reset_y_button.unwrap().bottom(), body_top);
        assert_eq!(layout.stats_viewport.top(), body_top);
        assert_eq!(layout.amplitude_ruler.unwrap().top(), body_top);
        assert_eq!(layout.db_ruler.unwrap().top(), body_top);
        assert_eq!(layout.waveform_header.bottom(), body_top);
        assert_eq!(layout.waveform_canvas.top(), body_top);
        assert_eq!(layout.waveform_canvas.bottom(), layout.track.bottom());
    }

    #[test]
    fn narrow_sidebar_clamps_ruler_slots_without_negative_rectangles() {
        let layout = TrackLayout::new(rect(100.0, 50.0), 50.0, true, true);

        assert_eq!(layout.amplitude_ruler.unwrap().width(), 50.0);
        assert!(layout.db_ruler.is_none());
        assert_eq!(layout.stats_viewport.width(), 0.0);
    }

    #[test]
    fn header_consumes_a_track_shorter_than_header_height_without_overflow() {
        let track = rect(100.0, 10.0);
        let layout = TrackLayout::new(track, 50.0, true, true);

        assert_eq!(layout.waveform_header.height(), 10.0);
        assert_eq!(layout.waveform_canvas.height(), 0.0);
        assert_eq!(layout.stats_viewport.height(), 0.0);
        assert!(layout.amplitude_ruler.is_none());
        assert!(layout.db_ruler.is_none());
        assert!(track.contains_rect(layout.resize_handle));
    }

    #[test]
    fn narrow_window_can_collapse_waveform_content_to_zero_width() {
        let track = rect(120.0, 60.0);
        let layout = TrackLayout::new(track, 250.0, true, true);

        assert_eq!(layout.columns.sidebar.width(), track.width());
        assert_eq!(layout.columns.content.width(), 0.0);
        assert_eq!(layout.waveform_header.width(), 0.0);
        assert_eq!(layout.waveform_canvas.width(), 0.0);
        assert_eq!(layout.amplitude_ruler.unwrap().width(), RULER_SLOT_WIDTH);
        assert_eq!(layout.db_ruler.unwrap().width(), 40.0);
        assert_eq!(layout.stats_viewport.width(), 0.0);
    }

    #[test]
    fn all_derived_regions_are_contained_by_the_track() {
        for (show_amplitude, show_db) in
            [(false, false), (true, false), (false, true), (true, true)]
        {
            let track = rect(500.0, HEADER_HEIGHT + 10.0);
            let layout = TrackLayout::new(track, 250.0, show_amplitude, show_db);
            let mut regions = vec![
                layout.columns.sidebar,
                layout.columns.content,
                layout.sidebar_header_controls,
                layout.stats_viewport,
                layout.waveform_header,
                layout.waveform_canvas,
                layout.resize_handle,
            ];
            regions.extend(layout.reset_y_button);
            regions.extend(layout.db_ruler);
            regions.extend(layout.amplitude_ruler);

            assert!(
                regions
                    .into_iter()
                    .all(|region| track.contains_rect(region))
            );
        }
    }

    #[test]
    fn top_row_and_track_columns_align_for_the_same_sidebar_width() {
        let top_row =
            egui::Rect::from_min_size(egui::pos2(10.0, 0.0), egui::vec2(500.0, HEADER_HEIGHT));
        let track = rect(500.0, 100.0);
        let sidebar_width = 250.0;

        let top_columns = TrackColumns::new(top_row, sidebar_width);
        let track_layout = TrackLayout::new(track, sidebar_width, true, true);

        assert_eq!(
            top_columns.sidebar.left(),
            track_layout.columns.sidebar.left()
        );
        assert_eq!(
            top_columns.sidebar.right(),
            track_layout.columns.sidebar.right()
        );
        assert_eq!(
            top_columns.content.left(),
            track_layout.columns.content.left()
        );
        assert_eq!(
            top_columns.content.right(),
            track_layout.columns.content.right()
        );
    }

    #[test]
    fn negative_sidebar_width_collapses_sidebar_and_leaves_content_available() {
        let layout = TrackLayout::new(rect(100.0, 50.0), -20.0, true, true);

        assert_eq!(layout.columns.sidebar.width(), 0.0);
        assert_eq!(layout.columns.content.width(), 100.0);
        assert_eq!(layout.stats_viewport.width(), 0.0);
        assert!(layout.amplitude_ruler.is_none());
        assert!(layout.db_ruler.is_none());
    }

    #[test]
    fn inverted_input_rect_is_collapsed_before_deriving_regions() {
        let inverted = egui::Rect::from_min_max(egui::pos2(20.0, 30.0), egui::pos2(10.0, 15.0));
        let layout = TrackLayout::new(inverted, 10.0, true, true);

        assert_eq!(layout.track.width(), 0.0);
        assert_eq!(layout.track.height(), 0.0);
        assert_eq!(layout.waveform_header.size(), egui::Vec2::ZERO);
        assert_eq!(layout.waveform_canvas.size(), egui::Vec2::ZERO);
        assert_eq!(layout.resize_handle.size(), egui::Vec2::ZERO);
    }

    #[test]
    fn nan_sidebar_width_is_treated_as_zero() {
        let columns = TrackColumns::new(rect(100.0, 50.0), f32::NAN);

        assert_eq!(columns.sidebar.width(), 0.0);
        assert_eq!(columns.content.width(), 100.0);
    }
}
