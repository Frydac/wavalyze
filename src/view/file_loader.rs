//! Bridge platform file pickers and native path drops to the view's loading queue.
//!
//! Native results keep full paths so the model can watch and reload them through its path
//! jobs. Browser results carry uploaded bytes instead. This module collects those inputs;
//! decoding and model integration belong to the existing load actions and workers.

#[cfg(target_arch = "wasm32")]
use crate::wav;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
use std::sync::mpsc::Sender;

/// Completed platform input handed back to the view for queuing the appropriate load actions.
/// Cancellation is explicit so the view can also clear its pending-picker state.
#[derive(Debug)]
pub enum PickerMessage {
    #[cfg(target_arch = "wasm32")]
    Files(Vec<wav::ReadConfigBytes>),
    #[cfg(not(target_arch = "wasm32"))]
    Paths(Vec<PathBuf>),
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
    let _ = tx.send(PickerMessage::Paths(paths));
}
