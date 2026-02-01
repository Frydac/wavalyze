use std::collections::BTreeMap;
use std::f32::consts::TAU;

use anyhow::Result;

use crate::{
    audio::{
        self,
        buffer2::{Buffer, BufferE},
        thumbnail::ThumbnailE,
    },
    wav,
};

pub fn load_demo_waveform(model: &mut crate::model::Model) -> Result<()> {
    const SAMPLE_RATE: u32 = 48_000;
    const BIT_DEPTH: u16 = 32;
    let duration_s = 4.0_f32;
    let nr_samples = (SAMPLE_RATE as f32 * duration_s) as usize;

    model.tracks2.remove_all_tracks();
    model.files2.clear();
    model.audio = audio::manager::AudioManager::default();

    let mut ch1 = Buffer::with_capacity(SAMPLE_RATE, BIT_DEPTH, nr_samples);
    let mut ch2 = Buffer::with_capacity(SAMPLE_RATE, BIT_DEPTH, nr_samples);
    let mut ch3 = Buffer::with_capacity(SAMPLE_RATE, BIT_DEPTH, nr_samples);
    let mut ch4 = Buffer::with_capacity(SAMPLE_RATE, BIT_DEPTH, nr_samples);

    for i in 0..nr_samples {
        let t = i as f32 / SAMPLE_RATE as f32;

        // 1) Pure sine
        let s1 = (TAU * 220.0 * t).sin();

        // 2) Sine + harmonics (stable but richer)
        let s2 = (0.7 * (TAU * 220.0 * t).sin() + 0.2 * (TAU * 440.0 * t).sin() + 0.1 * (TAU * 880.0 * t).sin()).clamp(-1.0, 1.0);

        // 3) Chirp (linear sweep 80Hz -> 880Hz)
        let f0 = 80.0;
        let f1 = 880.0;
        let k = (f1 - f0) / duration_s.max(0.001);
        let phase = TAU * (f0 * t + 0.5 * k * t * t);
        let s3 = phase.sin();

        // 4) Drum hit: decaying sine burst
        let decay = (-t * 6.0).exp();
        let s4 = (TAU * 110.0 * t).sin() * decay;

        ch1.data.push(s1);
        ch2.data.push(s2);
        ch3.data.push(s3);
        ch4.data.push(s4);
    }

    let ch1_id = model.audio.buffers.insert(BufferE::F32(ch1));
    let ch2_id = model.audio.buffers.insert(BufferE::F32(ch2));
    let ch3_id = model.audio.buffers.insert(BufferE::F32(ch3));
    let ch4_id = model.audio.buffers.insert(BufferE::F32(ch4));

    for buffer_id in [ch1_id, ch2_id, ch3_id, ch4_id] {
        let buffer = model
            .audio
            .buffers
            .get(buffer_id)
            .ok_or_else(|| anyhow::anyhow!("Buffer {:?} not found", buffer_id))?;
        let thumbnail = ThumbnailE::from_buffer_e(buffer, None);
        model.audio.thumbnails.insert(buffer_id, thumbnail);
    }

    let mut channels = BTreeMap::new();
    channels.insert(
        0,
        wav::file2::Channel {
            ch_ix: 0,
            buffer_id: ch1_id,
            channel_id: Some(audio::Id::Left),
        },
    );
    channels.insert(
        1,
        wav::file2::Channel {
            ch_ix: 1,
            buffer_id: ch2_id,
            channel_id: Some(audio::Id::Right),
        },
    );
    channels.insert(
        2,
        wav::file2::Channel {
            ch_ix: 2,
            buffer_id: ch3_id,
            channel_id: Some(audio::Id::Center),
        },
    );
    channels.insert(
        3,
        wav::file2::Channel {
            ch_ix: 3,
            buffer_id: ch4_id,
            channel_id: Some(audio::Id::LFE),
        },
    );

    let file = wav::file2::File {
        channels,
        sample_type: audio::SampleType::Float,
        bit_depth: BIT_DEPTH,
        sample_rate: SAMPLE_RATE,
        layout: Some(audio::Layout::LAYOUT_4_0),
        path: None,
        nr_samples: nr_samples as u64,
    };

    model.tracks2.add_tracks_from_file(&file, &model.user_config.track)?;
    model.files2.push(file);

    Ok(())
}
