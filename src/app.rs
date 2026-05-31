use crate::{
    args::{self, Args},
    log::TracingCollector,
    model::{self, Action},
    view,
    wav::ReadConfig,
};
use eframe::egui;
use tracing::trace;

#[derive(Debug)]
pub struct App {
    view: view::View,

    #[allow(dead_code)]
    args: Option<Args>,
}

impl Default for App {
    fn default() -> Self {
        let model = model::Model::new();
        Self {
            view: view::View::new(model, TracingCollector::default()),
            args: None,
        }
    }
}

impl eframe::App for App {
    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.view.ui_measured(ui.ctx(), frame);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.view.model().user_config.save_to_storage();
        }
        // self.save_user_config();
        // self.model.save_to_storage(storage);
    }
}

fn handle_cli_arguments(model: &mut model::Model, args: &Args) {
    let mut open_files = |files: &[ReadConfig]| {
        for file_read_config in files {
            model
                .actions
                .push(Action::OpenFilePath(file_read_config.clone()));
        }
    };
    match args.command {
        None => {
            trace!("No command");
            open_files(&args.files);
        }
        Some(ref command) => match command {
            args::Commands::Open { files } => {
                trace!("Open command");
                open_files(files);
            }
            args::Commands::Diff { file1, file2 } => {
                trace!("Diff command");
                model.actions.push(Action::OpenDiffFilePaths {
                    file_a: file1.clone(),
                    file_b: file2.clone(),
                });
            }
        },
    }
}

impl App {
    pub fn new_native(
        _cc: &eframe::CreationContext<'_>,
        args: Args,
        user_config: model::Config,
        tracing_collector: TracingCollector,
    ) -> Self {
        let mut model = model::Model::default();
        model.user_config = user_config;

        handle_cli_arguments(&mut model, &args);

        Self {
            view: view::View::new(model, tracing_collector),
            args: Some(args),
        }
    }

    pub fn new_web(_cc: &eframe::CreationContext<'_>, tracing_collector: TracingCollector) -> Self {
        let mut model = model::Model::new();
        model.actions.push(Action::LoadDemo);
        model.actions.push(Action::ZoomToFull);
        model.actions.push(Action::FillScreenHeight);

        Self {
            view: view::View::new(model, tracing_collector),
            args: None,
        }
    }
}
