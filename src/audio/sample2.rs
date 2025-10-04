
pub trait Sample: Default + Copy + PartialOrd + PartialEq + Clone {
    // Needed for dealing with partial ordering of floats
    fn is_nan(&self) -> bool;

    const MAX: Self;
    const MIN: Self;
}
impl Sample for f32 {
    fn is_nan(&self) -> bool {
        f32::is_nan(*self)
    }
    const MAX: Self = f32::INFINITY;
    const MIN: Self = f32::NEG_INFINITY;
}
impl Sample for i32 {
    fn is_nan(&self) -> bool {
        false
    }
    const MAX: Self = i32::MAX;
    const MIN: Self = i32::MIN;
}
impl Sample for i16 {
    fn is_nan(&self) -> bool {
        false
    }
    const MAX: Self = i16::MAX;
    const MIN: Self = i16::MIN;
}
