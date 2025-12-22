use crate::{audio::sample, rect::Rect};

pub fn sample_ix_to_screen_x(sample_ix: f64, sample_ix_range: sample::FracIxRange, screen_rect: Rect) -> Option<f32> {
    if !sample_ix_range.contains(sample_ix) {
        tracing::trace!("sample_ix {} not in ix_range {:?}", sample_ix, sample_ix_range);
        return None;
    }
    let sample_ix_offset = sample_ix - sample_ix_range.start;
    let sample_ix_frac = sample_ix_offset / sample_ix_range.len();
    let screen_x = screen_rect.left() + sample_ix_frac as f32 * screen_rect.width();
    Some(screen_x)
}

pub fn screen_x_to_sample_ix(screen_x: f32, sample_ix_range: sample::FracIxRange, screen_rect: Rect) -> Option<f64> {
    //     let ix_range = sample_ix_range.to_ix_range();
    if !screen_rect.contains_x(screen_x) {
        tracing::trace!("screen_x {} not in screen_rect {:?}", screen_x, screen_rect);
        return None;
    }
    let screen_x_offset = screen_x - screen_rect.left();
    let sample_ix_frac = screen_x_offset / screen_rect.width();
    let sample_ix = sample_ix_range.start + sample_ix_frac as f64 * sample_ix_range.len();
    Some(sample_ix)
}

// smallest multiple of m that is >= x
// e.g. -120, 50 -> -100
pub fn ceil_to_multiple(x: i64, m: i64) -> i64 {
    if x % m == 0 {
        x
    } else {
        x + (m - x.rem_euclid(m))
    }
}

// largest multiple of m that is <= x
pub fn floor_to_multiple(x: i64, m: i64) -> i64 {
    x - x.rem_euclid(m)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ceil_to_multiple() {
        assert_eq!(ceil_to_multiple(-120, 50), -100);
        assert_eq!(ceil_to_multiple(-100, 50), -100);
        assert_eq!(ceil_to_multiple(-50, 50), -50);
        assert_eq!(ceil_to_multiple(0, 50), 0);
        assert_eq!(ceil_to_multiple(50, 50), 50);
        assert_eq!(ceil_to_multiple(99, 50), 100);
        assert_eq!(ceil_to_multiple(100, 50), 100);
    }

    #[test]
    fn test_floor_to_multiple() {
        assert_eq!(floor_to_multiple(-120, 50), -150);
        assert_eq!(floor_to_multiple(-100, 50), -150);
        assert_eq!(floor_to_multiple(-50, 50), -50);
        assert_eq!(floor_to_multiple(0, 50), 0);
        assert_eq!(floor_to_multiple(50, 50), 0);
        assert_eq!(floor_to_multiple(99, 50), 50);
        assert_eq!(floor_to_multiple(100, 50), 50);
    }
}
