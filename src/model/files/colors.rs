//! Automatic accents in Oklab: fixed perceived lightness and chroma, rotating hue.

pub(super) fn automatic_color(index: usize) -> egui::Color32 {
    // The golden angle spreads consecutive files around the hue circle.
    let hue = (25.0 + index as f64 * 137.507_764).to_radians();
    let a = 0.12 * hue.cos();
    let b = 0.12 * hue.sin();
    let l = (0.70 + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m = (0.70 - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s = (0.70 - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    let red = 4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s;
    let green = -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s;
    let blue = -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701 * s;
    egui::Rgba::from_rgb(red as f32, green as f32, blue as f32).into()
}
