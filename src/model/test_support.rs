//! Shared fixtures for `model` unit tests, factored out so each domain module's `#[cfg(test)] mod
//! tests` can build the same `Model`/`File`/`LoadedFile` scaffolding without duplicating it.

use std::collections::BTreeMap;

use crate::audio::{self, thumbnail::ThumbnailE};
use crate::model::Model;
use crate::wav::{self, file};

pub(crate) fn add_buffer(model: &mut Model) -> audio::BufferId {
    model
        .audio
        .buffers
        .insert(std::sync::Arc::new(audio::buffer::BufferE::F32(
            audio::buffer::Buffer::with_size(48_000, 32, 16),
        )))
}

pub(crate) fn make_file(buffers: &[audio::BufferId]) -> file::File {
    let channels = buffers
        .iter()
        .enumerate()
        .map(|(ch_ix, buffer_id)| {
            (
                ch_ix as wav::read::ChIx,
                file::Channel {
                    ch_ix: ch_ix as wav::read::ChIx,
                    buffer_id: *buffer_id,
                    channel_id: None,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    file::File {
        total_nr_channels: channels.len(),
        channels,
        sample_type: audio::SampleType::Float,
        bit_depth: 32,
        sample_rate: 48_000,
        layout: None,
        path: None,
        nr_samples: 16,
        sample_ix_offset: 0,
    }
}

pub(crate) fn loaded_file_with_one_buffer(
    buffer: audio::buffer::BufferE,
    sample_ix_offset: audio::sample::Ix,
) -> wav::read::LoadedFile {
    loaded_file_with_buffers(std::slice::from_ref(&buffer), sample_ix_offset)
}

pub(crate) fn loaded_file_with_buffers(
    buffers: &[audio::buffer::BufferE],
    sample_ix_offset: audio::sample::Ix,
) -> wav::read::LoadedFile {
    let mut channels = BTreeMap::new();
    let mut thumbnails = BTreeMap::new();
    let mut nr_samples = 0;
    for (ch_ix, buffer) in buffers.iter().enumerate() {
        channels.insert(ch_ix, buffer.clone());
        thumbnails.insert(ch_ix, ThumbnailE::from_buffer_e(buffer, None));
        nr_samples = buffer.nr_samples() as u64;
    }
    wav::read::LoadedFile {
        load_id: 0,
        total_nr_channels: channels.len(),
        channels,
        thumbnails,
        sample_type: audio::SampleType::Float,
        bit_depth: 32,
        sample_rate: 48_000,
        layout: None,
        path: None,
        nr_samples,
        sample_ix_offset,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn write_test_wav(name: &str, channels: u16) -> std::path::PathBuf {
    let dir = std::path::PathBuf::from("target/test_output/diff_pairing");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    let spec = hound::WavSpec {
        channels,
        sample_rate: 48_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec).unwrap();
    for _ in 0..channels {
        writer.write_sample(0i16).unwrap();
    }
    writer.finalize().unwrap();
    path
}
