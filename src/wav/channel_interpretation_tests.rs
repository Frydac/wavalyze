use super::*;

#[test]
fn automatic_falls_back_only_for_absent_or_zero_masks() {
    for mask in [None, Some(0)] {
        let resolved = ChannelInterpretation::default().resolve(mask, 3);
        assert_eq!(
            resolved.source,
            InterpretationSource::AuroIdOrder { assumed: true }
        );
        assert_eq!(
            resolved.channels.iter().map(|ch| ch.id).collect::<Vec<_>>(),
            [Some(Id::Left), Some(Id::Right), Some(Id::Center)]
        );
        assert_eq!(resolved.active_layout, None);
        assert!(resolved.channels.iter().all(|ch| !ch.padding));
    }
    // Unknown bit, count mismatch, and duplicate speaker identities must not cause fallback.
    for (mask, count) in [(0x1000000, 1), (0x3, 3), (0x088000, 2)] {
        let resolved = ChannelInterpretation::default().resolve(Some(mask), count);
        assert_eq!(resolved.source, InterpretationSource::Unassigned);
        assert!(
            resolved
                .channels
                .iter()
                .all(|ch| ch.id.is_none() && !ch.padding)
        );
        assert!(resolved.candidate_layouts.is_empty());
        assert_eq!(resolved.active_layout, None);
    }
}

#[test]
fn valid_wavex_takes_precedence_over_auro_ids_and_explicit_active_layout() {
    let interpretation = ChannelInterpretation {
        active_layout: Some(Layout::LAYOUT_1_0),
        ..Default::default()
    };
    let resolved = interpretation.resolve(Some(0x1c0), 3);
    assert_eq!(resolved.source, InterpretationSource::Wavex);
    assert_eq!(
        resolved.channels.iter().map(|ch| ch.id).collect::<Vec<_>>(),
        [
            Some(Id::LeftCenter),
            Some(Id::RightCenter),
            Some(Id::CenterSurround)
        ]
    );
    assert_eq!(
        resolved.active_layout.unwrap().bits(),
        (1 << 18) | (1 << 19) | (1 << 6)
    );
    assert!(resolved.candidate_layouts.is_empty());
    assert!(resolved.channels.iter().all(|ch| !ch.padding));
}

#[test]
fn explicit_auro_order_overrides_valid_and_invalid_masks() {
    let interpretation = ChannelInterpretation {
        preset: ChannelOrderPreset::AuroIdOrder,
        active_layout: None,
    };
    for mask in [None, Some(0), Some(0x1c0), Some(0x1000000)] {
        let resolved = interpretation.resolve(mask, 3);
        assert_eq!(
            resolved.source,
            InterpretationSource::AuroIdOrder { assumed: false }
        );
        assert_eq!(resolved.channels[2].id, Some(Id::Center));
        assert_eq!(resolved.active_layout, None);
    }
}

#[test]
fn unassigned_suppresses_header_roles_and_selected_layout() {
    let resolved = ChannelInterpretation {
        preset: ChannelOrderPreset::Unassigned,
        active_layout: Some(Layout::LAYOUT_1_0),
    }
    .resolve(Some(0x7), 3);
    assert_eq!(resolved.source, InterpretationSource::Unassigned);
    assert!(
        resolved
            .channels
            .iter()
            .all(|ch| ch.id.is_none() && !ch.padding)
    );
    assert_eq!(resolved.active_layout, None);
    assert!(resolved.candidate_layouts.is_empty());
}

#[test]
fn auro_mapping_uses_numeric_ids_including_unknown_indices() {
    let resolved = ChannelInterpretation::default().resolve(None, 33);
    // Auro order differs from WAVEX order for surrounds, heights and bottoms.
    for (index, id) in [
        (4, Id::LeftSurround),
        (7, Id::LeftBack),
        (10, Id::HeightRight),
        (18, Id::LeftCenter),
        (22, Id::BottomRight),
        (23, Id::BottomCenter),
        (30, Id::Mono),
    ] {
        assert_eq!(resolved.channels[index].id, Some(id));
    }
    assert_eq!(resolved.channels[31].id, None);
    assert_eq!(resolved.channels[32].id, None);
    assert_eq!(resolved.channels.get(33), None);
    assert!(resolved.candidate_layouts.is_empty());
}

#[test]
fn candidates_use_padded_count_without_choosing_an_active_layout() {
    for (count, expected) in [
        (2, vec![0x3]),
        (3, vec![0x4, 0x7]),
        (4, vec![0x8, 0xb, 0xc, 0xf]),
        (6, vec![0x33, 0x37, 0x3b, 0x3f]),
        (24, vec![0xeffff7, 0xffffff]),
    ] {
        let resolved = ChannelInterpretation::default().resolve(None, count);
        assert_eq!(
            resolved
                .candidate_layouts
                .iter()
                .map(Layout::bits)
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(resolved.active_layout, None);
    }
    let resolved = ChannelInterpretation::default().resolve(None, 3);
    assert_eq!(
        resolved.candidate_layouts[0].names(),
        &["1.0", "TestMix1.0"]
    );
    // A one-index file cannot store the catalog's Center-at-index-2 mono layout.
    assert!(
        ChannelInterpretation::default()
            .resolve(None, 1)
            .candidate_layouts
            .is_empty()
    );
    let empty = ChannelInterpretation::default().resolve(None, 0);
    assert!(empty.channels.is_empty());
    assert!(empty.candidate_layouts.is_empty());
}

#[test]
fn selected_mono_marks_expected_padding_and_keeps_potential_roles() {
    let interpretation = ChannelInterpretation {
        active_layout: Some(Layout::LAYOUT_1_0),
        ..Default::default()
    };
    let resolved = interpretation.resolve(None, 3);
    assert_eq!(resolved.active_layout, Some(Layout::LAYOUT_1_0));
    assert_eq!(
        resolved.channels,
        vec![
            ChannelRole {
                id: Some(Id::Left),
                padding: true
            },
            ChannelRole {
                id: Some(Id::Right),
                padding: true
            },
            ChannelRole {
                id: Some(Id::Center),
                padding: false
            },
        ]
    );
    // No matching count, arbitrary trailing padding, empty and uncatalogued layouts are rejected.
    for layout in [
        Layout::LAYOUT_5_1,
        Layout::LAYOUT_2_0,
        Layout::NONE,
        Layout::CENTER | Layout::LEFT,
    ] {
        let resolved = ChannelInterpretation {
            active_layout: Some(layout),
            ..Default::default()
        }
        .resolve(None, 3);
        assert_eq!(resolved.active_layout, None);
        assert!(resolved.channels.iter().all(|ch| !ch.padding));
    }
    assert_eq!(interpretation.resolve(None, 4).active_layout, None);
}

#[test]
fn partial_channel_selection_keeps_original_indices_for_both_orders() {
    // Consumers load only these indices; resolution always uses the complete header count.
    let selected = [2, 4];
    let auro = ChannelInterpretation::default().resolve(None, 6);
    assert_eq!(
        selected.map(|ix| auro.channels[ix].id),
        [Some(Id::Center), Some(Id::LeftSurround)]
    );
    let wavex = ChannelInterpretation::default().resolve(Some(0x3f), 6);
    assert_eq!(
        selected.map(|ix| wavex.channels[ix].id),
        [Some(Id::Center), Some(Id::LeftBack)]
    );
}
