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

#[derive(Debug, PartialEq, Clone, Default)]
pub struct HoverInfo3 {
    hover_info_current: HoverInfoE,  // while drawing, read this
    hover_info_next: HoverInfoE, // while drawing, write this
}

impl HoverInfo3 {
    pub fn update(&mut self, hover_info: HoverInfoE) {
        self.hover_info_next = hover_info;
    }

    pub fn get(&self) -> HoverInfoE {
        self.hover_info_current
    }

    pub fn next(&mut self) {
        self.hover_info_current = self.hover_info_next;
        self.hover_info_next = HoverInfoE::NotHovered;
    }
}
