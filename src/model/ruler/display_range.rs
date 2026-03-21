use crate::audio::sample;

use super::value_scale::ValueDisplayScale;

pub fn sample_to_display_range(
    sample_range: sample::ValRange<f64>,
    scale: ValueDisplayScale,
) -> sample::ValRange<f64> {
    sample::ValRange {
        min: scale.sample_to_display(sample_range.min),
        max: scale.sample_to_display(sample_range.max),
    }
}

pub fn display_to_sample_range(
    display_range: sample::ValRange<f64>,
    scale: ValueDisplayScale,
) -> sample::ValRange<f64> {
    sample::ValRange {
        min: scale.display_to_sample(display_range.min),
        max: scale.display_to_sample(display_range.max),
    }
}
