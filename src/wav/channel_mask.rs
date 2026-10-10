//! WAVEFORMATEXTENSIBLE speaker metadata, read independently of sample decoding.

use crate::audio::{Id, Layout};
use std::io::{self, Read, Seek, SeekFrom};

// WAV speaker order differs from the discriminants of audio::Id.
const SPEAKERS: [Id; 18] = [
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
    Some(
        SPEAKERS
            .into_iter()
            .enumerate()
            .filter_map(move |(bit, id)| (mask & (1 << bit) != 0).then_some(id)),
    )
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
                    | Layout::HEIGHT_BACKS
                    | Layout::HEIGHT_CENTER_SURROUND
            ),
        );
        for mask in [None, Some(0), Some(3), Some(0x80000000)] {
            assert_eq!(layout(mask, 1), None);
            assert_eq!(channel_id(mask, 1, 0), None);
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
