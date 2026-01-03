// Store all app config in one place

use tracing::{error, info, trace, warn};

pub const APP_NAME: &str = "wavalyze";

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Config {
    /// Factor to multiply with the scroll wheel to zoom over the X-axis
    pub zoom_x_scroll_factor: f32,

    /// Show 'hover info' for each track, which is a floating rectangle over each track at the
    /// mouse position
    pub show_hover_info: bool,

    pub track: TrackConfig,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TrackConfig {
    pub min_height: f32,
}

impl Default for TrackConfig {
    fn default() -> Self {
        Self { min_height: 150.0 }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            zoom_x_scroll_factor: 4.0,
            show_hover_info: true,
            track: TrackConfig::default(),
        }
    }
}

impl Config {
    /// Load config from file or use default
    /// Creates the config file if it doesn't exist.
    pub fn load_from_storage_or_default() -> Self {
        let user_config: Self = confy::load(APP_NAME, None).unwrap_or_else(|e| {
            warn!(error = %e, "Failed to load config, using defaults");
            Default::default()
        });
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
