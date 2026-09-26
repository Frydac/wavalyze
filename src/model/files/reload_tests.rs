//! Regression scenarios for reload coordination across files, audio, tracks, and worker results.
//!
//! These tests keep real WAV decoding and diff computation while controlling notifications,
//! so atomic failure and stale-result checks do not depend on filesystem event timing.

use super::*;
use crate::audio::{buffer::BufferE, thumbnail::ThumbnailE};
use crate::wav;
use crate::{
    audio,
    model::selection_info::{SelectionInfo, SelectionInfoE},
};
use std::sync::atomic::{AtomicUsize, Ordering};

fn fixture() -> (Model, PathBuf) {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let directory = PathBuf::from(format!(
        "target/test_output/reload/{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let mut model = Model::new();
    model.files.changes.disable_watcher();
    (model, directory)
}
fn write(path: &std::path::Path, channels: u16, rate: u32, samples: &[i16]) {
    let mut writer = hound::WavWriter::create(
        path,
        hound::WavSpec {
            channels,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for sample in samples {
        writer.write_sample(*sample).unwrap();
    }
    writer.finalize().unwrap();
}
fn open(model: &mut Model, path: &std::path::Path) -> FileId {
    model.load_wav(&wav::ReadConfig::new(path)).unwrap();
    *model.files.order().last().unwrap()
}
fn changed(model: &mut Model, id: FileId) {
    let path = crate::model::files::disk::absolute_path(
        &model.files[id].source.as_ref().unwrap().filepath,
    );
    let change = model.files.changes.paths.get_mut(&path).unwrap();
    change.changed = true;
    change.revision += 1;
    change.last_event_time = None;
}
fn request(model: &mut Model, ids: &[FileId]) -> (ReloadRequest, ReloadJobInput) {
    let sources: Vec<_> = ids
        .iter()
        .map(|id| {
            let file = model.files[*id].clone();
            let path =
                crate::model::files::disk::absolute_path(&file.source.as_ref().unwrap().filepath);
            Source {
                id: *id,
                file,
                revision: model.files.changes.paths[&path].revision,
                path,
            }
        })
        .collect();
    let dirty = sources
        .iter()
        .flat_map(|s| s.file.channels.values().map(|c| c.buffer_id))
        .collect();
    let topology = model.tracks.diff_topology();
    let affected = affected_diffs(&topology, &dirty);
    let job_id = model
        .job_mgr
        .start_job(jobs::JobKind::Reload, "test reload");
    model.files.reload.active_job = Some(job_id);
    let request = ReloadRequest {
        job_id,
        generation: model.generation(),
        sources,
        topology,
        affected,
    };
    let input = request.job_input(&model.audio);
    (request, input)
}

fn finish(model: &mut Model, request: (ReloadRequest, ReloadJobInput)) {
    let result = prepare(&request, &model.job_mgr.sender()).map_err(|e| format!("{e:#}"));
    model.finish_reload(ReloadResult {
        request: request.0,
        result,
    });
    model.drain_job_events();
}
fn buffer(model: &Model, id: FileId) -> BufferId {
    model.files[id].channels[&0].buffer_id
}
fn add_diff(model: &mut Model, a: BufferId, b: BufferId, offset_a: i64, offset_b: i64) -> TrackId {
    let computed = jobs::diff::compute_diff_buffer(
        model.audio.get_buffer(a).unwrap(),
        model.audio.get_buffer(b).unwrap(),
        offset_a,
        offset_b,
    )
    .unwrap();
    let thumbnail = ThumbnailE::from_buffer_e(&computed.buffer, None);
    model
        .add_diff_buffer(jobs::ComputedDiff {
            buffer_id_a: a,
            buffer_id_b: b,
            sample_ix_offset_a: offset_a,
            sample_ix_offset_b: offset_b,
            sample_ix_offset_diff: computed.sample_ix_offset_diff,
            diff_buffer: computed.buffer,
            diff_thumbnail: thumbnail,
            insert_after: None,
        })
        .unwrap();
    *model.tracks.tracks_order.last().unwrap()
}
fn data(model: &Model, id: BufferId) -> Vec<i16> {
    match model.audio.get_buffer(id).unwrap() {
        BufferE::I16(b) => b.data.clone(),
        _ => panic!("expected i16"),
    }
}

#[test]
fn batch_reloads_shared_and_chained_diffs_preserving_workspace() {
    let (mut model, dir) = fixture();
    let a_path = dir.join("a.wav");
    let b_path = dir.join("b.wav");
    write(&a_path, 1, 48000, &[1, 2]);
    write(&b_path, 1, 48000, &[1, 1]);
    let a = open(&mut model, &a_path);
    let b = open(&mut model, &b_path);
    let a_buffer = buffer(&model, a);
    let b_buffer = buffer(&model, b);
    let source_track = model.find_track_id_for_buffer(a_buffer).unwrap();
    let diff = add_diff(&mut model, a_buffer, b_buffer, 2, 1);
    let diff_buffer = model.tracks.get_track(diff).unwrap().single.buffer_id;
    let chained = add_diff(&mut model, diff_buffer, b_buffer, 0, 0);
    // Deliberately reverse display order to exercise dependency ordering.
    model.tracks.tracks_order.reverse();
    model.tracks.get_track_mut(source_track).unwrap().height = 123.0;
    model.tracks.get_track_mut(source_track).unwrap().visible = false;
    model.set_file_sample_ix_offset(a, 7);
    model.tracks.selection_info = SelectionInfoE::IsSelected(SelectionInfo {
        ix_rng: (1..4).into(),
        ..Default::default()
    });
    let selection = model.tracks.selection_info;
    let order = model.tracks.tracks_order.clone();
    write(&a_path, 1, 48000, &[9, 8, 7]);
    write(&b_path, 1, 48000, &[2, 3, 4]);
    changed(&mut model, a);
    changed(&mut model, b);
    let req = request(&mut model, &[a, b]);
    assert_eq!(
        req.0.affected.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [
            diff_buffer,
            model.tracks.get_track(chained).unwrap().single.buffer_id
        ]
    );
    finish(&mut model, req);
    assert!(model.files.changed_ids().is_empty());
    assert_eq!(model.tracks.tracks_order, order);
    assert_eq!(model.tracks.selection_info, selection);
    let track = model.tracks.get_track(source_track).unwrap();
    assert_eq!(track.height, 123.0);
    assert!(!track.visible);
    assert_eq!(track.single.sample_ix_offset, 7.0);
    assert_eq!(data(&model, buffer(&model, a)), [9, 8, 7]);
    let metadata = model.tracks.get_track(diff).unwrap().diff.as_ref().unwrap();
    assert_eq!(
        (metadata.sample_ix_offset_a, metadata.sample_ix_offset_b),
        (2, 1)
    );
    assert_eq!(metadata.buffer_id_a, buffer(&model, a));
    assert!(!model.audio.buffers.contains_key(a_buffer));
    let chain = model
        .tracks
        .get_track(chained)
        .unwrap()
        .diff
        .as_ref()
        .unwrap();
    assert_eq!(chain.buffer_id_a, metadata.buffer_id_diff);
    let expected = jobs::diff::compute_diff_buffer(
        model.audio.get_buffer(chain.buffer_id_a).unwrap(),
        model.audio.get_buffer(chain.buffer_id_b).unwrap(),
        0,
        0,
    )
    .unwrap();
    if let BufferE::I16(expected) = expected.buffer {
        assert_eq!(data(&model, chain.buffer_id_diff), expected.data);
    }
}

#[test]
fn individual_update_uses_other_inputs_loaded_data_and_keeps_closed_tracks_closed() {
    let (mut model, dir) = fixture();
    let ap = dir.join("a.wav");
    let bp = dir.join("b.wav");
    write(&ap, 1, 48000, &[1, 2]);
    write(&bp, 1, 48000, &[1, 1]);
    let a = open(&mut model, &ap);
    let b = open(&mut model, &bp);
    let ab = buffer(&model, a);
    let bb = buffer(&model, b);
    let diff = add_diff(&mut model, ab, bb, 0, 0);
    model.remove_channel_track(ab);
    write(&ap, 1, 48000, &[9, 8]);
    write(&bp, 1, 48000, &[5, 5]);
    changed(&mut model, a);
    changed(&mut model, b);
    let req = request(&mut model, &[a]);
    finish(&mut model, req);
    assert_eq!(model.files.changed_ids(), [b]);
    assert!(model.find_track_id_for_buffer(buffer(&model, a)).is_none());
    assert_eq!(
        data(
            &model,
            model.tracks.get_track(diff).unwrap().single.buffer_id
        ),
        [8, 7]
    );
    assert_eq!(buffer(&model, b), bb);
}

#[test]
fn failure_is_atomic_for_bad_files_missing_channels_and_incompatible_rates() {
    for failure in 0..4 {
        let (mut model, dir) = fixture();
        let ap = dir.join("a.wav");
        let bp = dir.join("b.wav");
        write(&ap, 1, 48000, &[1]);
        write(&bp, 2, 48000, &[2, 3]);
        let a = open(&mut model, &ap);
        let b = open(&mut model, &bp);
        let ab = buffer(&model, a);
        let bb = buffer(&model, b);
        add_diff(&mut model, ab, bb, 0, 0);
        write(&ap, 1, 48000, &[9]);
        match failure {
            0 => std::fs::write(&bp, b"invalid wav").unwrap(),
            1 => std::fs::remove_file(&bp).unwrap(),
            2 => write(&bp, 1, 48000, &[4]),
            _ => write(&bp, 2, 44100, &[4, 5]),
        }
        changed(&mut model, a);
        changed(&mut model, b);
        let req = request(&mut model, &[a, b]);
        finish(&mut model, req);
        assert_eq!(buffer(&model, a), ab);
        assert_eq!(buffer(&model, b), bb);
        assert_eq!(model.files.changed_ids(), [a, b]);
        assert_eq!(
            model.job_mgr.recent_finished().next().unwrap().status,
            jobs::JobStatus::Failed
        );
    }
}

#[test]
fn stale_prepared_results_never_commit() {
    for mutation in 0..4 {
        let (mut model, dir) = fixture();
        let path = dir.join("a.wav");
        write(&path, 1, 48000, &[1]);
        let id = open(&mut model, &path);
        let old = buffer(&model, id);
        write(&path, 1, 48000, &[9]);
        changed(&mut model, id);
        let req = request(&mut model, &[id]);
        let prepared = prepare(&req, &model.job_mgr.sender()).unwrap();
        match mutation {
            0 => changed(&mut model, id),
            1 => {
                model.remove_file(id);
            }
            2 => model.close_all(),
            _ => {
                add_diff(&mut model, old, old, 0, 0);
            }
        }
        model.finish_reload(ReloadResult {
            request: req.0,
            result: Ok(prepared),
        });
        assert!(
            model
                .files
                .get(id)
                .is_none_or(|f| f.channels[&0].buffer_id == old)
        );
    }
}

#[test]
fn preserves_selection_recipe_and_custom_offsets_and_rejects_old_statistics() {
    let (mut model, dir) = fixture();
    let path = dir.join("selected.wav");
    write(&path, 2, 48000, &[1, 2, 3, 4, 5, 6]);
    let recipe = wav::ReadConfig::new(&path)
        .with_ch_ixs([1])
        .with_sample_range(audio::sample::OptIxRange {
            start: Some(1),
            end: Some(3),
        });
    model.load_wav(&recipe).unwrap();
    let id = model.files.order()[0];
    let old = model.files[id].channels[&1].buffer_id;
    let track = model.find_track_id_for_buffer(old).unwrap();
    model.set_track_use_file_offset(track, false);
    model.set_track_sample_ix_offset(track, 3.5);
    write(&path, 3, 48000, &[10, 20, 30, 40, 50, 60, 70, 80, 90]);
    changed(&mut model, id);
    let req = request(&mut model, &[id]);
    finish(&mut model, req);
    let file = &model.files[id];
    assert_eq!(file.source.as_deref().unwrap(), &recipe);
    assert_eq!(file.channels.len(), 1);
    assert_eq!(file.total_nr_channels, 3);
    assert_eq!(data(&model, file.channels[&1].buffer_id), [50, 80]);
    assert_eq!(
        model
            .tracks
            .get_track(track)
            .unwrap()
            .single
            .sample_ix_offset,
        3.5
    );
    Action::SetBufferStats {
        buffer_id: old,
        stats: crate::model::stats::BufferStats {
            range: (0..1).into(),
            rms_db: Some(0.0),
            peak: None,
        },
    }
    .process(&mut model)
    .unwrap();
    assert!(model.audio.stats.is_empty());
}

#[test]
fn duplicate_path_instances_are_acknowledged_individually() {
    let (mut model, dir) = fixture();
    let path = dir.join("a.wav");
    write(&path, 1, 48000, &[1]);
    let a = open(&mut model, &path);
    let b = open(&mut model, &path);
    write(&path, 1, 48000, &[9]);
    changed(&mut model, a);
    let req = request(&mut model, &[a]);
    finish(&mut model, req);
    assert_eq!(model.files.changed_ids(), [b]);
    let req = request(&mut model, &[b]);
    finish(&mut model, req);
    assert!(model.files.changed_ids().is_empty());
}
#[test]
fn removed_intermediate_diff_is_recomputed_without_restoring_its_track() {
    let (mut model, dir) = fixture();
    let path = dir.join("a.wav");
    write(&path, 1, 48000, &[1, 2]);
    let file = open(&mut model, &path);
    let input = buffer(&model, file);
    let intermediate = add_diff(&mut model, input, input, 0, 0);
    let intermediate_buffer = model
        .tracks
        .get_track(intermediate)
        .unwrap()
        .single
        .buffer_id;
    let downstream = add_diff(&mut model, intermediate_buffer, input, 0, 0);
    model.tracks.remove_track(intermediate);
    write(&path, 1, 48000, &[9, 8]);
    changed(&mut model, file);
    let req = request(&mut model, &[file]);
    assert_eq!(req.0.affected.len(), 2);
    finish(&mut model, req);
    assert!(model.tracks.get_track(intermediate).is_none());
    assert_eq!(
        data(
            &model,
            model.tracks.get_track(downstream).unwrap().single.buffer_id
        ),
        [-9, -8]
    );
    assert!(!model.audio.buffers.contains_key(intermediate_buffer));
    // The retained recipe must also refer to the new buffers on a subsequent reload.
    write(&path, 1, 48000, &[4]);
    changed(&mut model, file);
    let req = request(&mut model, &[file]);
    finish(&mut model, req);
    assert_eq!(
        data(
            &model,
            model.tracks.get_track(downstream).unwrap().single.buffer_id
        ),
        [-4]
    );
}

#[test]
fn action_job_pipeline_updates_audio_and_closing_cleans_monitoring() {
    let (mut model, dir) = fixture();
    let path = dir.join("worker.wav");
    write(&path, 1, 48000, &[1]);
    let file = open(&mut model, &path);
    let old = buffer(&model, file);
    let old_audio = model.audio.buffer_arc(old).unwrap();
    write(&path, 1, 48000, &[7, 8]);
    changed(&mut model, file);
    Action::ReloadFiles(vec![file]).process(&mut model).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while model.files.reload.active_job.is_some() && std::time::Instant::now() < deadline {
        model.drain_action_messages();
        model.process_actions().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(model.files.reload.active_job.is_none());
    assert_eq!(data(&model, buffer(&model, file)), [7, 8]);
    Action::OffsetDetectedChecked {
        result: jobs::OffsetDetectionResult {
            target: jobs::OffsetDetectionTarget::File { file_id: file },
            sample_ix_offset: Some(99),
        },
        buffers: vec![old_audio],
    }
    .process(&mut model)
    .unwrap();
    assert_eq!(model.files[file].sample_ix_offset, 0);
    model.remove_file(file);
    model.drain_job_events();
    model.files.poll_changes(&egui::Context::default(), false);
    assert!(model.files.changes.paths.is_empty());
}

#[test]
fn byte_loaded_filename_does_not_become_a_reload_source() {
    let (mut model, dir) = fixture();
    let path = dir.join("bytes.wav");
    write(&path, 1, 48000, &[1]);
    let config = wav::ReadConfigBytes::new(
        Some(path.display().to_string()),
        std::fs::read(&path).unwrap(),
    );
    let loaded = wav::read::read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap();
    let file = model.add_loaded_file(loaded).unwrap();
    assert!(model.files[file].source.is_none());
    assert!(model.files.changes.paths.is_empty());
}

fn prepare(
    request: &(ReloadRequest, ReloadJobInput),
    events: &std::sync::mpsc::Sender<JobEvent>,
) -> Result<PreparedReload> {
    jobs::reload::prepare(request.0.job_id, &request.1, events)
}
