use wavalyze::audio::sample_range::SampleIxRange;

#[test]
fn test_zoom() {
    let mut range = SampleIxRange::new(100.0, 200.0);
    range.zoom(10.0, 150.0);

    assert_eq!(range.end(), 205.0);
}

#[test]
fn test_zoom_center_before_range() {
    let mut range = SampleIxRange::new(100.0, 200.0);
    range.zoom(10.0, 50.0);
    assert_eq!(range.start(), 100.0);
    assert_eq!(range.end(), 210.0);
}

#[test]
fn test_zoom_center_after_range() {
    let mut range = SampleIxRange::new(100.0, 200.0);
    range.zoom(10.0, 250.0);
    assert_eq!(range.start(), 90.0);
    assert_eq!(range.end(), 200.0);
}

#[test]
fn test_zoom_negative_and_positive_range() {
    let mut range = SampleIxRange::new(-50.0, 50.0);
    range.zoom(10.0, 0.0);
    assert_eq!(range.start(), -55.0);
    assert_eq!(range.end(), 55.0);
}

#[test]
fn test_zoom_negative_range_quarter_center() {
    let mut range = SampleIxRange::new(-200.0, -100.0);
    range.zoom(10.0, -175.0);
    assert_eq!(range.start(), -202.5);
    assert_eq!(range.end(), -92.5);
}
