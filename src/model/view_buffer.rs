use crate::{pos, rect::Rect};

///
/// Buffer to store audio data that has been transformed to fit the screen, but starting as
/// coordinates (0, 0)
/// Top left is (0, 0)
/// Bottom right is (width, height)
///
/// Depending on the X-axis zoom level, i.e. in nr of samples per pixel, the data takes a different
/// form.
///
#[derive(Debug, Clone, PartialEq)]
pub enum ViewBufferE {
    /// Use this when the `pixels_per_sample < 1`
    SingleSamples(Vec<pos::Pos>),
    /// Use this when the `pixels_per_sample in [1, 2)`
    OneLine(Vec<pos::Pos>),
    /// Use this when the `pixels_per_sample >= 2`
    /// TODO: make some min, max struct?
    LinePerPixelColumn(Vec<[pos::Pos; 2]>),
}

impl ViewBufferE {
    pub fn len(&self) -> usize {
        match self {
            ViewBufferE::SingleSamples(v) => v.len(),
            ViewBufferE::OneLine(v) => v.len(),
            ViewBufferE::LinePerPixelColumn(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        match self {
            ViewBufferE::SingleSamples(v) => v.clear(),
            ViewBufferE::OneLine(v) => v.clear(),
            ViewBufferE::LinePerPixelColumn(v) => v.clear(),
        }
    }
}

pub struct ViewBuffer2 {
    samples_per_pixel: f32,
    view_rect: Rect,
    data: ViewBufferE,
}
