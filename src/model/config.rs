// Store all app config in one place
use crate::model::{ruler::ValueDisplayScale, shortcuts::ShortcutConfig};
use egui::{Color32, Visuals};

#[cfg(not(target_arch = "wasm32"))]
use tracing::{error, info, trace, warn};

pub const APP_NAME: &str = "wavalyze";

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Config {
    /// Show 'hover info' for each track, which is a floating rectangle over each track at the
    /// mouse position
    pub show_hover_info: bool,

    pub tracks_width_info: f32,
    /// Block size assigned to a new app session.
    pub default_block_size: u64,
    /// Show the per-track amplitude ruler (right-most slot in the track side panel).
    #[serde(default = "default_true")]
    pub show_amplitude_ruler: bool,
    /// Show the per-track decibel ruler. When also showing the amplitude ruler, the side panel
    /// widens by one ruler slot.
    #[serde(default)]
    pub show_db_ruler: bool,
    /// Round zoomed-out min/max waveform columns to pixel centers. This can look crisper at some
    /// zoom levels, but raw positions behave better on fractional display scaling.
    #[serde(default)]
    pub round_minmax_waveform_to_pixel_center: bool,
    pub value_display_scale: ValueDisplayScale,
    /// Scroll-wheel pan/zoom sensitivity and direction, per axis.
    pub navigation: NavigationConfig,
    pub shortcuts: ShortcutConfig,
    pub selection: SelectionConfig,
    pub track: TrackConfig,
    pub view: ViewConfig,
    pub colors: ColorPaletteSet,
}

fn default_true() -> bool {
    true
}

/// Width of a single value/dB ruler column inside a track's side panel.
pub const RULER_SLOT_WIDTH: f32 = 80.0;

/// Minimum width for non-ruler controls in the per-track side panel.
pub const TRACK_SIDE_CONTROLS_MIN_WIDTH: f32 = 170.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize)]
pub enum StartEditMode {
    #[default]
    KeepEnd,
    KeepLength,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct SelectionConfig {
    pub start_edit_mode: StartEditMode,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TrackConfig {
    pub min_height: f32,
    /// Whether a new app session starts with visible tracks sharing the viewport height equally.
    #[serde(default = "default_true")]
    pub equal_height_layout_by_default: bool,
}

/// Scroll-wheel navigation sensitivity and direction for the waveform, per axis.
///
/// Each axis has a `*_factor` (multiplier applied to the scroll delta) and an `invert_*`
/// boolean (flips the scroll direction). These apply to scroll-wheel pan/zoom only; mouse-drag
/// panning stays direct 1:1 manipulation. Use the `*_mult()` helpers to get the signed
/// multiplier (factor, negated when inverted).
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct NavigationConfig {
    /// Pan left/right in time.
    pub pan_x_factor: f32,
    /// Pan up/down in sample value.
    pub pan_y_factor: f32,
    /// Zoom in/out in time.
    pub zoom_x_factor: f32,
    /// Zoom in/out in sample value.
    pub zoom_y_factor: f32,
    /// Height in pixels around sample value zero where ruler zoom anchors to zero. Zero disables it.
    pub zoom_y_zero_deadzone_height: f32,
    pub invert_pan_x: bool,
    pub invert_pan_y: bool,
    pub invert_zoom_x: bool,
    pub invert_zoom_y: bool,
}

impl Default for NavigationConfig {
    fn default() -> Self {
        Self {
            // Pan factors default to 1.0 (raw scroll delta, unchanged from previous behaviour).
            pan_x_factor: 1.0,
            pan_y_factor: 1.0,
            // Zoom factors default to 4.0 (the previous `zoom_x_scroll_factor` default).
            zoom_x_factor: 4.0,
            zoom_y_factor: 4.0,
            zoom_y_zero_deadzone_height: 16.0,
            invert_pan_x: false,
            invert_pan_y: false,
            invert_zoom_x: false,
            invert_zoom_y: false,
        }
    }
}

impl NavigationConfig {
    fn signed(factor: f32, invert: bool) -> f32 {
        if invert { -factor } else { factor }
    }

    pub fn pan_x_mult(&self) -> f32 {
        Self::signed(self.pan_x_factor, self.invert_pan_x)
    }
    pub fn pan_y_mult(&self) -> f32 {
        Self::signed(self.pan_y_factor, self.invert_pan_y)
    }
    pub fn zoom_x_mult(&self) -> f32 {
        Self::signed(self.zoom_x_factor, self.invert_zoom_x)
    }
    pub fn zoom_y_mult(&self) -> f32 {
        Self::signed(self.zoom_y_factor, self.invert_zoom_y)
    }
}

/// Visibility and layout settings for the main view panels.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ViewConfig {
    /// Show the right side panel (settings, FPS, jobs, ruler/hover/selection info).
    pub show_right_panel: bool,
}

impl Default for ViewConfig {
    fn default() -> Self {
        Self {
            show_right_panel: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ColorPaletteSet {
    pub dark: ThemeColors,
    pub light: ThemeColors,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct ThemeColors {
    pub accent: Color32,
    pub waveform: Color32,
    pub waveform_hovered_sample: Color32,
    #[serde(alias = "selection_fill")]
    pub waveform_selection_fill: Color32,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            start_edit_mode: StartEditMode::KeepEnd,
        }
    }
}

impl Default for TrackConfig {
    fn default() -> Self {
        Self {
            min_height: 10.0,
            equal_height_layout_by_default: true,
        }
    }
}

impl Default for ColorPaletteSet {
    fn default() -> Self {
        Self {
            dark: ThemeColors::dark_default(),
            light: ThemeColors::light_default(),
        }
    }
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self::dark_default()
    }
}

impl ThemeColors {
    pub fn dark_default() -> Self {
        Self {
            accent: Color32::LIGHT_GRAY,
            waveform: Color32::LIGHT_RED,
            waveform_hovered_sample: Color32::WHITE,
            waveform_selection_fill: Color32::from_rgba_unmultiplied(211, 211, 211, 15),
        }
    }

    pub fn light_default() -> Self {
        Self {
            accent: Color32::from_rgb(0, 102, 204),
            waveform: Color32::from_rgb(196, 64, 64),
            waveform_hovered_sample: Color32::from_rgb(32, 32, 32),
            waveform_selection_fill: Color32::from_rgba_unmultiplied(0, 102, 204, 28),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            navigation: NavigationConfig::default(),
            show_hover_info: true,
            tracks_width_info: 250.0,
            default_block_size: 1024,
            show_amplitude_ruler: true,
            show_db_ruler: false,
            round_minmax_waveform_to_pixel_center: true,
            value_display_scale: ValueDisplayScale::default(),
            shortcuts: ShortcutConfig::default(),
            selection: SelectionConfig::default(),
            track: TrackConfig::default(),
            view: ViewConfig::default(),
            colors: ColorPaletteSet::default(),
        }
    }
}

impl Config {
    pub fn reset_to_default(&mut self) {
        *self = Self::default();
    }

    pub fn reset_shortcuts_to_default(&mut self) {
        self.shortcuts = ShortcutConfig::default();
    }

    pub fn active_theme_colors(&self, visuals: &Visuals) -> &ThemeColors {
        if visuals.dark_mode {
            &self.colors.dark
        } else {
            &self.colors.light
        }
    }

    /// Extra horizontal pixels the side panel needs to fit the enabled rulers. The base
    /// `tracks_width_info` already reserves space for one ruler, so each *additional* enabled
    /// ruler past the first costs another slot.
    pub fn ruler_stack_extra_width(&self) -> f32 {
        let count = self.show_amplitude_ruler as u32 + self.show_db_ruler as u32;
        RULER_SLOT_WIDTH * count.saturating_sub(1) as f32
    }

    /// Total width the per-track side panel should claim, given the enabled rulers.
    pub fn effective_tracks_width_info(&self) -> f32 {
        let ruler_count = self.show_amplitude_ruler as u32 + self.show_db_ruler as u32;
        let min_width = TRACK_SIDE_CONTROLS_MIN_WIDTH + RULER_SLOT_WIDTH * ruler_count as f32;
        (self.tracks_width_info + self.ruler_stack_extra_width()).max(min_width)
    }

    /// Load config from file or use default
    /// Creates the config file if it doesn't exist.
    pub fn load_from_storage_or_default() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let mut user_config: Self = confy::load(APP_NAME, None).unwrap_or_else(|e| {
            warn!(error = %e, "Failed to load config, using defaults");
            Default::default()
        });
        #[cfg(target_arch = "wasm32")]
        let mut user_config = Self::default();

        user_config.shortcuts.normalize();

        #[cfg(not(target_arch = "wasm32"))]
        info!(
            "Config loaded from {}: {user_config:#?}",
            confy::get_configuration_file_path("wavalyze", None)
                .as_ref()
                .map(|p| format!("{p:?}"))
                .unwrap_or("<failed to get path>".into())
        );

        user_config
    }

    pub fn save_to_storage(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(e) = confy::store(APP_NAME, None, self) {
            error!(error = %e, "Failed to save config");
        } else {
            // Using trace here as it gets saved often and prints a lot of info
            trace!(
                "Config saved to {}: {self:#?}",
                confy::get_configuration_file_path("wavalyze", None)
                    .as_ref()
                    .map(|p| format!("{p:?}"))
                    .unwrap_or("<failed to get path>".into())
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ColorPaletteSet, Config, ThemeColors};
    use crate::model::{
        ruler::ValueDisplayScale,
        shortcuts::{ShortcutAction, ShortcutScope},
    };
    use egui::Color32;

    #[test]
    fn default_config_has_shortcuts() {
        let config = Config::default();

        assert_eq!(
            config.shortcuts.bindings.len(),
            ShortcutAction::ALL.len() * ShortcutScope::ALL.len()
        );
    }

    #[test]
    fn equal_height_layout_is_enabled_by_default() {
        assert!(Config::default().track.equal_height_layout_by_default);
    }

    #[test]
    fn default_block_size_is_1024() {
        assert_eq!(Config::default().default_block_size, 1024);
    }

    #[test]
    fn old_config_without_default_block_size_uses_default() {
        let config: Config =
            toml::from_str("show_hover_info = true\ntracks_width_info = 120.0\n").unwrap();

        assert_eq!(config.default_block_size, 1024);
    }

    #[test]
    fn old_track_config_without_equal_height_default_enables_it() {
        let config: Config = toml::from_str("[track]\nmin_height = 25.0\n").unwrap();

        assert!(config.track.equal_height_layout_by_default);
    }

    #[test]
    fn old_config_without_navigation_uses_defaults() {
        let config: Config =
            toml::from_str("show_hover_info = true\ntracks_width_info = 120.0\n").unwrap();

        assert_eq!(config.navigation, super::NavigationConfig::default());
        assert_eq!(config.navigation.zoom_y_zero_deadzone_height, 16.0);
    }

    #[test]
    fn default_config_serializes_to_toml() {
        // TOML requires scalar fields before tables; a misplaced struct field breaks `confy`
        // saving. Guard the round-trip so field reordering can't regress config persistence.
        let toml = toml::to_string(&Config::default()).expect("serialize default config");
        let parsed: Config = toml::from_str(&toml).expect("parse serialized config");
        assert_eq!(parsed, Config::default());
    }

    #[test]
    fn navigation_invert_negates_multiplier() {
        let nav = super::NavigationConfig {
            zoom_x_factor: 4.0,
            invert_zoom_x: true,
            pan_y_factor: 2.0,
            ..super::NavigationConfig::default()
        };

        assert_eq!(nav.zoom_x_mult(), -4.0);
        assert_eq!(nav.pan_y_mult(), 2.0);
    }

    #[test]
    fn old_config_without_shortcuts_uses_defaults() {
        let config: Config = toml::from_str(
            "zoom_x_scroll_factor = 2.0\nshow_hover_info = true\ntracks_width_info = 120.0\n",
        )
        .unwrap();

        assert_eq!(config.value_display_scale, ValueDisplayScale::default());
        assert_eq!(
            config.shortcuts.bindings.len(),
            ShortcutAction::ALL.len() * ShortcutScope::ALL.len()
        );
        assert_eq!(config.colors, ColorPaletteSet::default());
    }

    #[test]
    fn old_config_without_colors_uses_defaults() {
        let config: Config = toml::from_str(
            "zoom_x_scroll_factor = 2.0\nshow_hover_info = true\ntracks_width_info = 120.0\n",
        )
        .unwrap();

        assert_eq!(config.colors, ColorPaletteSet::default());
    }

    #[test]
    fn partial_color_config_falls_back_missing_fields() {
        let dark_colors = toml::Value::try_from(ThemeColors {
            accent: Color32::from_rgb(1, 2, 3),
            ..ThemeColors::dark_default()
        })
        .unwrap();
        let light_colors = toml::Value::try_from(ThemeColors::light_default()).unwrap();
        let dark_table = dark_colors.as_table().unwrap();
        let light_table = light_colors.as_table().unwrap();
        let mut config_table = toml::map::Map::new();
        config_table.insert(
            String::from("zoom_x_scroll_factor"),
            toml::Value::Float(2.0),
        );
        config_table.insert(String::from("show_hover_info"), toml::Value::Boolean(true));
        config_table.insert(String::from("tracks_width_info"), toml::Value::Float(120.0));
        let mut dark_partial = toml::map::Map::new();
        dark_partial.insert(
            String::from("accent"),
            dark_table.get("accent").cloned().unwrap(),
        );
        let mut colors_table = toml::map::Map::new();
        colors_table.insert(String::from("dark"), toml::Value::Table(dark_partial));
        colors_table.insert(
            String::from("light"),
            toml::Value::Table(light_table.clone()),
        );
        config_table.insert(String::from("colors"), toml::Value::Table(colors_table));

        let config: Config = toml::Value::Table(config_table).try_into().unwrap();

        assert_eq!(config.colors.dark.accent, Color32::from_rgb(1, 2, 3));
        assert_eq!(
            config.colors.dark.waveform,
            ThemeColors::dark_default().waveform
        );
        assert_eq!(config.colors.light, ThemeColors::light_default());
    }

    #[test]
    fn old_selection_fill_name_is_still_accepted() {
        let config: Config = toml::from_str(
            r#"
zoom_x_scroll_factor = 2.0
show_hover_info = true
tracks_width_info = 120.0

[colors.dark]
selection_fill = [1, 2, 3, 4]
"#,
        )
        .unwrap();

        assert_eq!(
            config.colors.dark.waveform_selection_fill,
            Color32::from_rgba_premultiplied(1, 2, 3, 4)
        );
    }

    #[test]
    fn default_config_has_distinct_dark_and_light_palettes() {
        let config = Config::default();

        assert_ne!(config.colors.dark, config.colors.light);
    }

    #[test]
    fn old_config_without_ruler_flags_defaults_amplitude_on_and_db_off() {
        let config: Config = toml::from_str(
            "zoom_x_scroll_factor = 2.0\nshow_hover_info = true\ntracks_width_info = 120.0\n",
        )
        .unwrap();

        assert!(config.show_amplitude_ruler);
        assert!(!config.show_db_ruler);
    }

    #[test]
    fn old_config_without_waveform_rounding_flag_defaults_off() {
        let config: Config = toml::from_str(
            "zoom_x_scroll_factor = 2.0\nshow_hover_info = true\ntracks_width_info = 120.0\n",
        )
        .unwrap();

        assert!(!config.round_minmax_waveform_to_pixel_center);
    }

    #[test]
    fn ruler_stack_extra_width_only_adds_for_second_ruler() {
        let mut config = Config::default();
        assert_eq!(config.ruler_stack_extra_width(), 0.0);
        config.show_db_ruler = true;
        assert_eq!(config.ruler_stack_extra_width(), super::RULER_SLOT_WIDTH);
        config.show_amplitude_ruler = false;
        assert_eq!(config.ruler_stack_extra_width(), 0.0);
        config.show_db_ruler = false;
        assert_eq!(config.ruler_stack_extra_width(), 0.0);
    }

    #[test]
    fn effective_tracks_width_info_keeps_room_for_controls_and_enabled_rulers() {
        let mut config = Config {
            tracks_width_info: 150.0,
            ..Config::default()
        };

        assert_eq!(
            config.effective_tracks_width_info(),
            super::TRACK_SIDE_CONTROLS_MIN_WIDTH + super::RULER_SLOT_WIDTH
        );

        config.show_db_ruler = true;
        assert_eq!(
            config.effective_tracks_width_info(),
            super::TRACK_SIDE_CONTROLS_MIN_WIDTH + 2.0 * super::RULER_SLOT_WIDTH
        );

        config.show_amplitude_ruler = false;
        assert_eq!(
            config.effective_tracks_width_info(),
            super::TRACK_SIDE_CONTROLS_MIN_WIDTH + super::RULER_SLOT_WIDTH
        );

        config.show_db_ruler = false;
        assert_eq!(
            config.effective_tracks_width_info(),
            super::TRACK_SIDE_CONTROLS_MIN_WIDTH
        );
    }

    #[test]
    fn effective_tracks_width_info_preserves_larger_user_widths() {
        let config = Config {
            tracks_width_info: 300.0,
            ..Config::default()
        };

        assert_eq!(config.effective_tracks_width_info(), 300.0);
    }
}
