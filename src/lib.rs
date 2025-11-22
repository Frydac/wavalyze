#![warn(clippy::all, rust_2018_idioms)]
// TODO: remove
#![allow(dead_code)]
#![allow(unused_variables)]

// mod app;
pub mod app;
pub mod app_config;
pub mod audio;
pub mod generator;
pub mod id;
pub mod log;
pub mod math;
pub mod model;
pub mod pos;
pub mod rect;
pub mod sample;
pub mod util;
pub mod view;
pub mod wav;

// Code used only for test builds/configs
#[cfg(test)]
pub mod test_utils;

pub use app::App;
pub use app_config::AppCliConfig;
pub use pos::Pos;
