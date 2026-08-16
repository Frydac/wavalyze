use crate::{
    model::{
        Action, Model,
        track::{self, TrackId},
    },
    view::{db_ruler, grid::KeyValueGrid, value_ruler2},
};
use anyhow::Result;

mod hover;
// The geometry types are introduced separately from their rendering integration.
#[allow(dead_code)]
pub(super) mod layout;
mod selection;
mod waveform;

use layout::TrackLayout;

/// Preformatted metadata for the track-header hover popup.
///
/// The header itself is width-constrained and intentionally terse, while the popup can show the
/// full file/channel context in a stable key/value layout.
#[derive(Debug, Clone)]
pub(crate) struct TrackHeaderHoverInfo {
    path: String,
    channel: String,
    nr_channels: String,
    sample_type: String,
    bit_depth: String,
    sample_rate: String,
    nr_samples: String,
    duration: String,
    layout: String,
}

impl TrackHeaderHoverInfo {
    fn ui(&self, ui: &mut egui::Ui, track_id: TrackId) {
        ui.heading("Track source");
        ui.separator();

        // Use a per-track grid id so multiple track headers can coexist without sharing egui
        // interaction/layout state.
        let id: u64 = ui.id().with(("track_header_hover_grid", track_id)).value();
        let mut grid = KeyValueGrid::new(id).key_col_width(95.0);
        grid.row("path", self.path.clone());
        grid.row("channel ix", self.channel.clone());
        grid.row("channels", self.nr_channels.clone());
        grid.row("sample type", self.sample_type.clone());
        grid.row("bit depth", self.bit_depth.clone());
        grid.row("sample rate", self.sample_rate.clone());
        grid.row("samples", self.nr_samples.clone());
        grid.row("duration", self.duration.clone());
        grid.row("layout", self.layout.clone());
        grid.show(ui);
    }
}

/// One side of a diff track: the file/channel of a source buffer being compared.
#[derive(Debug, Clone)]
pub(crate) struct DiffSourceInfo {
    path: String,
    channel: String,
    sample_rate: String,
    duration: String,
}

impl DiffSourceInfo {
    fn rows(&self, ui: &mut egui::Ui, id: u64) {
        let mut grid = KeyValueGrid::new(id).key_col_width(95.0);
        grid.row("path", self.path.clone());
        grid.row("channel ix", self.channel.clone());
        grid.row("sample rate", self.sample_rate.clone());
        grid.row("duration", self.duration.clone());
        grid.show(ui);
    }
}

/// Hover popup for a diff track: describes both source channels being diffed (A − B).
#[derive(Debug, Clone)]
pub(crate) struct DiffHeaderHoverInfo {
    a: DiffSourceInfo,
    b: DiffSourceInfo,
}

impl DiffHeaderHoverInfo {
    fn ui(&self, ui: &mut egui::Ui, track_id: TrackId) {
        ui.heading("Diff sources (A − B)");
        ui.separator();
        ui.strong("A");
        let id_a = ui.id().with(("diff_header_hover_a", track_id)).value();
        self.a.rows(ui, id_a);
        ui.add_space(6.0);
        ui.strong("B");
        let id_b = ui.id().with(("diff_header_hover_b", track_id)).value();
        self.b.rows(ui, id_b);
    }
}

/// Either kind of track-header hover popup.
pub(crate) enum HeaderHover {
    Single(TrackHeaderHoverInfo),
    Diff(DiffHeaderHoverInfo),
}

impl HeaderHover {
    pub(crate) fn show(&self, ui: &mut egui::Ui, track_id: TrackId) {
        match self {
            HeaderHover::Single(info) => info.ui(ui, track_id),
            HeaderHover::Diff(info) => info.ui(ui, track_id),
        }
    }
}

/// Build the track-header hover popup data for a track, or `None` if the track is neither a
/// known single channel nor a diff. Shared by the central-panel header and the left-panel
/// Tracks list so both show identical metadata.
pub(crate) fn header_hover_info(model: &Model, track_id: TrackId) -> Option<HeaderHover> {
    if let Some((file, channel)) = model.get_file_channel_for_track(track_id) {
        let path = file
            .path
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("unknown");
        Some(HeaderHover::Single(TrackHeaderHoverInfo {
            path: path.to_string(),
            channel: channel.ch_ix.to_string(),
            nr_channels: file.channels.len().to_string(),
            sample_type: format!("{:?}", file.sample_type),
            bit_depth: file.bit_depth.to_string(),
            sample_rate: format!("{} Hz", file.sample_rate),
            nr_samples: file.nr_samples.to_string(),
            duration: if file.sample_rate == 0 {
                String::from("unknown")
            } else {
                format!("{:.3} s", file.nr_samples as f64 / file.sample_rate as f64)
            },
            layout: file
                .layout
                .as_ref()
                .map(|layout| format!("{layout:?}"))
                .unwrap_or_else(|| String::from("unknown")),
        }))
    } else {
        let diff = model
            .tracks
            .get_track(track_id)
            .and_then(|track| track.diff.clone())?;
        Some(HeaderHover::Diff(DiffHeaderHoverInfo {
            a: diff_source_info(model, diff.buffer_id_a),
            b: diff_source_info(model, diff.buffer_id_b),
        }))
    }
}

/// One-line label for a track, matching the central-panel header text:
/// `"<basename> - ch N"` for single tracks, `"Diff"` for diff tracks.
pub(crate) fn track_label(model: &Model, track_id: TrackId) -> String {
    if let Some((file, channel)) = model.get_file_channel_for_track(track_id) {
        let basename = file
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("Demo");
        format!("{basename} - ch {}", channel.ch_ix)
    } else if model
        .tracks
        .get_track(track_id)
        .is_some_and(|track| track.diff.is_some())
    {
        String::from("Diff")
    } else {
        String::from("track")
    }
}

/// Resolve a source buffer to a [`DiffSourceInfo`], falling back to "unknown" if the file/channel is
/// no longer present (e.g. the source file was closed).
fn diff_source_info(model: &Model, buffer_id: crate::audio::BufferId) -> DiffSourceInfo {
    match model.get_file_channel_for_buffer(buffer_id) {
        Some((file, channel)) => DiffSourceInfo {
            path: file
                .path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or("unknown")
                .to_string(),
            channel: channel.ch_ix.to_string(),
            sample_rate: format!("{} Hz", file.sample_rate),
            duration: if file.sample_rate == 0 {
                String::from("unknown")
            } else {
                format!("{:.3} s", file.nr_samples as f64 / file.sample_rate as f64)
            },
        },
        None => DiffSourceInfo {
            path: String::from("unknown"),
            channel: String::from("unknown"),
            sample_rate: String::from("unknown"),
            duration: String::from("unknown"),
        },
    }
}

pub fn ui(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId) -> Result<()> {
    let theme_colors = model.user_config.active_theme_colors(ui.visuals()).clone();
    let min_height = track::min_total_height(&model.user_config.track);
    let width = ui.available_width().max(0.0);
    let sidebar_width = model.user_config.effective_tracks_width_info();
    let height = model
        .tracks
        .get_track_height(track_id)
        .unwrap_or(min_height);
    let height = height.max(0.0);

    // reserves space in the parent ui, moves the parent cursor
    let (track_rect, _) =
        ui.allocate_exact_size(egui::Vec2::new(width, height), egui::Sense::hover());

    // crate::view::util::debug_rect_text(ui, track_rect, egui::Color32::RED, "track_rect");

    let layout = TrackLayout::new(
        track_rect,
        sidebar_width,
        model.user_config.show_amplitude_ruler,
        model.user_config.show_db_ruler,
    );

    // Components render into geometry derived from `track_rect`; their intrinsic sizes no longer
    // participate in positioning their siblings.
    let mut track_ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(("track", track_id))
            .max_rect(layout.track),
    );
    track_ui.set_clip_rect(ui.clip_rect().intersect(layout.track));

    let mut sidebar_ui = component_ui(
        &mut track_ui,
        track_id,
        "sidebar",
        layout.columns.sidebar,
        egui::Layout::top_down(egui::Align::Min),
    );
    ui_side(&mut sidebar_ui, model, track_id, &layout);

    let mut header_ui = component_ui(
        &mut track_ui,
        track_id,
        "waveform_header",
        layout.waveform_header,
        egui::Layout::top_down(egui::Align::Min),
    );
    let _ = ui_header(&mut header_ui, model, track_id, layout.waveform_header);

    let mut waveform_ui = component_ui(
        &mut track_ui,
        track_id,
        "waveform_canvas",
        layout.waveform_canvas,
        egui::Layout::top_down(egui::Align::Min),
    );
    let _ = waveform::ui_waveform_canvas(
        &mut waveform_ui,
        model,
        track_id,
        layout.waveform_canvas,
        &theme_colors,
    );

    let mut resize_ui = component_ui(
        &mut track_ui,
        track_id,
        "resize_handle",
        layout.resize_handle,
        egui::Layout::top_down(egui::Align::Min),
    );
    let min_height = track::min_total_height(&model.user_config.track);
    let resize_id = resize_ui.id().with("interaction");
    let response = resize_handle(&mut resize_ui, resize_id, layout.resize_handle);
    if response.dragged() {
        let modifiers = resize_ui.input(|i| i.modifiers);
        let track = model
            .tracks
            .get_track_mut(track_id)
            .ok_or_else(|| anyhow::anyhow!("Track {:?} not found", track_id))?;

        if !modifiers.ctrl && !modifiers.shift && !modifiers.alt {
            track.height = (track.height + response.drag_delta().y).max(min_height);
        } else if modifiers.shift {
            let new_height = (track.height + response.drag_delta().y).max(min_height);
            model.tracks.set_tracks_height(new_height);
        }
    }

    Ok(())
}

fn component_ui(
    parent: &mut egui::Ui,
    track_id: TrackId,
    component: &'static str,
    rect: egui::Rect,
    layout: egui::Layout,
) -> egui::Ui {
    let clip_rect = parent.clip_rect().intersect(rect);
    let mut child = parent.new_child(
        egui::UiBuilder::new()
            .id_salt((component, track_id))
            .max_rect(rect)
            .layout(layout),
    );
    child.set_clip_rect(clip_rect);
    child
}

// UI part on the left side of each track
// contains:
// - track info
// - sample value ruler
fn ui_side(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId, layout: &TrackLayout) {
    let info_rect = layout.columns.sidebar;
    let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
    ui.painter().rect(
        info_rect,
        0.0,
        egui::Color32::TRANSPARENT,
        stroke,
        egui::epaint::StrokeKind::Inside,
    );

    ui_offset_controls(ui, model, track_id, layout.sidebar_header_controls);

    if let Some(rect) = layout.y_zoom_controls {
        ui_y_zoom_controls(ui, model, track_id, rect);
    }

    ui_stats_viewport(ui, model, track_id, layout.stats_viewport);

    ui_rulers(ui, model, track_id, layout.db_ruler, layout.amplitude_ruler);
}

fn ui_offset_controls(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId, rect: egui::Rect) {
    let mut controls_ui = component_ui(
        ui,
        track_id,
        "sidebar_header_controls",
        rect,
        egui::Layout::left_to_right(egui::Align::Center),
    );
    controls_ui.spacing_mut().item_spacing = egui::Vec2::new(3.0, 3.0);

    if let Some(track) = model.tracks.get_track_mut(track_id) {
        let sample_ix_offset = &mut track.single.sample_ix_offset;
        controls_ui.label("offset:");
        let response = controls_ui.add(egui::DragValue::new(sample_ix_offset).speed(1.0));
        if response.changed() {
            track.single.mark_dirty();
        }
    }
}

fn ui_y_zoom_controls(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId, rect: egui::Rect) {
    let mut controls_ui = component_ui(
        ui,
        track_id,
        "y_zoom_controls",
        rect,
        egui::Layout::left_to_right(egui::Align::Center),
    );
    controls_ui.spacing_mut().item_spacing.x = 2.0;

    if controls_ui
        .button("r")
        .on_hover_text("Reset Y-axis zoom and pan")
        .clicked()
    {
        model.actions.push(Action::RecenterY { track_id });
    }
    if controls_ui
        .button("a")
        .on_hover_text(
            "Auto-fit Y to the selection peak; uses the visible time range when there is no selection",
        )
        .clicked()
    {
        model.actions.push(Action::AutoFitY { track_id });
    }
}

fn ui_stats_viewport(ui: &mut egui::Ui, model: &mut Model, track_id: TrackId, rect: egui::Rect) {
    if rect.width() <= 0.0 || rect.height() <= 0.0 {
        return;
    }
    let Some(buffer_id) = model
        .tracks
        .get_track(track_id)
        .map(|track| track.single.buffer_id)
    else {
        return;
    };

    let mut stats_ui = component_ui(
        ui,
        track_id,
        "stats_viewport",
        rect,
        egui::Layout::top_down(egui::Align::Min),
    );
    stats_ui.spacing_mut().item_spacing = egui::Vec2::new(3.0, 2.0);
    egui::ScrollArea::vertical()
        .id_salt(("track_stats", track_id))
        .max_height(rect.height())
        // egui defaults this to 64 points. Compact tracks must scroll inside their fixed,
        // clipped viewport instead of increasing the track's height.
        .min_scrolled_height(0.0)
        .show(&mut stats_ui, |ui| {
            let stats = model.audio.stats.get(buffer_id).copied();
            let label = if stats.is_some() { "recalc" } else { "stats" };
            if ui
                .button(label)
                .on_hover_text("Gather stats (RMS, peak) over the selection, or the whole buffer when nothing is selected")
                .clicked()
            {
                model.actions.push(Action::ComputeBufferStats {
                    buffer_id,
                    track_id,
                    options: crate::model::stats::StatsOptions::default(),
                });
            }
            if let Some(stats) = stats
                && let Some(track) = model.tracks.get_track_mut(track_id)
            {
                ui_stats_grid(ui, track_id, stats, &mut track.show_peak_marker);
            }
        });
}

fn ui_rulers(
    ui: &mut egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    db_rect: Option<egui::Rect>,
    amplitude_rect: Option<egui::Rect>,
) {
    let Some(track) = model.tracks.get_track(track_id) else {
        return;
    };
    let hover_info = model.tracks.hover_info;
    let theme_colors = model.user_config.active_theme_colors(ui.visuals());
    let pan_y_mult = model.user_config.navigation.pan_y_mult();
    let zoom_y_mult = model.user_config.navigation.zoom_y_mult();
    let display_scale = model.user_config.value_display_scale;

    if let Some(rect) = db_rect {
        let mut ruler_ui = component_ui(
            ui,
            track_id,
            "db_ruler",
            rect,
            egui::Layout::top_down(egui::Align::Min),
        );
        let mut ctx = db_ruler::DbRulerContext {
            actions: &mut model.actions,
            hover_info: &hover_info,
            audio: &model.audio,
            pan_y_mult,
            zoom_y_mult,
            display_scale,
        };
        db_ruler::ui(
            &mut ruler_ui,
            track,
            track_id,
            rect,
            db_ruler::DbRulerConfig {
                show_hover_tick: false,
            },
            theme_colors,
            &mut ctx,
        );
    }

    if let Some(rect) = amplitude_rect {
        let mut ruler_ui = component_ui(
            ui,
            track_id,
            "amplitude_ruler",
            rect,
            egui::Layout::top_down(egui::Align::Min),
        );
        let mut ctx = value_ruler2::ValueRulerContext {
            actions: &mut model.actions,
            hover_info: &hover_info,
            audio: &model.audio,
            pan_y_mult,
            zoom_y_mult,
            display_scale,
        };
        value_ruler2::ui(
            &mut ruler_ui,
            track,
            track_id,
            rect,
            value_ruler2::ValueRulerConfig {
                show_hover_tick: false,
            },
            theme_colors,
            &mut ctx,
        );
    }
}

fn ui_stats_grid(
    ui: &mut egui::Ui,
    track_id: TrackId,
    stats: crate::model::stats::BufferStats,
    show_peak_marker: &mut bool,
) {
    egui::Grid::new(ui.id().with(("stats_grid", track_id)))
        .num_columns(2)
        .spacing([6.0, 3.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label("Range");
            ui.label(format!("{}..{}", stats.range.start, stats.range.end));
            ui.end_row();

            if let Some(rms_db) = stats.rms_db {
                ui.label("RMS");
                ui.label(format!("{rms_db:.2} dB"));
                ui.end_row();
            }

            if let Some(peak) = stats.peak {
                ui.label("Peak dB");
                ui.label(format!("{:.2} dB", peak.magnitude_db));
                ui.end_row();

                ui.label("Peak norm");
                ui.label(format!("{:.6}", peak.magnitude_norm));
                ui.end_row();

                ui.label("Peak raw");
                ui.label(peak.raw.to_string());
                ui.end_row();

                ui.label("Peak index");
                ui.horizontal(|ui| {
                    ui.checkbox(show_peak_marker, "").on_hover_text(
                        "Show a vertical guide at the peak sample index in this track",
                    );
                    ui.label(peak.global_ix.to_string());
                });
                ui.end_row();
            }
        });
}

pub fn ui_header(
    ui: &mut egui::Ui,
    model: &mut Model,
    track_id: TrackId,
    header_rect: egui::Rect,
) -> Result<()> {
    ui.set_clip_rect(ui.clip_rect().intersect(header_rect));
    ui.painter().rect(
        header_rect,
        0.0,
        egui::Color32::TRANSPARENT,
        ui.style().visuals.window_stroke(),
        egui::epaint::StrokeKind::Inside,
    );
    if header_rect.width() <= 0.0 || header_rect.height() <= 0.0 {
        return Ok(());
    }

    let mut text = String::from("track header");
    let mut path_text = None;
    let mut channel_text = None;

    // Build the hover data once per frame, while we still have both the file and the specific
    // channel for this track. The popup renderer below only formats it.
    let hover_info = header_hover_info(model, track_id);

    if let Some((file, channel)) = model.get_file_channel_for_track(track_id) {
        let path = file
            .path
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("unknown");
        path_text = Some(path.to_string());
        channel_text = Some(format!(" - ch {}", channel.ch_ix));
        text = format!("{} - ch {}", path, channel.ch_ix);
    } else if model
        .tracks
        .get_track(track_id)
        .is_some_and(|track| track.diff.is_some())
    {
        text = String::from("Diff");
    }

    let font_id = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .cloned()
        .unwrap_or_else(|| egui::FontId::proportional(8.0));
    let color = ui.style().visuals.text_color();
    let padding = ui.spacing().button_padding;
    let item_spacing = ui.spacing().item_spacing.x.max(2.0);
    let text_size = ui
        .painter()
        .layout_no_wrap("x".to_owned(), font_id.clone(), color)
        .size();
    let button_size = egui::vec2(
        (text_size.x + padding.x * 2.0).min(header_rect.width()),
        (text_size.y + padding.y * 2.0).min(header_rect.height()),
    );
    let right_margin = 10.0_f32.min((header_rect.width() - button_size.x).max(0.0));
    let button_right = header_rect.right() - right_margin;
    let button_rect = egui::Rect::from_min_size(
        egui::pos2(
            button_right - button_size.x,
            header_rect.center().y - button_size.y / 2.0,
        ),
        button_size,
    );

    if ui.put(button_rect, egui::Button::new("x")).clicked() {
        model.actions.push(Action::RemoveTrack(track_id));
    }

    let label_right = (button_rect.left() - item_spacing).max(header_rect.left());
    let label_rect = egui::Rect::from_min_max(
        header_rect.left_top(),
        egui::pos2(label_right, header_rect.bottom()),
    );
    let display_text = if let (Some(path), Some(suffix)) = (path_text, channel_text) {
        truncate_path_keep_basename_to_width(ui, &path, &suffix, label_rect.width())
    } else {
        text
    };
    let galley = ui.painter().layout_no_wrap(display_text, font_id, color);
    let text_pos = egui::pos2(
        label_rect.left() + 2.0,
        header_rect.center().y - galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, galley, color);

    if let Some(hover_info) = hover_info {
        let response = ui.interact(
            label_rect,
            ui.id().with(("header_label", track_id)),
            egui::Sense::hover(),
        );
        response.on_hover_ui(move |ui| hover_info.show(ui, track_id));
    }

    Ok(())
}

fn truncate_path_keep_basename_to_width(
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

fn resize_handle(ui: &mut egui::Ui, id: egui::Id, rect: egui::Rect) -> egui::Response {
    let response = ui.interact(rect, id, egui::Sense::drag());
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    response
}
