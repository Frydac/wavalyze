use wavalyze::model::TimeCamera;
use wavalyze::rect::Rect;

#[test]
fn defaults_have_zero_zoom_and_zero_start() {
    let camera = TimeCamera::default();
    assert_eq!(camera.seconds_per_pixel(), 0.0);
    assert_eq!(camera.time_start, 0.0);

    let time_range = camera.time_range(100.0);
    assert_eq!(time_range.start, 0.0);
    assert_eq!(time_range.end, 0.0);
}

#[test]
fn time_range_scales_with_zoom_and_offset() {
    let mut camera = TimeCamera::default();

    // 0.01 s/px over 100 px = 1 s window starting at 0.
    camera.set_seconds_per_pixel(0.01);
    let time_range = camera.time_range(100.0);
    assert!((time_range.start - 0.0).abs() < 1e-9);
    assert!((time_range.end - 1.0).abs() < 1e-9);

    // Shift left by 0.5 s — window slides without changing length.
    camera.time_start = -0.5;
    let time_range = camera.time_range(100.0);
    assert!((time_range.start - -0.5).abs() < 1e-9);
    assert!((time_range.end - 0.5).abs() < 1e-9);
}

#[test]
fn time_screen_x_round_trip() {
    let mut camera = TimeCamera::default();
    camera.set_seconds_per_pixel(0.01);
    camera.time_start = 2.0;
    let screen_rect = Rect::new(100.0, 0.0, 200.0, 50.0);

    for screen_x in [100.0_f32, 125.0, 150.0, 200.0] {
        let time = camera.screen_x_to_time(screen_x, screen_rect);
        let round_trip = camera.time_to_screen_x(time, screen_rect);
        assert!((round_trip - screen_x).abs() < 1e-3);
    }
}

#[test]
fn sample_ix_helpers_invert_at_a_given_sample_rate() {
    use wavalyze::model::time_camera::{sample_ix_to_time, time_to_sample_ix};
    let sample_rate = 48_000;
    let ix = 4_800.5;
    let time = sample_ix_to_time(ix, sample_rate);
    let back = time_to_sample_ix(time, sample_rate);
    assert!((back - ix).abs() < 1e-9);
}
