// Store all app config in one place
use crate::model::shortcuts::ShortcutConfig;

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
    pub shortcuts: ShortcutConfig,
    pub selection: SelectionConfig,
    pub track: TrackConfig,
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

impl Default for Config {
    fn default() -> Self {
        Self {
            zoom_x_scroll_factor: 4.0,
            show_hover_info: true,
            tracks_width_info: 150.0,
            shortcuts: ShortcutConfig::default(),
            selection: SelectionConfig::default(),
            track: TrackConfig::default(),
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
    use super::Config;
    use crate::model::shortcuts::{ShortcutAction, ShortcutScope};

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

        assert_eq!(
            config.shortcuts.bindings.len(),
            ShortcutAction::ALL.len() * ShortcutScope::ALL.len()
        );
    }
}
