use crate::{
    AppCliConfig,
    args::{self, Args},
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
    cli_config: Option<AppCliConfig>,

    args: Option<Args>,
}

impl Default for App {
    fn default() -> Self {
        let model = model::Model::new();
        Self {
            view: view::View::new(model),
            cli_config: None,
            args: None,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // draw ui, and measure frame time
        self.view.ui_measured(ctx, frame);
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

impl App {
    pub fn new2(_cc: &eframe::CreationContext<'_>, args: Args, user_config: model::Config) -> Self {
        let mut model = model::Model {
            user_config,
            ..Default::default()
        };
        let mut open_files = |files: &[ReadConfig]| {
            for file_read_config in files {
                model
                    .actions
                    .push(Action::OpenFile(file_read_config.clone()));
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
                    // todo!("diff files");
                }
            },
        }

        Self {
            view: view::View::new(model),
            cli_config: None,
            args: Some(args),
        }
    }

    pub fn new_web(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut model = model::Model::new();
        model.actions.push(Action::LoadDemo);
        model.actions.push(Action::ZoomToFull);
        model.actions.push(Action::FillScreenHeight);

        Self {
            view: view::View::new(model),
            cli_config: None,
            args: None,
        }
    }
}
