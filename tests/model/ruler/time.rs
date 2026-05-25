//! Integration coverage for the time-axis camera, now owned by `Tracks::time_camera`. The
//! previous suite drove `ruler::Time` directly; with the camera lifted out of the ruler the
//! interesting behavior lives one level up. Sample-ix assertions assume a single
//! `TEST_SAMPLE_RATE` track is present.

use wavalyze::audio;
use wavalyze::model::tracks2::Tracks;
use wavalyze::rect::Rect;

const TEST_SAMPLE_RATE: u32 = 48_000;

fn insert_buffer(audio: &mut audio::manager::AudioManager, nr_samples: usize) -> audio::BufferId {
    let buffer = audio::buffer::BufferE::F32(audio::buffer::Buffer::with_size(
        TEST_SAMPLE_RATE,
        32,
        nr_samples,
    ));
    audio.buffers.insert(std::sync::Arc::new(buffer))
}

fn setup_tracks(
    screen_rect: Rect,
    samples_per_pixel: f64,
    nr_samples: usize,
) -> (Tracks, audio::manager::AudioManager) {
    let mut audio = audio::manager::AudioManager::default();
    let buffer_id = insert_buffer(&mut audio, nr_samples);
    let mut tracks = Tracks::default();
    tracks
        .add_track_to_end(
            buffer_id,
            TEST_SAMPLE_RATE,
            &wavalyze::model::config::TrackConfig::default(),
        )
        .unwrap();
    tracks.ruler.set_screen_rect(screen_rect);
    tracks
        .time_camera
        .set_seconds_per_pixel(samples_per_pixel / TEST_SAMPLE_RATE as f64);
    (tracks, audio)
}

#[test]
fn zoom_to_time_range_clamped_centers_target_range() {
    let screen_rect = Rect::new(0.0, 0.0, 1000.0, 100.0);
    let (mut tracks, _audio) = setup_tracks(screen_rect, 0.0, 1_000);

    let start_t = wavalyze::model::time_camera::sample_ix_to_time(100.0, TEST_SAMPLE_RATE);
    let end_t = wavalyze::model::time_camera::sample_ix_to_time(101.0, TEST_SAMPLE_RATE);
    tracks.zoom_to_time_range_clamped(start_t..end_t);

    let ix_range = tracks.ix_range().unwrap();
    assert_eq!(tracks.samples_per_pixel(), Some(0.002));
    assert!(((ix_range.start + ix_range.end) / 2.0 - 100.5).abs() < 1e-9);
}

#[test]
fn sample_ix_to_screen_x_maps_through_camera_at_reference_rate() {
    let screen_rect = Rect::new(100.0, 100.0, 1100.0, 140.0);
    let (mut tracks, _audio) = setup_tracks(screen_rect, 10.0, 100_000);
    assert_eq!(tracks.sample_ix_to_screen_x(5000.0), Some(600.0));

    // Pin the camera to the same window via `zoom_to_time_range` and confirm round-trip
    // behavior is preserved.
    let end_t = wavalyze::model::time_camera::sample_ix_to_time(10_000.0, TEST_SAMPLE_RATE);
    tracks.zoom_to_time_range(0.0..end_t);

    assert_eq!(tracks.sample_ix_to_screen_x(5000.0), Some(600.0));
    assert_eq!(tracks.screen_x_to_sample_ix(600.0), Some(5000.0));
}

#[test]
fn screen_x_to_sample_ix_is_none_when_no_tracks_present() {
    let mut tracks = Tracks::default();
    tracks
        .ruler
        .set_screen_rect(Rect::new(100.0, 0.0, 1100.0, 100.0));
    assert_eq!(tracks.sample_ix_to_screen_x(100.0), None);
}

#[test]
fn sample_ix_to_screen_x_basic() {
    let screen_rect = Rect::new(100.0, 0.0, 1100.0, 100.0);
    let (tracks, _audio) = setup_tracks(screen_rect, 10.0, 100_000);
    // screen width = 1000px. ix_range is [0.0, 10000.0) with time_start = 0.0
    // middle sample is 5000.0
    // middle of screen is 100.0 + 1000.0 * 0.5 = 600.0
    assert_eq!(tracks.sample_ix_to_screen_x(5000.0), Some(600.0));
}

#[test]
fn sample_ix_to_screen_x_start_edge() {
    let screen_rect = Rect::new(100.0, 0.0, 1100.0, 100.0);
    let (tracks, _audio) = setup_tracks(screen_rect, 10.0, 100_000);
    assert_eq!(tracks.sample_ix_to_screen_x(0.0), Some(100.0));
}

#[test]
fn zoom_x_changes_camera_state() {
    let screen_rect = Rect::new(0.0, 0.0, 1000.0, 100.0);
    let (mut tracks, _audio) = setup_tracks(screen_rect, 10.0, 100_000);
    let initial_spp = tracks.time_camera.seconds_per_pixel();

    // Zoom around the screen center — both time_start and seconds_per_pixel should move.
    tracks.zoom_x(10.0, 500.0);

    assert_ne!(tracks.time_camera.time_start, 0.0);
    assert_ne!(tracks.time_camera.seconds_per_pixel(), initial_spp);
}
