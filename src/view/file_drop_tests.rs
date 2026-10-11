use super::*;

fn setup() -> (View, wav::file::FileId) {
    let mut model = model::Model::new();
    model
        .tracks
        .ruler
        .set_screen_rect(crate::rect::Rect::new(0.0, 0.0, 800.0, 30.0));
    #[cfg(not(target_arch = "wasm32"))]
    model.files.changes.disable_watcher();
    let config = wav::ReadConfigBytes::new(
        Some("target.wav".into()),
        wav::channel_mask::tests::wav_bytes(None, 3, &[]),
    )
    .with_ch_ixs([0, 2]);
    let target = model
        .add_loaded_file(wav::read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap())
        .unwrap();
    (View::new(model, TracingCollector::default()), target)
}

fn integrate_worker_result(model: &mut model::Model) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !model.drain_action_messages() {
        assert!(std::time::Instant::now() < deadline, "worker timed out");
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    model.process_actions().unwrap();
}

#[test]
fn new_wav_dropped_on_a_file_loads_once_and_opens_pairing_for_that_target() {
    let mut drops = vec![egui::DroppedFile {
        name: "new.wav".into(),
        bytes: Some(wav::channel_mask::tests::wav_bytes(None, 3, &[]).into()),
        ..Default::default()
    }];
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = std::path::Path::new("target/test_output/new_file_diff/NEW.WAV");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, wav::channel_mask::tests::wav_bytes(None, 3, &[])).unwrap();
        drops.push(egui::DroppedFile {
            path: Some(path.into()),
            ..Default::default()
        });
    }
    for drop in drops {
        let (mut view, target) = setup();
        let saved_track = view
            .model
            .find_track_id_for_buffer(view.model.files[target].channels[&2].buffer_id)
            .unwrap();
        let ctx = egui::Context::default();
        let frame = |view: &mut View, input: egui::RawInput| {
            ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| file::ui(ui, &mut view.model));
                view.handle_drag_and_drop_into_app(ctx);
            })
        };
        let _ = frame(&mut view, Default::default());
        let output = frame(&mut view, Default::default());
        let pos = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.text() == "target.wav" => {
                    Some(text.pos + text.galley.size() / 2.0)
                }
                _ => None,
            })
            .unwrap();
        let _ = frame(
            &mut view,
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(pos)],
                ..Default::default()
            },
        );
        let _ = frame(
            &mut view,
            egui::RawInput {
                dropped_files: vec![drop.clone()],
                ..Default::default()
            },
        );
        assert_eq!(
            view.model.actions.len(),
            1,
            "drop must not also use the global opener"
        );
        let action = view.model.actions.pop().unwrap();
        assert!(
            matches!(action, Action::OpenDroppedFileForDiff { target: id, .. } if id == target)
        );
        action.process(&mut view.model).unwrap();
        integrate_worker_result(&mut view.model);
        assert_eq!(view.model.files.len(), 2);
        let pending = view.model.pending_diff_pairing.as_ref().unwrap();
        assert_eq!(pending.ch_ixs_a, [0, 1, 2]);
        assert_eq!(pending.ch_ixs_b, [0, 2]);
        assert_eq!(pending.existing_files.as_ref().unwrap()[1].id, target);
        let source = pending.existing_files.as_ref().unwrap()[0].id;
        assert_eq!(
            view.model.files[source].source.is_some(),
            drop.path.is_some()
        );
        let pending = view.model.pending_diff_pairing.as_mut().unwrap();
        pending.clear();
        pending.checked[2][1] = true;
        Action::ConfirmDiffPairing.process(&mut view.model).unwrap();
        integrate_worker_result(&mut view.model);
        let order = &view.model.tracks.tracks_order;
        assert_eq!(order.len(), 3);
        assert_eq!(order[1], saved_track);
        assert!(
            view.model
                .tracks
                .get_track(order[2])
                .unwrap()
                .diff
                .is_some()
        );
        assert_eq!(view.model.files[target].channels.len(), 2);
        assert_eq!(view.model.files[source].channels.len(), 3);
    }
}

#[test]
fn dropping_bytes_away_from_file_headers_keeps_the_normal_open_action() {
    let (mut view, _) = setup();
    let ctx = egui::Context::default();
    let _ = ctx.run(
        egui::RawInput {
            events: vec![egui::Event::PointerMoved(egui::pos2(900.0, 700.0))],
            dropped_files: vec![egui::DroppedFile {
                name: "new.wav".into(),
                bytes: Some(wav::channel_mask::tests::wav_bytes(None, 3, &[]).into()),
                ..Default::default()
            }],
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| file::ui(ui, &mut view.model));
            view.handle_drag_and_drop_into_app(ctx);
        },
    );
    assert_eq!(view.model.actions.len(), 1);
    assert!(matches!(
        view.model.actions.pop(),
        Some(Action::OpenFileBytes(_))
    ));
}

#[test]
fn external_drag_exposes_padded_targets_without_pointer_hover_and_accepts_margin_drops() {
    let (mut view, target) = setup();
    let ctx = egui::Context::default();
    let frame = |view: &mut View, input| {
        ctx.run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| file::ui(ui, &mut view.model));
        })
    };
    let _ = frame(&mut view, egui::RawInput::default());
    let output = frame(
        &mut view,
        egui::RawInput {
            events: vec![egui::Event::PointerGone],
            hovered_files: vec![egui::HoveredFile {
                path: Some("external.wav".into()),
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    assert!(ctx.pointer_hover_pos().is_none());
    let label = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.text() == "target.wav" => {
                Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
            }
            _ => None,
        })
        .unwrap();
    let outline = output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Rect(rect)
                if rect.stroke.width == 1.0
                    && rect.stroke.color
                        == ctx
                            .style()
                            .visuals
                            .selection
                            .stroke
                            .color
                            .gamma_multiply(0.6) =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .expect("OS hover should show a drop target even without pointer coordinates");
    assert!(outline.left() < label.left());
    assert!(outline.right() > label.right() + 100.0);
    assert!(outline.top() < label.top());
    assert!(outline.bottom() > label.bottom());
    assert_eq!(file::take_external_drop_target(&ctx), None);
    let margin = egui::pos2(outline.right() - 2.0, outline.top() + 1.0);
    assert!(!label.contains(margin));
    let output = frame(
        &mut view,
        egui::RawInput {
            events: vec![egui::Event::PointerMoved(margin)],
            hovered_files: vec![egui::HoveredFile {
                path: Some("external.wav".into()),
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.stroke.width == 2.0 && rect.stroke.color == ctx.style().visuals.selection.stroke.color)));
    assert_eq!(file::take_external_drop_target(&ctx), Some(target));
    let _ = frame(
        &mut view,
        egui::RawInput {
            dropped_files: vec![egui::DroppedFile {
                name: "external.wav".into(),
                bytes: Some(wav::channel_mask::tests::wav_bytes(None, 3, &[]).into()),
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    view.handle_drag_and_drop_into_app(&ctx);
    assert!(
        matches!(view.model.actions.pop(), Some(Action::OpenDroppedFileForDiff { target: id, .. }) if id == target)
    );
}
