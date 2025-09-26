use crate::{model, view, AppConfig};
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
    config: AppConfig,
}

impl Default for App {
    fn default() -> Self {
        let model = model::Model::default_shared();
        Self {
            model: model.clone(),
            view: view::View::new(model),
            config: AppConfig::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.view.update(ctx, frame);
    }
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>, config: AppConfig) -> Self {
        let model = model::SharedModel::default();

        // Diff gets precedence over audio files, we ignore any extra audio files if we have the
        // diff option set
        if let Some(ref diff_files) = config.diff {
            for diff_file in diff_files {
                model
                    .borrow_mut()
                    .add_wav_file(&diff_file.path, diff_file.channel, diff_file.offset)
                    .unwrap_or_else(|err| eprintln!("Failed to add wav file: {}", err));
            }
        } else {
            for path in &config.audio_files {
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

        App {
            model: model.clone(),
            view: view::View::new(model),
            config,
        }
    }
}

// impl App {
//     pub fn new() -> Self {
//         Self {
//             model: model::Model::new(),
//             view: view::View::new(),
//         }
//     }
// }
