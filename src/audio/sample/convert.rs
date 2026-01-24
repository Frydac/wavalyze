// use crate::audio::sample2::Sample;

// pub struct PcmConverter<const BITS: u32>;

// impl<const BITS: u32> PcmConverter<BITS> {
//     pub const FLT2PCM_FACTOR: u32 = (1_u32 << (BITS - 1));
//     pub const PCM2FLT_FACTOR: f32 = 1.0 / Self::FLT2PCM_FACTOR as f32;

//     pub fn to_pcm<T: Sample>(val: f32) -> T {
//         // The compiler replaces self::FACTOR with a literal number (e.g., 32768.0)
//         (val * Self::FLT2PCM_FACTOR as f32) as T
//     }

//     pub fn to_float<T: Sample>(val: T) -> f32 {
//         val as f32 * Self::PCM2FLT_FACTOR
//     }
// }

pub const fn float2pcm_factor(bit_depth: u32) -> u64 {
    1_u64 << (bit_depth - 1)
}

pub const fn pcm2float_factor(bit_depth: u32) -> f64 {
    1.0_f64 / (float2pcm_factor(bit_depth) as f64)
}

pub fn flt2pcm16(val: f32) -> i16 {
    // PcmConverter::<16>::to_pcm(val) as i16
    (val * float2pcm_factor(16) as f32) as i16
}

pub fn pcm162flt(val: i16) -> f64 {
    val as f64 / float2pcm_factor(16) as f64
}

pub fn flt2pcm24(val: f32) -> i32 {
    (val * float2pcm_factor(24) as f32) as i32
}

pub fn pcm242flt(val: i32) -> f64 {
    val as f64 / float2pcm_factor(24) as f64
}

pub fn flt2pcm32(val: f32) -> i32 {
    (val * float2pcm_factor(32) as f32) as i32
}

pub fn pcm322flt(val: i32) -> f64 {
    val as f64 / float2pcm_factor(32) as f64
}
