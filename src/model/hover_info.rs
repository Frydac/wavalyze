use crate::pos;

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct HoverInfo {
    pub screen_pos: pos::Pos,
    pub sample_ix: f64,
}

#[derive(Debug, PartialEq, Clone, Default, Copy)]
pub enum HoverInfoE {
    #[default]
    NotHovered,
    IsHovered(HoverInfo),
}

// HoverInfoE is the single source of truth for hover rendering state.
