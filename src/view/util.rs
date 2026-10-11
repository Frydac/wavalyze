/// draw a rectangle with a text label
pub fn debug_rect_text(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    color: egui::Color32,
    text: impl ToString,
) {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), egui::FontId::monospace(12.0), color);
    // let rect = rect.expand(2.0);
    ui.painter().galley(rect.min, galley, color);
    debug_rect(ui, rect, color);
}

/// draw a rectangle
pub fn debug_rect(ui: &mut egui::Ui, rect: egui::Rect, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.0, color);
    ui.painter().rect(
        rect,
        0.0,
        egui::Color32::TRANSPARENT,
        stroke,
        egui::epaint::StrokeKind::Inside,
    );
    // ui.painter().line_segment([rect.min, rect.max], egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
    // ui.painter().line_segment([rect.left_top(), rect.right_top()], egui::Stroke::new(1.0, egui::Color32::LIGHT_BLUE));
    ui.painter().line_segment([rect.min, rect.max], stroke);
}

/// round to pixel center (TODO: move to somehwere more general)
/// Useful for pixel-perfect rendering of lines that are one pixel wide (or any odd number of pixels).
pub fn rpc(ui: &egui::Ui, pos: egui::Pos2) -> egui::Pos2 {
    ui.painter().round_pos_to_pixel_center(pos)
}

/// Useful for pixel-perfect rendering of filled shapes.
pub fn rp(ui: &egui::Ui, pos: egui::Pos2) -> egui::Pos2 {
    ui.painter().round_pos_to_pixels(pos)
}

/// Add a left-aligned, truncated label that fills the remaining horizontal space. Used for
/// the left-panel list rows (Files / Tracks) so long names truncate instead of wrapping.
pub fn add_row_label(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) -> egui::Response {
    let size = egui::vec2(ui.available_width().max(0.0), ui.spacing().interact_size.y);
    ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Min), |ui| {
        ui.add(egui::Label::new(text).truncate())
    })
    .inner
}

/// Shorten a path to fit, preserving its filename and suffix where possible.
pub fn truncate_path_keep_basename_to_width(
    ui: &egui::Ui,
    path: &str,
    suffix: &str,
    max_width: f32,
) -> String {
    let font_id = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .cloned()
        .unwrap_or_else(|| egui::FontId::proportional(14.0));
    let color = ui.style().visuals.text_color();

    let measure = |text: &str| -> f32 {
        ui.painter()
            .layout_no_wrap(text.to_string(), font_id.clone(), color)
            .size()
            .x
    };

    let full = format!("{path}{suffix}");
    if measure(&full) <= max_width {
        return full;
    }

    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let base_with_suffix = format!("{base}{suffix}");
    if measure(&base_with_suffix) > max_width {
        return truncate_basename_to_width(&measure, base, suffix, max_width);
    }

    let parent_len = path.len().saturating_sub(base.len());
    let parent = &path[..parent_len];
    truncate_parent_to_width(&measure, parent, base, suffix, max_width)
}

fn truncate_basename_to_width(
    measure: &dyn Fn(&str) -> f32,
    base: &str,
    suffix: &str,
    max_width: f32,
) -> String {
    let ellipsis = "...";
    let base_chars: Vec<char> = base.chars().collect();
    let mut lo = 0usize;
    let mut hi = base_chars.len();

    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let tail: String = base_chars[base_chars.len() - mid..].iter().collect();
        let candidate = format!("{ellipsis}{tail}{suffix}");
        if measure(&candidate) <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    if lo == 0 {
        return format!("{ellipsis}{suffix}");
    }

    let tail: String = base_chars[base_chars.len() - lo..].iter().collect();
    format!("{ellipsis}{tail}{suffix}")
}

fn truncate_parent_to_width(
    measure: &dyn Fn(&str) -> f32,
    parent: &str,
    base: &str,
    suffix: &str,
    max_width: f32,
) -> String {
    let ellipsis = "...";
    let parent_chars: Vec<char> = parent.chars().collect();
    let mut lo = 0usize;
    let mut hi = parent_chars.len();

    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        let tail: String = parent_chars[parent_chars.len() - mid..].iter().collect();
        let candidate = format!("{ellipsis}{tail}{base}{suffix}");
        if measure(&candidate) <= max_width {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }

    let tail: String = parent_chars[parent_chars.len() - lo..].iter().collect();
    format!("{ellipsis}{tail}{base}{suffix}")
}

/// Build the visible part of a ruler's zero-centered zoom deadzone.
pub fn ruler_zero_deadzone(rect: egui::Rect, zero_y: f32, height: f32) -> Option<egui::Rect> {
    if height <= 0.0 || !height.is_finite() || zero_y < rect.top() || zero_y > rect.bottom() {
        return None;
    }

    let deadzone = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, zero_y),
        egui::vec2(rect.width(), height),
    )
    .intersect(rect);
    deadzone.is_positive().then_some(deadzone)
}

/// Convert egui's multiplicative zoom factor back into the wheel-like delta used by the app.
pub fn zoom_delta_to_scroll_delta(zoom_delta: f32, scroll_zoom_speed: f32) -> f32 {
    if zoom_delta == 1.0 || scroll_zoom_speed == 0.0 {
        0.0
    } else {
        zoom_delta.ln() / scroll_zoom_speed
    }
}

/// Whether the ruler should show the exact sample under the horizontal hover position.
pub fn sample_value_readout_visible(samples_per_pixel: f32, max_samples_per_pixel: f32) -> bool {
    samples_per_pixel < max_samples_per_pixel
}

#[cfg(test)]
mod tests {
    use super::{ruler_zero_deadzone, sample_value_readout_visible, zoom_delta_to_scroll_delta};

    #[test]
    fn sample_value_readout_uses_configured_zoom_cutoff() {
        assert!(sample_value_readout_visible(0.5, 1.0));
        assert!(!sample_value_readout_visible(1.0, 1.0));
        assert!(sample_value_readout_visible(4.0, 5.0));
        assert!(!sample_value_readout_visible(0.0, 0.0));
    }

    fn ruler_rect() -> egui::Rect {
        egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(110.0, 120.0))
    }

    #[test]
    fn ruler_zero_deadzone_uses_configured_height_and_full_width() {
        let deadzone = ruler_zero_deadzone(ruler_rect(), 70.0, 16.0).unwrap();

        assert_eq!(deadzone.left(), 10.0);
        assert_eq!(deadzone.right(), 110.0);
        assert_eq!(deadzone.top(), 62.0);
        assert_eq!(deadzone.bottom(), 78.0);
    }

    #[test]
    fn ruler_zero_deadzone_clips_to_ruler() {
        let deadzone = ruler_zero_deadzone(ruler_rect(), 22.0, 16.0).unwrap();

        assert_eq!(deadzone.top(), 20.0);
        assert_eq!(deadzone.bottom(), 30.0);
    }

    #[test]
    fn zero_height_disables_ruler_zero_deadzone() {
        assert_eq!(ruler_zero_deadzone(ruler_rect(), 70.0, 0.0), None);
    }

    #[test]
    fn ruler_zero_deadzone_hit_detection_uses_visible_rectangle() {
        let deadzone = ruler_zero_deadzone(ruler_rect(), 70.0, 16.0).unwrap();

        assert!(deadzone.contains(egui::pos2(60.0, 70.0)));
        assert!(!deadzone.contains(egui::pos2(60.0, 79.0)));
        assert!(!deadzone.contains(egui::pos2(9.0, 70.0)));
    }

    #[test]
    fn zoom_delta_round_trips_to_scroll_delta() {
        let scroll_delta: f32 = 24.0;
        let scroll_zoom_speed: f32 = 1.0 / 200.0;
        let zoom_delta = (scroll_delta * scroll_zoom_speed).exp();

        assert!(
            (zoom_delta_to_scroll_delta(zoom_delta, scroll_zoom_speed) - scroll_delta).abs() < 1e-4
        );
    }
}
