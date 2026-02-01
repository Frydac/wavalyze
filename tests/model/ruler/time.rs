use tracing_test::traced_test;
use wavalyze::audio;
use wavalyze::model::ruler::time::Time;
use wavalyze::rect::Rect;

fn setup_time(screen_rect: Rect, samples_per_pixel: f64) -> Time {
    let mut time = Time::default();
    time.set_screen_rect(screen_rect);
    time.set_samples_per_pixel(samples_per_pixel);
    time
}

fn setup_time_with_ix_range(screen_rect: Rect, sample_range: audio::sample::FracIxRange) -> Time {
    let mut time = Time::default();
    time.set_screen_rect(screen_rect);
    time.zoom_to_ix_range(sample_range);
    time
}

#[test]
#[traced_test]
fn test_sample_ix_to_screen_x_001() {
    let screen_rect = Rect::new(100.0, 100.0, 1100.0, 140.0);
    let mut time = setup_time(screen_rect, 10.0);
    assert_eq!(time.sample_ix_to_screen_x(5000.0), Some(600.0));

    let ix_range_ref = audio::sample::FracIxRange {
        start: 0.0,
        end: 10000.0,
    };
    time.zoom_to_ix_range(ix_range_ref);
    let ix_range = time.ix_range();

    assert_eq!(ix_range, Some(ix_range_ref));
    dbg!(ix_range);
    assert_eq!(time.sample_ix_to_screen_x(5000.0), Some(600.0));

    assert_eq!(time.screen_x_to_sample_ix(600.0), Some(5000.0));
}

#[test]
#[traced_test]
fn test_sample_ix_to_screen_x_002() {
    let screen_rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let ix_range = audio::sample::FracIxRange {
        start: 0.0,
        end: 3.0,
    };
    let time = setup_time_with_ix_range(screen_rect, ix_range);
    for screen_x in screen_rect.x_range_inc_floor() {
        let sample_ix = time.screen_x_to_sample_ix(screen_x as f32).unwrap();
        let screen_x_2 = time.sample_ix_to_screen_x(sample_ix).unwrap();
        println!(
            "pix_ix {:2} -> sample_ix {:6.3} -> pix_ix2 {2:}",
            screen_x, sample_ix, screen_x_2
        );
    }

    let ix_range = time.ix_range().unwrap();
    dbg!(ix_range);
    for sample_ix in ix_range.start.floor() as i32..ix_range.end.floor() as i32 {
        let screen_x = time.sample_ix_to_screen_x(sample_ix as f64).unwrap();
        let sample_ix_2 = time.screen_x_to_sample_ix(screen_x).unwrap();
        println!(
            "sample_ix {:2} -> pix_ix {:5} -> sample_ix2 {:6.3}",
            sample_ix, screen_x, sample_ix_2
        );
    }
}

#[test]
fn test_sample_ix_to_screen_x_no_timeline() {
    let mut time = Time::default();
    time.set_screen_rect(Rect::new(100.0, 0.0, 1100.0, 100.0));
    assert_eq!(time.sample_ix_to_screen_x(100.0), None);
}

#[test]
fn test_sample_ix_to_screen_x_basic() {
    let screen_rect = Rect::new(100.0, 0.0, 1100.0, 100.0);
    let time = setup_time(screen_rect, 10.0);
    // screen width = 1000px. ix_range is [0.0, 10000.0) with ix_start = 0.0
    // middle sample is 5000.0
    // middle of screen is 100.0 + 1000.0 * 0.5 = 600.0
    assert_eq!(time.sample_ix_to_screen_x(5000.0), Some(600.0));
}

#[test]
fn test_sample_ix_to_screen_x_start_edge() {
    let screen_rect = Rect::new(100.0, 0.0, 1100.0, 100.0);
    let time = setup_time(screen_rect, 10.0);
    // screen width = 1000px. ix_range is [0.0, 10000.0)
    // start sample is 0.0
    // start of screen is 100.0
    assert_eq!(time.sample_ix_to_screen_x(0.0), Some(100.0));
}

#[test]
fn test_sample_ix_to_screen_x_end_edge() {
    let screen_rect = Rect::new(100.0, 0.0, 1100.0, 100.0);
    let time = setup_time(screen_rect, 10.0);
    // screen width = 1000px. ix_range is [0.0, 10000.0)
    // end of range is exclusive, but floating point inaccuracies can be tricky
    assert_eq!(time.sample_ix_to_screen_x(10000.0 + 0.000001), Some(1100.0));

    // a value at what should be the exclusive end might be included due to floating point representation
    assert!(time.sample_ix_to_screen_x(10000.0).is_some());

    // a value just before the end
    let just_before_end = 10000.0 - 0.000001;
    let expected_x = 100.0 + (just_before_end / 10000.0) as f32 * 1000.0;
    let actual_x = time.sample_ix_to_screen_x(just_before_end).unwrap();
    assert!((actual_x - expected_x).abs() < 1e-5);
    assert!(actual_x <= 1100.0);
}

#[test]
fn test_sample_ix_to_screen_x_out_of_bounds() {
    let screen_rect = Rect::new(100.0, 0.0, 1100.0, 100.0);
    let time = setup_time(screen_rect, 10.0);
    // screen width = 1000px. ix_range is [0.0, 10000.0)
    assert_eq!(time.sample_ix_to_screen_x(-1.0), Some(99.9));
    assert_eq!(time.sample_ix_to_screen_x(10000.1), Some(1100.01));
}

#[test]
#[traced_test]
fn test_zoom_x_001() {
    let screen_rect = Rect::new(0.0, 0.0, 1000.0, 100.0); // A 1000px wide screen
    let mut time = setup_time(screen_rect, 10.0); // 10 samples per pixel
    dbg!(&time);
    let sample_ix_range = time.ix_range().unwrap();
    dbg!(sample_ix_range);

    // let initial_ix_start = time.time_line.as_ref().unwrap().ix_start;
    // let initial_samples_per_pixel = time.time_line.as_ref().unwrap().samples_per_pixel;

    // Simulate zooming in by 100 pixels around the center of the screen
    time.zoom_x(10.0, 0.0);

    // Verify that the timeline still exists
    assert!(time.time_line.is_some());
    dbg!(&time);
    let sample_ix_range = time.ix_range().unwrap();
    dbg!(sample_ix_range);

    // let new_ix_start = time.time_line.as_ref().unwrap().ix_start;
    // let new_samples_per_pixel = time.time_line.as_ref().unwrap().samples_per_pixel;

    // // The ix_start and samples_per_pixel should have changed as a result of the zoom
    // // (even if the current implementation of zoom_x is incorrect, it should attempt to modify these values)
    // assert_ne!(new_ix_start, initial_ix_start);
    // assert_ne!(new_samples_per_pixel, initial_samples_per_pixel);
}
