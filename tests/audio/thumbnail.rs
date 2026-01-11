// use tracing::trace;
// use tracing_test::traced_test;
// use wavalyze::audio::buffer2::Buffer;
// use wavalyze::audio::thumbnail::{LevelData, Thumbnail, ThumbnailConfig};

// #[test]
// #[traced_test]
// fn test_thumbnail() {
//     let mut buffer = Buffer::<f32>::new(48000, 24);
//     let steps = 128;
//     buffer.data = (0..steps).map(|i| i as f32 / (steps - 1) as f32).collect();
//     // dbg!(&buffer);

//     let config = ThumbnailConfig {
//         samples_per_pixel_delta: 4,
//         min_nr_level_data_size: 2,
//     };
//     let thumbnail = Thumbnail::from_buffer(&buffer, Some(config));
//     // dbg!(&thumbnail);
//     // println!("thumbnail: {}", &thumbnail);

//     let view = thumbnail.get_sample_view((0..8).into(), 7.5).unwrap();
//     println!("view: {}", view);
//     // dbg!(view);
//     // let thumbnail = Thumbnail::from_leve(&buffer, config);
// }

// #[test]
// #[traced_test]
// fn test_level_data() {
//     trace!("test_level_data");
//     let mut buffer = Buffer::<f32>::new(48000, 24);
//     let steps = 16;
//     buffer.data = (0..steps).map(|i| i as f32 / (steps - 1) as f32).collect();
//     trace!("buffer before");
//     trace!("buffer: {:?}", &buffer);

//     let lvl_data = LevelData::<f32>::from_buffer(&buffer, 2);
//     trace!("lvl_data: {:?}", &lvl_data);

//     let view = lvl_data.get_sample_view((0..8).into(), 2.5).unwrap();
//     trace!("view: {:?}", view);
//     // buffer
// }
