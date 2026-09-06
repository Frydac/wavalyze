use crate::{model::track::TrackId, pos};

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub struct HoverInfo {
    pub screen_pos: pos::Pos,
    pub sample_ix: f64,
    pub sample_pos_x: Option<f64>,
    pub track_id: Option<TrackId>,
}

impl HoverInfo {
    pub fn sample_pos_hovered(&self, sample_pos_x: f64, samples_per_pixel: f64) -> bool {
        let pixels_per_half_sample = 0.5 / samples_per_pixel;
        self.sample_pos_x.is_some_and(|x| {
            crate::math::compare::near_absolute(
                x as f32,
                sample_pos_x as f32,
                pixels_per_half_sample as f32,
            )
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
    pub fn sample_pos_is_hovered(&self, sample_pos_x: f64, samples_per_pixel: f64) -> bool {
        match self {
            HoverInfoE::NotHovered => false,
            HoverInfoE::IsHovered(hover_info) => {
                hover_info.sample_pos_hovered(sample_pos_x, samples_per_pixel)
            }
        }
    }
}

// HoverInfoE is the single source of truth for hover rendering state.
