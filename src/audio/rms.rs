use crate::audio;

pub fn rms(samples: &[f32]) -> f32 {
    let mut sum = 0.0;
    for sample in samples {
        sum += sample * sample;
    }
    (sum / samples.len() as f32).sqrt()
}

pub fn rms_db(samples: &[f32]) -> f32 {
    audio::db::gain_to_db(rms(samples))
}

