use crate::{model, view, AppCliConfig};
use eframe::egui;

#[derive(Debug)]
pub struct App {
    #[allow(dead_code)]
    // TODO: probably dont need the SharedModel here, we can just pass the model around I think
    // also doesn't need to be a member, we can just use the run function to create them on the
    // stack?
    model: model::SharedModel,

    view: view::View,

    #[allow(dead_code)]
    cli_config: AppCliConfig,
}

impl Default for App {
    fn default() -> Self {
        let model = model::Model::default_shared();
        Self {
            model: model.clone(),
            view: view::View::new(model),
            cli_config: AppCliConfig::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {

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
        self.view.ui(ctx, frame);
    }

    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.model.borrow().config.save_to_storage();
        // self.save_user_config();
        // self.model.save_to_storage(storage);
    }
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, cli_config: AppCliConfig, user_config: model::Config) -> Self {
        let model = model::SharedModel::default();

        model.borrow_mut().config = user_config;

        // Diff gets precedence over audio files, we ignore any extra audio files if we have the
        // diff option set
        if let Some(ref diff_files) = cli_config.diff {
            for diff_file in diff_files {
                model
                    .borrow_mut()
                    .add_wav_file(&diff_file.path, diff_file.channel, diff_file.offset)
                    .unwrap_or_else(|err| eprintln!("Failed to add wav file: {}", err));
            }
        } else {
            for path in &cli_config.audio_files {
                model
                    .borrow_mut()
                    .add_wav_file(path, None, None)
                    .unwrap_or_else(|err| eprintln!("Failed to add wav file: {}", err));
            }
        }

        println!(
            "app after adding files model.borrow().tracks.len(): {}",
            model.borrow().tracks.len()
        );

        Self {
            model: model.clone(),
            view: view::View::new(model),
            cli_config,
        }
    }

    // fn save_user_config(&mut self) {
    //     self.user_config.save_to_storage();
    // }
}
