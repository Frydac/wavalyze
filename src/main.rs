#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use wavalyze::{self, log, model};

    // fn main() -> eframe::Result {
    let args2 = wavalyze::args::Args::parse();

    let tracing_collector = log::init_tracing(args2.log_level.as_deref())?;

    // let args = wavalyze::AppCliConfig::parse();
    let user_config = model::Config::load_from_storage_or_default();

    let eframe_native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([480.0, 320.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(
                    &include_bytes!("../assets/wavalyze_icon_001.png")[..],
                )
                .expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "wavalyze",
        eframe_native_options,
        Box::new(move |cc| {
            Ok(Box::new(wavalyze::App::new_native(
                cc,
                args2,
                user_config,
                tracing_collector,
            )))
        }),
    ) {
        tracing::error!("Error: {}", err);
        std::process::exit(1);
    }

    Ok(())
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {}
