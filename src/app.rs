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
        // static ONCE: std::sync::Once = std::sync::Once::new();
        // ONCE.call_once(|| {
        //     dbg!(&ctx.style().spacing);
        // });

        // if !ctx.input().raw.dropped_files.is_empty() {
        //     println!("files dropped");

        //     let dropped_files = ctx
        //         .input()
        //         .raw
        //         .dropped_files
        //         .clone()
        //         .iter()
        //         .map(|file| file.path.as_ref().unwrap().clone())
        //         .collect::<Vec<PathBuf>>();

        //     dbg!(&dropped_files);
        // }
        // self.view.ui(ctx, frame);
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
                model.actions.push(Action::OpenFile(file_read_config.clone()));
            }
            model.actions.push(Action::ZoomToFull);
            model.actions.push(Action::FillScreenHeight);
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
    // pub fn new(_cc: &eframe::CreationContext<'_>, cli_config: AppCliConfig, user_config: model::Config) -> Self {
    //     let model = model::SharedModel::default();

    //     model.borrow_mut().user_config = user_config;

    //     // Diff gets precedence over audio files, we ignore any extra audio files if we have the
    //     // diff option set
    //     if let Some(ref diff_files) = cli_config.diff {
    //         for diff_file in diff_files {
    //             model
    //                 .borrow_mut()
    //                 .add_wav_file(&diff_file.path, diff_file.channel, diff_file.offset)
    //                 .unwrap_or_else(|err| eprintln!("Failed to add wav file: {}", err));
    //         }
    //     } else {
    //         for path in &cli_config.audio_files {
    //             model
    //                 .borrow_mut()
    //                 .add_wav_file(path, None, None)
    //                 .unwrap_or_else(|err| eprintln!("Failed to add wav file: {}", err));
    //         }
    //     }

    //     println!(
    //         "app after adding files model.borrow().tracks.len(): {}",
    //         model.borrow().tracks.len()
    //     );

    //     Self {
    //         model: model.clone(),
    //         view: view::View::new(model),
    //         cli_config: Some(cli_config),
    //         args: None,
    //     }
    // }

    // fn save_user_config(&mut self) {
    //     self.user_config.save_to_storage();
    // }
}
