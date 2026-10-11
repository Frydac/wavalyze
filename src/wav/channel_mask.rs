//! WAVEFORMATEXTENSIBLE speaker metadata, read independently of sample decoding.

use crate::audio::{Id, Layout};
use std::io::{self, Read, Seek, SeekFrom};

// WAV speaker order differs from the discriminants of audio::Id.
// Ordinary Auro mapping from Revision 2 (2024-04-19), pages 1-2:
// https://www.auro-3d.com/wp-content/uploads/2024/04/Auro-3D-in-multi-channel-WAV-White-Paper-20240419.pdf
// Complete 22_x masks override TopBackLeft/Right to height backs below.
const SPEAKERS: [Id; 24] = [
    Id::Left,
    Id::Right,
    Id::Center,
    Id::LFE,
    Id::LeftBack,
    Id::RightBack,
    Id::LeftCenter,
    Id::RightCenter,
    Id::CenterSurround,
    Id::LeftSurround,
    Id::RightSurround,
    Id::Top,
    Id::HeightLeft,
    Id::HeightCenter,
    Id::HeightRight,
    Id::HeightLeftSurround,
    Id::HeightCenterSurround,
    Id::HeightRightSurround,
    // Published Auro extension bits, not Microsoft-standard WAVEX positions.
    Id::LFE2,
    Id::HeightLeftSurround,
    Id::HeightRightSurround,
    Id::BottomLeft,
    Id::BottomCenter,
    Id::BottomRight,
];

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// Read the raw extensible channel mask from a RIFF/WAVE stream at its current position.
/// `None` means a non-extensible format; `Some(0)` means unspecified speakers.
/// The original stream position is restored on success and on parsing errors.
pub fn read_channel_mask<R: Read + Seek>(reader: &mut R) -> io::Result<Option<u32>> {
    let start = reader.stream_position()?;
    let result = read_mask(reader, start);
    reader.seek(SeekFrom::Start(start))?;
    result
}

fn read_mask<R: Read + Seek>(reader: &mut R, start: u64) -> io::Result<Option<u32>> {
    let mut riff = [0; 12];
    reader.read_exact(&mut riff)?;
    if &riff[..4] != b"RIFF" || &riff[8..] != b"WAVE" {
        return Err(invalid("expected RIFF/WAVE header"));
    }
    let size = u32::from_le_bytes(riff[4..8].try_into().unwrap());
    let end = start
        .checked_add(8 + u64::from(size))
        .ok_or_else(|| invalid("RIFF size overflow"))?;
    loop {
        let position = reader.stream_position()?;
        if position > end || end - position < 8 {
            return Err(invalid("missing fmt chunk"));
        }
        let mut chunk = [0; 8];
        reader.read_exact(&mut chunk)?;
        let len = u32::from_le_bytes(chunk[4..].try_into().unwrap());
        let padded_len = u64::from(len) + u64::from(len & 1);
        if padded_len > end - position - 8 {
            return Err(invalid("chunk exceeds RIFF size"));
        }
        if &chunk[..4] == b"fmt " {
            if len < 16 {
                return Err(invalid("fmt chunk is too short"));
            }
            let mut fmt = [0; 40];
            reader.read_exact(&mut fmt[..16])?;
            if u16::from_le_bytes(fmt[..2].try_into().unwrap()) != 0xfffe {
                return Ok(None);
            }
            if len < 40 {
                return Err(invalid("extensible fmt chunk is too short"));
            }
            reader.read_exact(&mut fmt[16..])?;
            let extension_len = u16::from_le_bytes(fmt[16..18].try_into().unwrap());
            if extension_len < 22 || u32::from(extension_len) > len - 18 {
                return Err(invalid("invalid extensible fmt extension size"));
            }
            return Ok(Some(u32::from_le_bytes(fmt[20..24].try_into().unwrap())));
        }
        if &chunk[..4] == b"data" {
            return Err(invalid("data precedes fmt chunk"));
        }
        reader.seek(SeekFrom::Current(padded_len as i64))?;
    }
}

fn speakers(mask: Option<u32>, channels: usize) -> Option<impl Iterator<Item = Id>> {
    let mask = mask?;
    if mask == 0 || mask >> SPEAKERS.len() != 0 || mask.count_ones() as usize != channels {
        return None;
    }
    // The channel count has been validated. 22.0 is derived by removing
    // LFE (bit 3) and LFE2 (bit 18) from the paper's complete 22.2 mask.
    let is_22_x = matches!(mask, 0xffffff | 0xfbfff7);
    let mut order = SPEAKERS;
    if is_22_x {
        order[15] = Id::HeightLeftBack;
        order[17] = Id::HeightRightBack;
    }
    let speakers = order
        .into_iter()
        .enumerate()
        .filter_map(move |(bit, id)| (mask & (1 << bit) != 0).then_some(id));
    // Ordinary top-back/top-side aliases must not assign two file channels
    // to the same identity. Validate the whole map before exposing any IDs.
    let identities = speakers
        .clone()
        .fold(0u32, |bits, id| bits | (1 << id as u32));
    if identities.count_ones() as usize != channels {
        return None;
    }
    Some(speakers)
}

pub(crate) fn layout(mask: Option<u32>, channels: usize) -> Option<Layout> {
    Some(speakers(mask, channels)?.fold(Layout::NONE, |layout, id| {
        layout | Layout::from_bits_retain(1 << id as u32)
    }))
}

pub(crate) fn channel_id(mask: Option<u32>, channels: usize, index: usize) -> Option<Id> {
    speakers(mask, channels)?.nth(index)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::audio::buffer::BufferE;
    use crate::wav::{
        ReadConfig, ReadConfigBytes, read_bytes_to_loaded_file_with_sink, read_to_loaded_file,
    };
    use std::io::Cursor;

    fn chunk(bytes: &mut Vec<u8>, id: &[u8; 4], payload: &[u8]) {
        bytes.extend_from_slice(id);
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(payload);
        if !payload.len().is_multiple_of(2) {
            bytes.push(0);
        }
    }

    pub(crate) fn wav_bytes(mask: Option<u32>, channels: u16, junk: &[u8]) -> Vec<u8> {
        let mut bytes = b"RIFF\0\0\0\0WAVE".to_vec();
        if !junk.is_empty() {
            chunk(&mut bytes, b"JUNK", junk);
        }
        let mut fmt = Vec::new();
        fmt.extend_from_slice(&(if mask.is_some() { 0xfffeu16 } else { 1 }).to_le_bytes());
        fmt.extend_from_slice(&channels.to_le_bytes());
        fmt.extend_from_slice(&48_000u32.to_le_bytes());
        fmt.extend_from_slice(&(48_000u32 * u32::from(channels) * 2).to_le_bytes());
        fmt.extend_from_slice(&(channels * 2).to_le_bytes());
        fmt.extend_from_slice(&16u16.to_le_bytes());
        if let Some(mask) = mask {
            fmt.extend_from_slice(&22u16.to_le_bytes());
            fmt.extend_from_slice(&16u16.to_le_bytes());
            fmt.extend_from_slice(&mask.to_le_bytes());
            fmt.extend_from_slice(&[
                1, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71,
            ]);
        }
        chunk(&mut bytes, b"fmt ", &fmt);
        let samples: Vec<u8> = (0..channels * 2)
            .flat_map(|sample| (sample as i16).to_le_bytes())
            .collect();
        chunk(&mut bytes, b"data", &samples);
        let size = (bytes.len() - 8) as u32;
        bytes[4..8].copy_from_slice(&size.to_le_bytes());
        bytes
    }

    #[test]
    fn reads_masks_and_restores_position_with_padded_unknown_chunks() {
        for mask in [None, Some(0), Some(0x60f), Some(0x80000000)] {
            let mut bytes = vec![99; 7];
            bytes.extend(wav_bytes(mask, 6, &[1, 2, 3]));
            let mut reader = Cursor::new(bytes);
            reader.set_position(7);
            assert_eq!(read_channel_mask(&mut reader).unwrap(), mask);
            assert_eq!(reader.position(), 7);
        }
    }

    #[test]
    fn rejects_malformed_headers_and_restores_position() {
        let valid = wav_bytes(Some(3), 2, &[]);
        let mut malformed = Vec::new();
        malformed.push(valid[..10].to_vec());
        malformed.push(valid[..59].to_vec()); // truncated SubFormat GUID
        let mut short_fmt = valid.clone();
        short_fmt[16..20].copy_from_slice(&24u32.to_le_bytes());
        malformed.push(short_fmt);
        for size in [0u16, 21, 23] {
            let mut bytes = valid.clone();
            bytes[36..38].copy_from_slice(&size.to_le_bytes());
            malformed.push(bytes);
        }
        let mut bad_riff = valid.clone();
        bad_riff[..4].copy_from_slice(b"RIFX");
        malformed.push(bad_riff);
        let mut oversized_chunk = valid.clone();
        oversized_chunk[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        malformed.push(oversized_chunk);
        for bytes in malformed {
            let mut reader = Cursor::new(bytes);
            assert!(read_channel_mask(&mut reader).is_err());
            assert_eq!(reader.position(), 0);
        }
    }

    #[test]
    fn maps_wav_speakers_instead_of_internal_bit_order() {
        assert_eq!(layout(Some(0x60f), 6), Some(Layout::LAYOUT_5_1));
        assert_eq!(
            layout(Some(0x3f), 6),
            Some(Layout::LOWER_FRONTS | Layout::CENTER | Layout::LFE | Layout::LOWER_BACKS)
        );
        assert_eq!(layout(Some(0x3ffff), 18).unwrap().bits().count_ones(), 18);
        // Sparse masks retain WAV order even where internal speaker order differs.
        assert_eq!(channel_id(Some(0x1c0), 3, 0), Some(Id::LeftCenter));
        assert_eq!(channel_id(Some(0x1c0), 3, 1), Some(Id::RightCenter));
        assert_eq!(channel_id(Some(0x1c0), 3, 2), Some(Id::CenterSurround));
        assert_eq!(channel_id(Some(0x1c0), 3, 3), None);
        assert_eq!(
            layout(Some(0x3f800), 7),
            Some(
                Layout::TOP
                    | Layout::HEIGHT_FRONTS
                    | Layout::HEIGHT_CENTER
                    | Layout::HEIGHT_SURROUNDS
                    | Layout::HEIGHT_CENTER_SURROUND
            ),
        );
        for mask in [None, Some(0), Some(3), Some(0x80000000)] {
            assert_eq!(layout(mask, 1), None);
            assert_eq!(channel_id(mask, 1, 0), None);
        }
    }

    #[test]
    fn ordinary_top_back_bits_are_height_surrounds() {
        // Revision 2 (2024-04-19): ordinary context uses HLs / HRs.
        // Complete 22_x masks use HLb / HRb instead (tested separately).
        for (mask, id, expected_layout) in [
            (
                0x008000,
                Id::HeightLeftSurround,
                Layout::HEIGHT_LEFT_SURROUND,
            ),
            (
                0x020000,
                Id::HeightRightSurround,
                Layout::HEIGHT_RIGHT_SURROUND,
            ),
        ] {
            assert_eq!(channel_id(Some(mask), 1, 0), Some(id));
            assert_eq!(layout(Some(mask), 1), Some(expected_layout));
        }
        assert_eq!(layout(Some(0x028000), 2), Some(Layout::HEIGHT_SURROUNDS));
        assert_eq!(
            channel_id(Some(0x028000), 2, 0),
            Some(Id::HeightLeftSurround)
        );
        assert_eq!(
            channel_id(Some(0x028000), 2, 1),
            Some(Id::HeightRightSurround)
        );
        assert_eq!(channel_id(Some(0x028000), 2, 2), None);
    }

    #[test]
    fn loading_auro_9_1_uses_pdf_channel_order_and_layout() {
        // Literal PDF Auro 9.1 mask and order, independent of SPEAKERS.
        let mask = 0x2d60f;
        let expected = [
            Id::Left,
            Id::Right,
            Id::Center,
            Id::LFE,
            Id::LeftSurround,
            Id::RightSurround,
            Id::HeightLeft,
            Id::HeightRight,
            Id::HeightLeftSurround,
            Id::HeightRightSurround,
        ];
        let config = ReadConfigBytes::new(None, wav_bytes(Some(mask), 10, &[]));
        let loaded = read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap();
        assert_eq!(loaded.channel_mask, Some(mask));
        assert_eq!(loaded.layout, Some(Layout::LAYOUT_AURO9_1));
        let file = loaded.into_file(&mut crate::audio::manager::Buffers::default());
        assert_eq!(file.layout, Some(Layout::LAYOUT_AURO9_1));
        for (index, id) in expected.into_iter().enumerate() {
            assert_eq!(file.channels[&index].channel_id, Some(id));
        }
    }

    #[test]
    fn pdf_extension_bits_have_distinct_auro_identities() {
        // Revision 2 Auro extension assignments, independent of SPEAKERS.
        for (mask, id, expected_layout) in [
            (0x040000, Id::LFE2, Layout::LFE2),
            (
                0x080000,
                Id::HeightLeftSurround,
                Layout::HEIGHT_LEFT_SURROUND,
            ),
            (
                0x100000,
                Id::HeightRightSurround,
                Layout::HEIGHT_RIGHT_SURROUND,
            ),
            (0x200000, Id::BottomLeft, Layout::BOTTOM_LEFT),
            (0x400000, Id::BottomCenter, Layout::BOTTOM_CENTER),
            (0x800000, Id::BottomRight, Layout::BOTTOM_RIGHT),
        ] {
            assert_eq!(channel_id(Some(mask), 1, 0), Some(id));
            assert_eq!(layout(Some(mask), 1), Some(expected_layout));
        }
    }

    fn check_loading_22_x(mask: u32) {
        // Literal Revision 2 order. Derived 22.0 omits LFE (bit 3) and LFE2 (bit 18).
        let order_22_2 = [
            Id::Left,
            Id::Right,
            Id::Center,
            Id::LFE,
            Id::LeftBack,
            Id::RightBack,
            Id::LeftCenter,
            Id::RightCenter,
            Id::CenterSurround,
            Id::LeftSurround,
            Id::RightSurround,
            Id::Top,
            Id::HeightLeft,
            Id::HeightCenter,
            Id::HeightRight,
            Id::HeightLeftBack,
            Id::HeightCenterSurround,
            Id::HeightRightBack,
            Id::LFE2,
            Id::HeightLeftSurround,
            Id::HeightRightSurround,
            Id::BottomLeft,
            Id::BottomCenter,
            Id::BottomRight,
        ];
        let order_22_0 = [
            Id::Left,
            Id::Right,
            Id::Center,
            Id::LeftBack,
            Id::RightBack,
            Id::LeftCenter,
            Id::RightCenter,
            Id::CenterSurround,
            Id::LeftSurround,
            Id::RightSurround,
            Id::Top,
            Id::HeightLeft,
            Id::HeightCenter,
            Id::HeightRight,
            Id::HeightLeftBack,
            Id::HeightCenterSurround,
            Id::HeightRightBack,
            Id::HeightLeftSurround,
            Id::HeightRightSurround,
            Id::BottomLeft,
            Id::BottomCenter,
            Id::BottomRight,
        ];
        for (fixture_mask, expected, expected_layout) in [
            (0xffffff, order_22_2.as_slice(), Layout::LAYOUT_NHK_22_2),
            (0xfbfff7, order_22_0.as_slice(), Layout::LAYOUT_NHK_22_0),
        ] {
            if fixture_mask != mask {
                continue;
            }
            let channels = expected.len();
            let config = ReadConfigBytes::new(None, wav_bytes(Some(mask), channels as u16, &[]));
            let loaded = read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap();
            assert_eq!(loaded.channel_mask, Some(mask));
            assert_eq!(loaded.total_nr_channels, channels);
            assert_eq!(loaded.layout, Some(expected_layout.clone()));
            let mut buffers = crate::audio::manager::Buffers::default();
            let file = loaded.into_file(&mut buffers);
            assert_eq!(file.channel_mask, Some(mask));
            assert_eq!(file.layout, Some(expected_layout));
            assert_eq!(file.channels.len(), channels);
            for (index, id) in expected.iter().copied().enumerate() {
                let channel = &file.channels[&index];
                assert_eq!(channel.channel_id, Some(id));
                let BufferE::I16(buffer) = buffers[channel.buffer_id].as_ref() else {
                    panic!("expected i16 channel samples");
                };
                // The fixture interleaves two frames of distinct channel samples.
                assert_eq!(buffer.data, [index as i16, (index + channels) as i16]);
            }
            assert_eq!(channel_id(Some(mask), channels, channels), None);
        }
    }

    #[test]
    fn loading_22_2_preserves_pdf_order_and_channel_signals() {
        check_loading_22_x(0xffffff);
    }

    #[test]
    fn loading_22_0_preserves_derived_order_and_channel_signals() {
        check_loading_22_x(0xfbfff7);
    }

    #[test]
    fn sparse_pdf_mask_and_selected_channels_keep_wav_indices() {
        // HLs, HRs, LFE2, BL, BR; skips intermediate bits, including BC.
        let mask = 0xa68000;
        let expected = [
            Id::HeightLeftSurround,
            Id::HeightRightSurround,
            Id::LFE2,
            Id::BottomLeft,
            Id::BottomRight,
        ];
        for (index, id) in expected.into_iter().enumerate() {
            assert_eq!(channel_id(Some(mask), 5, index), Some(id));
        }
        let config =
            ReadConfigBytes::new(None, wav_bytes(Some(mask), 5, &[])).with_ch_ixs([1, 3, 4]);
        let loaded = read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap();
        assert_eq!(
            loaded.layout,
            Some(
                Layout::HEIGHT_SURROUNDS
                    | Layout::BOTTOM_LEFT
                    | Layout::BOTTOM_RIGHT
                    | Layout::LFE2
            )
        );
        let mut buffers = crate::audio::manager::Buffers::default();
        let file = loaded.into_file(&mut buffers);
        assert_eq!(file.channels.len(), 3);
        for (index, id) in [
            (1, Id::HeightRightSurround),
            (3, Id::BottomLeft),
            (4, Id::BottomRight),
        ] {
            let channel = &file.channels[&index];
            assert_eq!(channel.channel_id, Some(id));
            let BufferE::I16(buffer) = buffers[channel.buffer_id].as_ref() else {
                panic!("expected i16 channel samples");
            };
            assert_eq!(buffer.data, [index as i16, (index + 5) as i16]);
        }
    }

    #[test]
    fn sparse_extension_masks_do_not_imply_22_x_context() {
        // LFE2 and top-side extensions alone do not select the complete 22_x mapping.
        let mask = 0x9c0000;
        let expected = [
            Id::LFE2,
            Id::HeightLeftSurround,
            Id::HeightRightSurround,
            Id::BottomRight,
        ];
        for (index, id) in expected.into_iter().enumerate() {
            assert_eq!(channel_id(Some(mask), 4, index), Some(id));
        }
        assert_eq!(
            layout(Some(mask), 4),
            Some(Layout::LFE2 | Layout::HEIGHT_SURROUNDS | Layout::BOTTOM_RIGHT)
        );
    }

    #[test]
    fn duplicate_identities_keep_raw_masks_and_samples_without_speaker_metadata() {
        // Ordinary top-back/top-side aliases collide. Incomplete 22_x masks
        // (including the memo-era 22.0 mask) must not bypass this rule.
        for mask in [0x088000u32, 0x120000, 0x1a8000, 0xfffff7, 0x7ffff7] {
            let channels = mask.count_ones() as usize;
            assert_eq!(layout(Some(mask), channels), None, "mask {mask:#08x}");
            for index in 0..channels {
                assert_eq!(channel_id(Some(mask), channels, index), None);
            }
            let config = ReadConfigBytes::new(None, wav_bytes(Some(mask), channels as u16, &[]));
            let loaded = read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap();
            assert_eq!(loaded.channel_mask, Some(mask));
            assert_eq!(loaded.layout, None);
            let mut buffers = crate::audio::manager::Buffers::default();
            let file = loaded.into_file(&mut buffers);
            assert_eq!(file.channel_mask, Some(mask));
            assert_eq!(file.layout, None);
            assert_eq!(file.channels.len(), channels);
            for index in 0..channels {
                let channel = &file.channels[&index];
                assert_eq!(channel.channel_id, None);
                let BufferE::I16(buffer) = buffers[channel.buffer_id].as_ref() else {
                    panic!("expected i16 channel samples");
                };
                assert_eq!(buffer.data, [index as i16, (index + channels) as i16]);
            }
        }
    }

    #[test]
    fn unsupported_bits_and_count_mismatches_keep_raw_masks_without_assigning_ids() {
        for (mask, channels) in (24..32).map(|bit| (1u32 << bit, 1)).chain([
            (0xffffff, 23),
            (0xfbfff7, 24),
            (0x040000, 2),
        ]) {
            assert_eq!(layout(Some(mask), channels), None);
            assert_eq!(channel_id(Some(mask), channels, 0), None);
            let config = ReadConfigBytes::new(None, wav_bytes(Some(mask), channels as u16, &[]));
            let loaded = read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap();
            assert_eq!(loaded.channel_mask, Some(mask));
            assert_eq!(loaded.layout, None);
            let file = loaded.into_file(&mut crate::audio::manager::Buffers::default());
            assert!(
                file.channels
                    .values()
                    .all(|channel| channel.channel_id.is_none())
            );
        }
    }

    #[test]
    fn loading_preserves_raw_masks_and_selected_speaker_indices() {
        for mask in [None, Some(0), Some(0x60f), Some(3), Some(0x80000000)] {
            let config = ReadConfigBytes {
                name: None,
                bytes: wav_bytes(mask, 6, &[1, 2]),
                ch_ixs: Some(vec![2, 4]),
                sample_range: Default::default(),
                sample_ix_offset: 0,
            };
            let loaded = read_bytes_to_loaded_file_with_sink(&config, 0, None).unwrap();
            assert_eq!(loaded.channel_mask, mask);
            assert_eq!(
                loaded.layout,
                if mask == Some(0x60f) {
                    Some(Layout::LAYOUT_5_1)
                } else {
                    None
                }
            );
            assert_eq!(loaded.channels[&2].nr_samples(), 2);
            let file = loaded.into_file(&mut crate::audio::manager::Buffers::default());
            assert_eq!(file.channel_mask, mask);
            assert_eq!(
                file.channels[&2].channel_id,
                (mask == Some(0x60f)).then_some(Id::Center)
            );
            assert_eq!(
                file.channels[&4].channel_id,
                (mask == Some(0x60f)).then_some(Id::LeftSurround)
            );
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn file_and_model_loading_preserve_extensible_metadata() {
        let path = std::path::PathBuf::from("target/test_output/channel_mask/side_surround.wav");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, wav_bytes(Some(0x60f), 6, &[])).unwrap();
        let loaded = read_to_loaded_file(&ReadConfig::new(&path).with_ch_ixs([4, 5])).unwrap();
        assert_eq!(loaded.channel_mask, Some(0x60f));
        let mut model = crate::model::Model::new();
        let id = model.add_loaded_file(loaded).unwrap();
        assert_eq!(model.files[id].layout, Some(Layout::LAYOUT_5_1));
        assert_eq!(model.files[id].channel_mask, Some(0x60f));
        assert_eq!(
            model.files[id].channels[&4].channel_id,
            Some(Id::LeftSurround)
        );
        assert_eq!(
            model.files[id].channels[&5].channel_id,
            Some(Id::RightSurround)
        );
    }
}
