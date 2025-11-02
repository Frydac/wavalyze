use std::fmt::Debug;

// Represents a single audio sample value
pub trait Sample: Debug + Default + Copy + PartialOrd + PartialEq + Clone {
    // Needed for dealing with partial ordering of floats
    fn is_nan(&self) -> bool;

    fn min(self, other: Self) -> Self;
    fn max(self, other: Self) -> Self;

    const MAX: Self;
    const MIN: Self;
    const ZERO: Self;
}
impl Sample for f32 {
    fn is_nan(&self) -> bool {
        f32::is_nan(*self)
    }

    fn min(self, other: Self) -> Self {
        if self.is_nan() {
            return other;
        }
        if other.is_nan() {
            return self;
        }
        if self < other {
            self
        } else {
            other
        }
    }

    fn max(self, other: Self) -> Self {
        if self.is_nan() {
            return other;
        }
        if other.is_nan() {
            return self;
        }
        if self > other {
            self
        } else {
            other
        }
    }

    const MAX: Self = f32::INFINITY;
    const MIN: Self = f32::NEG_INFINITY;
    const ZERO: Self = 0.0;
}
impl Sample for i32 {
    fn is_nan(&self) -> bool {
        false
    }

    fn min(self, other: Self) -> Self {
        std::cmp::min(self, other)
    }

    fn max(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }

    const MAX: Self = i32::MAX;
    const MIN: Self = i32::MIN;
    const ZERO: Self = 0;
}
impl Sample for i16 {
    fn is_nan(&self) -> bool {
        false
    }

    fn min(self, other: Self) -> Self {
        std::cmp::min(self, other)
    }

    fn max(self, other: Self) -> Self {
        std::cmp::max(self, other)
    }

    const MAX: Self = i16::MAX;
    const MIN: Self = i16::MIN;
    const ZERO: Self = 0;
}
