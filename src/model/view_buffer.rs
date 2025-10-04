use crate::pos;

///
/// Buffer to store audio data that has been transformed to fit the screen, but starting as
/// coordinates (0, 0)
///
/// Depending on the X-axis zoom level, i.e. in nr of samples per pixel, the data takes a different
/// form.
///
#[derive(Debug, Clone, PartialEq)]
pub enum ViewBuffer {
    /// Use this when the `pixels_per_sample < 1`
    SingleSamples(Vec<pos::Pos>),
    /// Use this when the `pixels_per_sample in [1, 2)`
    OneLine(Vec<pos::Pos>),
    /// Use this when the `pixels_per_sample >= 2`
    /// TODO: make some min, max struct?
    LinePerPixelColumn(Vec<[pos::Pos; 2]>),
}

impl ViewBuffer {
    pub fn len(&self) -> usize {
        match self {
            ViewBuffer::SingleSamples(v) => v.len(),
            ViewBuffer::OneLine(v) => v.len(),
            ViewBuffer::LinePerPixelColumn(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        match self {
            ViewBuffer::SingleSamples(v) => v.clear(),
            ViewBuffer::OneLine(v) => v.clear(),
            ViewBuffer::LinePerPixelColumn(v) => v.clear(),
        }
    }
}
