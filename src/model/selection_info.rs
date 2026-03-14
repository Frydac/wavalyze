use crate::audio::sample;

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct SelectionInfo {
    pub ix_rng: sample::IxRange,

    pub screen_x_start: f32,
    pub screen_x_end: f32,
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum SelectionInfoE {
    #[default]
    NotSelected,
    IsSelected(SelectionInfo),
}

impl SelectionInfoE {
    pub fn is_selected(&self) -> bool {
        matches!(self, SelectionInfoE::IsSelected(_))
    }
}
