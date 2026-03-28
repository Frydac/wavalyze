use crate::wav;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::mpsc::Sender;

#[derive(Debug)]
pub enum PickerMessage {
    Files(Vec<wav::ReadConfigBytes>),
    #[cfg(not(target_arch = "wasm32"))]
    Error(String),
    Cancelled,
}

pub fn pick_wav_files(tx: Sender<PickerMessage>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let files = rfd::FileDialog::new()
            .add_filter("WAV", &["wav"])
            .pick_files();

        let Some(paths) = files else {
            let _ = tx.send(PickerMessage::Cancelled);
            return;
        };

        load_paths(paths, tx);
    }

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            let files = rfd::AsyncFileDialog::new()
                .add_filter("WAV", &["wav"])
                .pick_files()
                .await;

            let Some(files) = files else {
                let _ = tx.send(PickerMessage::Cancelled);
                return;
            };

            let mut configs = Vec::with_capacity(files.len());
            for file in files {
                let name = Some(file.file_name());
                let bytes = file.read().await;
                configs.push(wav::ReadConfigBytes::new(name, bytes));
            }

            let _ = tx.send(PickerMessage::Files(configs));
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load_paths(paths: Vec<PathBuf>, tx: Sender<PickerMessage>) {
    std::thread::spawn(move || {
        let mut configs = Vec::with_capacity(paths.len());
        for path in paths {
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(err) => {
                    let _ = tx.send(PickerMessage::Error(format!(
                        "Failed to read '{}': {err}",
                        path.display()
                    )));
                    return;
                }
            };
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .map(str::to_owned);
            configs.push(wav::ReadConfigBytes::new(name, bytes));
        }

        let _ = tx.send(PickerMessage::Files(configs));
    });
}
