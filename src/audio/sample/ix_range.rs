use std::fmt::Debug;
use std::ops::Range;

pub type Ix = i64;
pub type IxRange = IxRangeG<Ix>;
pub type OptIx = Option<Ix>;
pub type FracIx = f64;
pub type FracIxRange = IxRangeG<FracIx>;

pub trait IxTrait: Debug + Default + Copy + PartialOrd + PartialEq + Clone {}

impl IxTrait for Ix {}
impl IxTrait for FracIx {}
impl IxTrait for OptIx {}

// Used for representing user-specified ranges where omission means from the start or to the end of
// the buffer
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct OptIxRange {
    pub start: Option<Ix>,
    pub end: Option<Ix>,
}

impl OptIxRange {
    // pub EMPTY: OptIxRange = OptIxRange { start: None, end: None };
    pub fn to_ix_range(&self, default_start: Ix, default_end: Ix) -> IxRange {
        let start = self.start.unwrap_or(default_start);
        let end = self.end.unwrap_or(default_end);
        IxRange { start, end }
    }
}

/// Half-open interval to represent sample indices into a sample buffer
/// We allow for floating point indices as we need sub-pixel precision when rendering
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub struct IxRangeG<T: IxTrait> {
    pub start: T,
    pub end: T,
}

impl<T: IxTrait + std::ops::Sub<Output = T> + std::ops::Add<Output = T> + std::ops::AddAssign>
    IxRangeG<T>
{
    pub fn len(&self) -> T {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.len() == T::default()
    }

    pub fn contains(&self, ix: T) -> bool {
        self.start <= ix && ix <= self.end
    }

    pub fn shift(&mut self, offset: T) {
        self.start += offset;
        self.end += offset;
    }

    pub fn include(&mut self, ix: T) {
        if self.start > ix {
            self.start = ix;
        }
        if self.end < ix {
            self.end = ix;
        }
    }
}

impl<T> From<Range<T>> for IxRangeG<T>
where
    T: IxTrait,
{
    fn from(r: Range<T>) -> Self {
        IxRangeG {
            start: r.start,
            end: r.end,
        }
    }
}
