use crate::pos;

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct HoverInfo {
    pub screen_pos: pos::Pos,
    pub sample_ix: f64,
    pub sample_pos_x: Option<f64>,
}

impl HoverInfo {
    pub fn sample_pos_hovered(&self, sample_pos_x: f64) -> bool {
        self.sample_pos_x.is_some_and(|x| {
            crate::math::compare::near_absolute(x as f32, sample_pos_x as f32, 0.1)
        })
    }
}

#[derive(Debug, PartialEq, Clone, Default, Copy)]
pub enum HoverInfoE {
    #[default]
    NotHovered,
    IsHovered(HoverInfo),
}

impl HoverInfoE {
    // Given a sample position, are we hovering over that sample?
    // NOTE: utility function, as we have transformed our sample indices to screen positions and
    // don't store the sample ix (we probably should, it is not that much data, but this works too)
    pub fn sample_pos_is_hovered(&self, sample_pos_x: f64) -> bool {
        match self {
            HoverInfoE::NotHovered => false,
            HoverInfoE::IsHovered(hover_info) => hover_info.sample_pos_hovered(sample_pos_x),
        }
    }
}

// HoverInfoE is the single source of truth for hover rendering state.
