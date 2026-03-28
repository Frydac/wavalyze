use eframe::wasm_bindgen::JsCast as _;

#[wasm_bindgen::prelude::wasm_bindgen(js_name = startWebApp)]
pub fn start_web_app() {
    console_error_panic_hook::set_once();
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    log::info!("Starting wasm app");

    let Some(window) = web_sys::window() else {
        log::error!("Failed to start web app: no window");
        return;
    };
    let Some(document) = window.document() else {
        log::error!("Failed to start web app: no document");
        return;
    };

    let Some(canvas) = document.get_element_by_id("the_canvas_id") else {
        log::error!("Failed to start web app: missing the_canvas_id");
        return;
    };
    let Ok(canvas) = canvas.dyn_into::<web_sys::HtmlCanvasElement>() else {
        log::error!("Failed to start web app: the_canvas_id was not a HtmlCanvasElement");
        return;
    };

    wasm_bindgen_futures::spawn_local(async move {
        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                eframe::WebOptions::default(),
                Box::new(|cc| Ok(Box::new(crate::App::new_web(cc)))),
            )
            .await;

        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => loading_text.remove(),
                Err(error) => {
                    loading_text.set_inner_html(
                        "<p>The app has crashed. See the developer console for details.</p>",
                    );
                    log::error!("Failed to start eframe: {error:?}");
                }
            }
        } else if let Err(error) = start_result {
            log::error!("Failed to start eframe: {error:?}");
        }
    });
}
