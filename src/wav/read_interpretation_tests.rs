use super::*;
use crate::audio::{Id, Layout};
use crate::wav::channel_interpretation::{
    ChannelInterpretation, ChannelOrderPreset, InterpretationSource,
};

fn data(buffer: &BufferE) -> &[i16] {
    match buffer {
        BufferE::I16(buffer) => &buffer.data,
        _ => panic!("expected PCM16"),
    }
}

#[test]
fn uploaded_presets_keep_selected_indices_samples_and_offsets() {
    for (mask, preset, expected) in [
        (None, ChannelOrderPreset::Automatic, Some(Id::LeftSurround)),
        (
            Some(0),
            ChannelOrderPreset::Automatic,
            Some(Id::LeftSurround),
        ),
        (
            Some(0x3f),
            ChannelOrderPreset::Automatic,
            Some(Id::LeftBack),
        ),
        (
            Some(0x3f),
            ChannelOrderPreset::AuroIdOrder,
            Some(Id::LeftSurround),
        ),
        (Some(0x3f), ChannelOrderPreset::Unassigned, None),
    ] {
        let interpretation = ChannelInterpretation {
            preset,
            active_layout: None,
        };
        let config = ReadConfigBytes::new(
            Some("upload.wav".into()),
            super::super::channel_mask::tests::wav_bytes(mask, 6, &[]),
        )
        .with_channel_interpretation(interpretation.clone())
        .with_ch_ixs([4])
        .with_sample_ix_offset(17);
        let loaded = read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap();
        assert_eq!(loaded.channel_interpretation, interpretation);
        assert_eq!(loaded.channels.keys().copied().collect::<Vec<_>>(), [4]);
        assert_eq!(data(&loaded.channels[&4]), [4, 10]);
        assert_eq!(loaded.total_nr_channels, 6);
        assert_eq!(loaded.sample_ix_offset, 17);
        assert_eq!(loaded.channel_mask, mask);
        assert!(loaded.source.is_none());
        // Synchronous conversion and worker integration must assign the same roles.
        let other = read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap();
        let mut buffers = Buffers::default();
        let file = loaded.into_file(&mut buffers);
        assert_eq!(file.channels[&4].channel_id, expected);
        assert_eq!(file.channel_interpretation, interpretation);
        assert_eq!(data(&buffers[file.channels[&4].buffer_id]), [4, 10]);
        let mut model = crate::model::Model::new();
        #[cfg(not(target_arch = "wasm32"))]
        model.files.changes.disable_watcher();
        let id = model.add_loaded_file(other).unwrap();
        assert_eq!(model.files[id].channels[&4].channel_id, expected);
        assert_eq!(model.files[id].channel_interpretation, interpretation);
        assert_eq!(model.files[id].sample_ix_offset, 17);
    }
}

#[test]
fn role_changes_preserve_model_audio_and_track_state() {
    let config = ReadConfigBytes::new(
        None,
        super::super::channel_mask::tests::wav_bytes(None, 3, &[]),
    )
    .with_sample_ix_offset(9);
    let mut model = crate::model::Model::new();
    #[cfg(not(target_arch = "wasm32"))]
    model.files.changes.disable_watcher();
    let id = model
        .add_loaded_file(read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap())
        .unwrap();
    let original = model.files[id].clone();
    let tracks = model.tracks.tracks_order.clone();
    let buffers: Vec<_> = original
        .channels
        .values()
        .map(|ch| (ch.buffer_id, model.audio.buffers[ch.buffer_id].clone()))
        .collect();
    let interpretation = ChannelInterpretation {
        preset: ChannelOrderPreset::AuroIdOrder,
        active_layout: Some(Layout::LAYOUT_1_0),
    };
    model
        .files
        .get_mut(id)
        .unwrap()
        .set_channel_interpretation(interpretation.clone());
    assert_eq!(model.files[id].layout, Some(Layout::LAYOUT_1_0));
    let resolved = model.files[id].resolved_channel_interpretation();
    assert!(resolved.channels[0].padding);
    assert!(resolved.channels[1].padding);
    assert!(!resolved.channels[2].padding);
    assert_eq!(resolved.channels[2].id, Some(Id::Center));
    model
        .files
        .get_mut(id)
        .unwrap()
        .set_channel_interpretation(ChannelInterpretation {
            preset: ChannelOrderPreset::Unassigned,
            active_layout: interpretation.active_layout,
        });
    assert_eq!(model.files[id].layout, None);
    assert_eq!(model.files[id].channel_interpretation.active_layout, None);
    assert!(
        model.files[id]
            .channels
            .values()
            .all(|ch| ch.channel_id.is_none())
    );
    assert_eq!(
        model.files[id].channels.keys().collect::<Vec<_>>(),
        original.channels.keys().collect::<Vec<_>>()
    );
    assert_eq!(model.files[id].sample_ix_offset, original.sample_ix_offset);
    assert_eq!(model.files[id].channel_mask, original.channel_mask);
    assert_eq!(model.tracks.tracks_order, tracks);
    for (buffer_id, buffer) in buffers {
        assert!(std::sync::Arc::ptr_eq(
            &model.audio.buffers[buffer_id],
            &buffer
        ));
        assert_eq!(
            model.find_track_id_for_buffer(buffer_id).map(|id| model
                .tracks
                .get_track(id)
                .unwrap()
                .single
                .sample_ix_offset),
            Some(9.0)
        );
    }
}

#[test]
fn uploaded_config_serialization_retains_the_interpretation() {
    let config =
        ReadConfigBytes::new(None, vec![]).with_channel_interpretation(ChannelInterpretation {
            preset: ChannelOrderPreset::AuroIdOrder,
            active_layout: Some(Layout::LAYOUT_1_0),
        });
    let encoded = toml::to_string(&config).unwrap();
    assert_eq!(toml::from_str::<ReadConfigBytes>(&encoded).unwrap(), config);
    for preset in [
        ChannelOrderPreset::Automatic,
        ChannelOrderPreset::AuroIdOrder,
        ChannelOrderPreset::Unassigned,
    ] {
        let config =
            ReadConfigBytes::new(None, vec![]).with_channel_interpretation(ChannelInterpretation {
                preset,
                active_layout: None,
            });
        let encoded = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<ReadConfigBytes>(&encoded).unwrap(), config);
    }
    let mut old: toml::Value = toml::from_str(&encoded).unwrap();
    old.as_table_mut().unwrap().remove("channel_interpretation");
    let restored: ReadConfigBytes = old.try_into().unwrap();
    assert_eq!(
        restored.channel_interpretation,
        ChannelInterpretation::default()
    );
}

#[test]
fn selected_layout_is_cleared_when_header_requires_another_interpretation() {
    let config = ReadConfigBytes::new(
        None,
        super::super::channel_mask::tests::wav_bytes(Some(0x7), 3, &[]),
    )
    .with_channel_interpretation(ChannelInterpretation {
        preset: ChannelOrderPreset::Automatic,
        active_layout: Some(Layout::LAYOUT_1_0),
    });
    let loaded = read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap();
    assert_eq!(loaded.channel_interpretation.active_layout, None);
    assert_eq!(loaded.layout, Some(Layout::LAYOUT_3_0));
    assert_eq!(
        loaded
            .channel_interpretation
            .resolve(loaded.channel_mask, loaded.total_nr_channels)
            .source,
        InterpretationSource::Wavex
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn header_peeking_retains_raw_masks_and_total_counts() {
    let directory = std::path::Path::new("target/test_output/channel_metadata");
    std::fs::create_dir_all(directory).unwrap();
    for mask in [None, Some(0), Some(0x3f), Some(0x1000000)] {
        let path = directory.join(format!("header_{mask:?}.wav"));
        std::fs::write(
            &path,
            super::super::channel_mask::tests::wav_bytes(mask, 6, &[]),
        )
        .unwrap();
        assert_eq!(
            peek_channel_metadata(&path).unwrap(),
            WavChannelMetadata {
                total_nr_channels: 6,
                channel_mask: mask
            }
        );
    }
}
