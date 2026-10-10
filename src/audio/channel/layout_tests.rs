use super::Layout;

#[test]
fn exact_masks_expose_catalog_names_and_synonyms() {
    assert_eq!(Layout::LAYOUT_5_1.names(), &["5.1"]);
    assert_eq!(Layout::LAYOUT_AURO9_1.names(), &["5.1_4H", "Auro9.1"]);
    assert_eq!(Layout::LAYOUT_NHK_22_0.names(), &["NHK_22.0"]);
    assert!(Layout::NONE.names().is_empty());
    assert!((Layout::LEFT | Layout::TOP).names().is_empty());
    // Alternative names such as "stereo" do not become catalog synonyms.
    assert_eq!(Layout::LAYOUT_2_0.names(), &["2.0"]);
}

#[test]
fn individual_channels_and_stereo_keep_their_masks() {
    assert_eq!(Layout::LEFT.bits(), 1);
    assert_eq!((Layout::LEFT | Layout::RIGHT).bits(), 3);
    assert_eq!(Layout::LAYOUT_2_0, Layout::LOWER_FRONTS);
    assert!(!Layout::LAYOUT_2_0.contains(Layout::CENTER));
}

#[test]
fn side_surround_5_1_matches_internal_ids() {
    assert_eq!(Layout::LAYOUT_5_1.bits(), 0x3f);
    assert!(Layout::LAYOUT_5_1.contains(Layout::LOWER_SURROUNDS));
    assert!(!Layout::LAYOUT_5_1.intersects(Layout::LOWER_BACKS));
}

#[test]
fn nhk_22_0_excludes_both_lfe_channels() {
    assert_eq!(Layout::LAYOUT_NHK_22_2.bits().count_ones(), 24);
    assert_eq!(Layout::LAYOUT_NHK_22_0.bits().count_ones(), 22);
    assert_eq!(
        Layout::LAYOUT_NHK_22_0,
        Layout::LAYOUT_NHK_22_2 & !(Layout::LFE | Layout::LFE2)
    );
    assert!(!Layout::LAYOUT_NHK_22_0.intersects(Layout::LFE | Layout::LFE2));
}

#[test]
fn imported_synonyms_and_missing_layouts_are_available() {
    assert_eq!(Layout::LAYOUT_AURO9_1, Layout::LAYOUT_5_1_4H);
    assert_eq!(Layout::LAYOUT_AURO8_1, Layout::LAYOUT_4_1_4H);
    assert_eq!(
        Layout::LAYOUT_7_1_5H_1T_NO_C,
        Layout::LAYOUT_AURO13_1 & !Layout::CENTER
    );
    assert_eq!(
        Layout::LAYOUT_9_1_5H_2T,
        Layout::LAYOUT_7_1_5H_2T | Layout::LOWER_WIDES
    );
}

#[test]
fn supplemental_masks_and_legacy_aliases_remain_available() {
    assert_eq!(
        Layout::HEIGHT_LAYER,
        Layout::HEIGHT_FRONTS
            | Layout::HEIGHT_SURROUNDS
            | Layout::HEIGHT_BACKS
            | Layout::HEIGHT_CENTER
            | Layout::HEIGHT_CENTER_SURROUND
    );
    assert_eq!(Layout::LAYOUT_AURO13_1_2T, Layout::LAYOUT_7_1_5H_2T);
    assert_eq!(
        Layout::LAYOUT_AURO12_1_2T,
        Layout::LAYOUT_AURO12_1 | Layout::STEREO_TOP
    );
    assert_eq!(
        Layout::LAYOUT_AURO10_1_2T,
        Layout::LAYOUT_AURO10_1 | Layout::STEREO_TOP
    );
}
