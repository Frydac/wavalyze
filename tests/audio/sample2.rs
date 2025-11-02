use wavalyze::audio::sample2::Sample;

#[test]
fn test_i16_min_max() {
    assert_eq!(Sample::min(5i16, 10), 5);
    assert_eq!(Sample::min(10i16, 5), 5);
    assert_eq!(Sample::min(-5i16, -10), -10);
    assert_eq!(Sample::min(-10i16, -5), -10);
    assert_eq!(Sample::min(0i16, 5), 0);

    assert_eq!(Sample::max(5i16, 10), 10);
    assert_eq!(Sample::max(10i16, 5), 10);
    assert_eq!(Sample::max(-5i16, -10), -5);
    assert_eq!(Sample::max(-10i16, -5), -5);
    assert_eq!(Sample::max(0i16, 5), 5);
}

#[test]
fn test_i32_min_max() {
    assert_eq!(Sample::min(5i32, 10), 5);
    assert_eq!(Sample::min(10i32, 5), 5);
    assert_eq!(Sample::min(-5i32, -10), -10);
    assert_eq!(Sample::min(-10i32, -5), -10);
    assert_eq!(Sample::min(0i32, 5), 0);

    assert_eq!(Sample::max(5i32, 10), 10);
    assert_eq!(Sample::max(10i32, 5), 10);
    assert_eq!(Sample::max(-5i32, -10), -5);
    assert_eq!(Sample::max(-10i32, -5), -5);
    assert_eq!(Sample::max(0i32, 5), 5);
}

#[test]
fn test_f32_min_max() {
    assert_eq!(Sample::min(-5.0f32, -10.0), -10.0);
    assert_eq!(Sample::min(-10.0f32, -5.0), -10.0);
    assert_eq!(Sample::min(0.0f32, 5.0), 0.0);
    assert_eq!(Sample::min(5.0f32, 10.0), 5.0);
    assert_eq!(Sample::min(10.0f32, 5.0), 5.0);

    assert_eq!(Sample::max(5.0f32, 10.0), 10.0);
    assert_eq!(Sample::max(10.0f32, 5.0), 10.0);
    assert_eq!(Sample::max(-5.0f32, -10.0), -5.0);
    assert_eq!(Sample::max(-10.0f32, -5.0), -5.0);
    assert_eq!(Sample::max(0.0f32, 5.0), 5.0);
}

#[test]
fn test_f32_nan_min_max() {
    assert_eq!(Sample::min(f32::NAN, 5.0), 5.0);
    assert_eq!(Sample::min(5.0f32, f32::NAN), 5.0);
    assert!(Sample::min(f32::NAN, f32::NAN).is_nan());

    assert_eq!(Sample::max(f32::NAN, 5.0), 5.0);
    assert_eq!(Sample::max(5.0f32, f32::NAN), 5.0);
    assert!(Sample::max(f32::NAN, f32::NAN).is_nan());
}
