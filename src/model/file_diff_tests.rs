use super::*;
use crate::audio::Id;
use crate::model::Action;
use crate::wav::channel_interpretation::ChannelOrderPreset;

fn add_file(
    model: &mut Model,
    name: &str,
    count: u16,
    mask: Option<u32>,
    indices: &[usize],
) -> FileId {
    let config = wav::ReadConfigBytes::new(
        Some(name.into()),
        wav::channel_mask::tests::wav_bytes(mask, count, &[]),
    )
    .with_ch_ixs(indices.to_vec());
    model
        .add_loaded_file(wav::read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap())
        .unwrap()
}

fn fixture() -> (Model, FileId, FileId, FileId) {
    let mut model = Model::new();
    #[cfg(not(target_arch = "wasm32"))]
    model.files.changes.disable_watcher();
    let a = add_file(&mut model, "A.wav", 3, None, &[0, 2]);
    let x = add_file(&mut model, "unrelated.wav", 2, None, &[0]);
    let b = add_file(&mut model, "B.wav", 6, Some(0x3f), &[1, 4]);
    (model, a, b, x)
}

fn source_track(model: &Model, file: FileId, index: usize) -> track::TrackId {
    model
        .find_track_id_for_buffer(model.files[file].channels[&index].buffer_id)
        .unwrap()
}

fn prepare(model: &mut Model, a: FileId, b: FileId) -> FileDiffJobInput {
    model.open_existing_file_pairing(a, b).unwrap();
    let mut pending = model.pending_diff_pairing.take().unwrap();
    pending.clear();
    pending.checked[1][1] = true; // A2 - B4
    pending.checked[1][0] = true; // A2 - B1 (row-major order)
    model.prepare_existing_file_diff(pending).unwrap().unwrap()
}

fn compute(input: FileDiffJobInput) -> FileDiffResult {
    jobs::diff::compute_file_diff(1, input, &std::sync::mpsc::channel().0).unwrap()
}

#[test]
fn pairing_reuses_loaded_headers_roles_indices_and_is_cancelable_without_changes() {
    let (mut model, a, b, _) = fixture();
    let order = model.tracks.tracks_order.clone();
    model.set_file_sample_ix_offset(a, 7);
    model
        .files
        .get_mut(a)
        .unwrap()
        .set_channel_interpretation(ChannelInterpretation {
            preset: ChannelOrderPreset::AuroIdOrder,
            active_layout: Some(crate::audio::Layout::LAYOUT_1_0),
        });
    let file_a = model.files[a].clone();
    Action::DiffFiles {
        file_a: a,
        file_b: b,
    }
    .process(&mut model)
    .unwrap();
    let pending = model.pending_diff_pairing.as_ref().unwrap();
    assert_eq!(pending.ch_ixs_a, [0, 2]);
    assert_eq!(pending.ch_ixs_b, [1, 4]);
    assert_eq!(pending.metadata_a.total_nr_channels, 3);
    assert_eq!(pending.metadata_b.total_nr_channels, 6);
    assert_eq!(pending.metadata_b.channel_mask, Some(0x3f));
    assert_eq!(pending.file_a.sample_ix_offset, 7);
    assert_eq!(
        pending.file_a.channel_interpretation,
        file_a.channel_interpretation
    );
    Action::CancelDiffPairing.process(&mut model).unwrap();
    assert_eq!(model.files[a], file_a);
    assert_eq!(model.tracks.tracks_order, order);
    assert!(model.pending_diff_pairing.is_none());
    model.open_existing_file_pairing(a, b).unwrap();
    let mut pending = model.pending_diff_pairing.take().unwrap();
    pending.clear();
    assert!(model.prepare_existing_file_diff(pending).unwrap().is_none());
    assert_eq!(model.tracks.tracks_order, order);
}

#[test]
fn diff_groups_selected_tracks_closes_unpaired_and_preserves_buffers_settings_and_unrelated_order()
{
    let (mut model, a, b, x) = fixture();
    let a0 = source_track(&model, a, 0);
    let a2 = source_track(&model, a, 2);
    let b1 = source_track(&model, b, 1);
    let b4 = source_track(&model, b, 4);
    let x0 = source_track(&model, x, 0);
    model.tracks.tracks_order = vec![x0, b4, a0, b1, a2];
    let original_buffers: Vec<_> = model
        .files
        .iter()
        .flat_map(|(_, file)| file.channels.values().map(|ch| ch.buffer_id))
        .map(|id| (id, model.audio.buffers[id].clone()))
        .collect();
    let saved = model.tracks.get_track_mut(b1).unwrap();
    saved.visible = false;
    saved.height = 171.0;
    saved.single.sample_ix_offset = 6.0;
    saved.use_file_offset = false;
    model.tracks.track_selection.apply(
        b1,
        crate::model::track_selection::TrackSelectionMode::Replace,
        &model.tracks.tracks_order,
    );
    let mut input = prepare(&mut model, a, b);
    input.request.interpretations[1].preset = ChannelOrderPreset::Unassigned;
    let result = compute(input);
    for (pair, expected) in result.pairs.iter().zip([[1, -2], [-2, -5]]) {
        match &pair.diff_buffer {
            BufferE::I16(buffer) => assert_eq!(buffer.data, expected),
            _ => panic!("expected PCM16"),
        }
    }
    model.integrate_existing_file_diff(result).unwrap();
    let order = &model.tracks.tracks_order;
    assert_eq!(order.len(), 6);
    assert_eq!([order[0], order[1], order[2], order[4]], [x0, a2, b1, b4]);
    assert!(model.tracks.get_track(a0).is_none());
    for (id, source_b) in [(order[3], b1), (order[5], b4)] {
        let diff = model.tracks.get_track(id).unwrap().diff.as_ref().unwrap();
        assert_eq!(diff.buffer_id_a, model.files[a].channels[&2].buffer_id);
        assert_eq!(
            diff.buffer_id_b,
            model.tracks.get_track(source_b).unwrap().single.buffer_id
        );
    }
    assert_eq!(model.files.len(), 3);
    assert_eq!(model.files.order(), [a, x, b]);
    assert!(model.files[a].source.is_none()); // Browser uploads work without disk recipes.
    assert_eq!(
        model.files[b].channel_interpretation.preset,
        ChannelOrderPreset::Unassigned
    );
    assert!(
        model.files[b]
            .channels
            .values()
            .all(|ch| ch.channel_id.is_none())
    );
    assert_eq!(model.files[a].channels[&2].channel_id, Some(Id::Center));
    for (id, buffer) in original_buffers {
        assert!(Arc::ptr_eq(&buffer, &model.audio.buffers[id]));
    }
    let saved = model.tracks.get_track(b1).unwrap();
    assert!(!saved.visible);
    assert_eq!(saved.height, 171.0);
    assert_eq!(saved.single.sample_ix_offset, 6.0);
    assert!(!saved.use_file_offset);
    assert!(model.tracks.track_selection.is_selected(b1));
}

#[test]
fn confirmation_computes_one_batch_and_restores_only_paired_closed_tracks() {
    let (mut model, a, b, _) = fixture();
    let a0 = source_track(&model, a, 0);
    let a2 = source_track(&model, a, 2);
    model.tracks.remove_track(a0);
    model.tracks.remove_track(a2);
    model.open_existing_file_pairing(a, b).unwrap();
    let pending = model.pending_diff_pairing.as_mut().unwrap();
    pending.clear();
    pending.checked[1][1] = true;
    pending.file_a.sample_ix_offset = 2;
    pending.offset_detection_a = Some(jobs::OffsetDetectionMode::FirstNonZero);
    Action::ConfirmDiffPairing.process(&mut model).unwrap();
    assert!(model.pending_diff_pairing.is_none());
    model
        .actions_rx
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap()
        .process(&mut model)
        .unwrap();
    assert!(
        model
            .find_track_id_for_buffer(model.files[a].channels[&0].buffer_id)
            .is_none()
    );
    let restored = source_track(&model, a, 2);
    assert_ne!(restored, a2);
    assert!(model.tracks.get_track(restored).unwrap().use_file_offset);
    assert_eq!(model.files[a].sample_ix_offset, 0); // Nonzero at first sample overrides manual 2.
    assert_eq!(model.files.len(), 3);
    assert_eq!(
        model
            .tracks
            .tracks
            .values()
            .filter(|track| track.diff.is_some())
            .count(),
        1
    );
}

#[test]
fn changed_or_closed_sources_reject_the_whole_result_before_mutating_workspace() {
    for change in ["close", "offset", "roles", "buffer"] {
        let (mut model, a, b, _) = fixture();
        let result = compute(prepare(&mut model, a, b));
        match change {
            "close" => {
                Action::CloseFile { file_id: a }
                    .process(&mut model)
                    .unwrap();
            }
            "offset" => {
                model.set_file_sample_ix_offset(a, 19);
            }
            "roles" => {
                model
                    .files
                    .get_mut(a)
                    .unwrap()
                    .set_channel_interpretation(ChannelInterpretation {
                        preset: ChannelOrderPreset::Unassigned,
                        active_layout: None,
                    })
            }
            "buffer" => {
                let new = model
                    .audio
                    .buffers
                    .insert(model.audio.buffers[model.files[a].channels[&2].buffer_id].clone());
                model
                    .files
                    .get_mut(a)
                    .unwrap()
                    .channels
                    .get_mut(&2)
                    .unwrap()
                    .buffer_id = new;
            }
            _ => unreachable!(),
        }
        let order = model.tracks.tracks_order.clone();
        let count = model.audio.buffers.len();
        let files: Vec<_> = model
            .files
            .iter()
            .map(|(id, file)| (id, file.clone()))
            .collect();
        assert!(
            model.integrate_existing_file_diff(result).is_err(),
            "{change}"
        );
        assert_eq!(model.tracks.tracks_order, order);
        assert_eq!(model.audio.buffers.len(), count);
        assert_eq!(
            model
                .files
                .iter()
                .map(|(id, file)| (id, file.clone()))
                .collect::<Vec<_>>(),
            files
        );
    }
}

#[test]
fn invalid_inputs_or_changed_dialog_source_do_not_start_computation() {
    let (mut model, a, b, _) = fixture();
    assert!(model.open_existing_file_pairing(a, a).is_err());
    model.files.get_mut(b).unwrap().sample_rate = 44100;
    assert!(model.open_existing_file_pairing(a, b).is_err());
    model.files.get_mut(b).unwrap().sample_rate = 48000;
    model.open_existing_file_pairing(a, b).unwrap();
    let pending = model.pending_diff_pairing.take().unwrap();
    model.set_file_sample_ix_offset(b, 3);
    assert!(model.prepare_existing_file_diff(pending).is_err());
    let mut input = prepare(&mut model, a, b);
    let buffer = Arc::make_mut(input.buffers[1].get_mut(&1).unwrap());
    match buffer {
        BufferE::I16(buffer) => buffer.sample_rate = 44100,
        _ => unreachable!(),
    }
    assert!(jobs::diff::compute_file_diff(1, input, &std::sync::mpsc::channel().0).is_err());
    assert!(
        model
            .tracks
            .tracks
            .values()
            .all(|track| track.diff.is_none())
    );
}

#[test]
fn a_dropped_file_stays_open_on_cancel_but_is_not_added_after_its_target_closes() {
    let loaded = || {
        let config = wav::ReadConfigBytes::new(
            Some("dropped.wav".into()),
            wav::channel_mask::tests::wav_bytes(None, 3, &[]),
        );
        wav::read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap()
    };
    let (mut model, target, _, _) = fixture();
    let original = model.tracks.tracks_order.clone();
    Action::IntegrateDroppedFileForDiff {
        generation: model.generation(),
        target,
        loaded: loaded(),
    }
    .process(&mut model)
    .unwrap();
    assert_eq!(model.files.len(), 4);
    assert_eq!(&model.tracks.tracks_order[..original.len()], &original);
    let with_dropped = model.tracks.tracks_order.clone();
    Action::CancelDiffPairing.process(&mut model).unwrap();
    assert_eq!(model.tracks.tracks_order, with_dropped);
    assert_eq!(model.files.len(), 4);
    Action::CloseFile { file_id: target }
        .process(&mut model)
        .unwrap();
    let count = model.files.len();
    let buffers = model.audio.buffers.len();
    Action::IntegrateDroppedFileForDiff {
        generation: model.generation(),
        target,
        loaded: loaded(),
    }
    .process(&mut model)
    .unwrap();
    assert_eq!(model.files.len(), count);
    assert_eq!(model.audio.buffers.len(), buffers);
}
