// Store all app config in one place

use tracing::{error, info, warn};

pub const APP_NAME: &str = "wavalyze";

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub zoom_x_factor: f32,
    pub show_hover_info: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            zoom_x_factor: 4.0,
            show_hover_info: true,
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
            info!(
                "Config saved to {}: {self:#?}",
                confy::get_configuration_file_path("wavalyze", None)
                    .as_ref()
                    .map(|p| format!("{p:?}"))
                    .unwrap_or("<failed to get path>".into())
            );
        }
    }
}
