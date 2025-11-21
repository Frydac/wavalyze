// Store all app config in one place

#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub zoom_x_factor: f32,
    pub show_hover_info: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { zoom_x_factor: 4.0, show_hover_info: true }
    }
}
