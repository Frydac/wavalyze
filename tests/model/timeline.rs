#[test]
fn test_timeline() {
    let mut timeline = wavalyze::model::SampleIxZoom::default();
    assert_eq!(timeline.samples_per_pixel(), 0.0);
    assert_eq!(timeline.ix_start, 0.0);

    let ix_range = timeline.get_ix_range(100.0);
    assert_eq!(
        ix_range,
        wavalyze::audio::sample::FracIxRange {
            start: 0.0,
            end: 0.0
        }
    );

    timeline.set_samples_per_pixel(10.0);
    let ix_range = timeline.get_ix_range(100.0);
    assert_eq!(
        ix_range,
        wavalyze::audio::sample::FracIxRange {
            start: 0.0,
            end: 1000.0
        }
    );

    timeline.ix_start = -50.0;
    let ix_range = timeline.get_ix_range(100.0);
    assert_eq!(
        ix_range,
        wavalyze::audio::sample::FracIxRange {
            start: -50.0,
            end: 950.0
        }
    );
}
