// Store all app config in one place
use crate::model::{ruler::ValueDisplayScale, shortcuts::ShortcutConfig};
use egui::{Color32, Visuals};

use tracing::{error, info, trace, warn};

pub const APP_NAME: &str = "wavalyze";

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Config {
    /// Factor to multiply with the scroll wheel to zoom over the X-axis
    pub zoom_x_scroll_factor: f32,

    /// Show 'hover info' for each track, which is a floating rectangle over each track at the
    /// mouse position
    pub show_hover_info: bool,

    pub tracks_width_info: f32,
    pub value_display_scale: ValueDisplayScale,
    pub shortcuts: ShortcutConfig,
    pub selection: SelectionConfig,
    pub track: TrackConfig,
    pub colors: ColorPaletteSet,
}

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
        Self { min_height: 10.0 }
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
            accent: Color32::LIGHT_BLUE,
            waveform: Color32::LIGHT_RED,
            waveform_hovered_sample: Color32::WHITE,
            waveform_selection_fill: Color32::from_rgba_unmultiplied(211, 211, 211, 13),
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
            zoom_x_scroll_factor: 4.0,
            show_hover_info: true,
            tracks_width_info: 150.0,
            value_display_scale: ValueDisplayScale::default(),
            shortcuts: ShortcutConfig::default(),
            selection: SelectionConfig::default(),
            track: TrackConfig::default(),
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

    /// Load config from file or use default
    /// Creates the config file if it doesn't exist.
    pub fn load_from_storage_or_default() -> Self {
        let mut user_config: Self = confy::load(APP_NAME, None).unwrap_or_else(|e| {
            warn!(error = %e, "Failed to load config, using defaults");
            Default::default()
        });
        user_config.shortcuts.normalize();
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
}
