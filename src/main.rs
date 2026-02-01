#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> anyhow::Result<()> {
    use clap::Parser;
    use wavalyze::{self, log, model};

    // fn main() -> eframe::Result {
    let args2 = wavalyze::args::Args::parse();

    log::init_tracing(args2.log_level.as_deref())?;

    // let args = wavalyze::AppCliConfig::parse();
    let user_config = model::Config::load_from_storage_or_default();

    let eframe_native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([480.0, 320.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/wavalyze_icon_001.png")[..]).expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        "wavalyze",
        eframe_native_options,
        Box::new(|cc| Ok(Box::new(wavalyze::App::new2(cc, args2, user_config)))),
    ) {
        tracing::error!("Error: {}", err);
        std::process::exit(1);
    }

    Ok(())
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    console_error_panic_hook::set_once();
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    log::info!("Starting wasm app");
    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().expect("No window").document().expect("No document");

        let _canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(_canvas, web_options, Box::new(|cc| Ok(Box::new(wavalyze::App::new_web(cc)))))
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html("<p> The app has crashed. See the developer console for details. </p>");
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
