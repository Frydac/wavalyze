#![warn(clippy::all, rust_2018_idioms)]
// TODO: remove
// #![allow(dead_code)]
#![allow(unused_variables)]

// mod app;
pub mod app;
pub mod args;
pub mod audio;
pub mod generator;
pub mod log;
pub mod math;
pub mod model;
pub mod pos;
pub mod rect;
pub mod sample;
pub mod util;
pub mod view;
pub mod wav;
#[cfg(target_arch = "wasm32")]
mod web;

// Code used only for test builds/configs
#[cfg(test)]
pub mod test_utils;

pub use app::App;
pub use pos::Pos;
#[cfg(target_arch = "wasm32")]
pub use wasm_bindgen_rayon::init_thread_pool;
#[cfg(target_arch = "wasm32")]
pub use web::start_web_app;
