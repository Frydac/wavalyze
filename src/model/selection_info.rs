use crate::audio::sample;

/// Round boundaries, keeping an exclusive end and at least one complete block.
pub(crate) fn snapped_selection_range(
    anchor: f64,
    current: f64,
    block_size: u64,
    toward_left: bool,
) -> sample::IxRange {
    let size = i128::from(block_size.max(1).min(i64::MAX as u64));
    let min_block = i128::from(i64::MIN).div_euclid(size)
        + i128::from(i128::from(i64::MIN).rem_euclid(size) != 0);
    let max_block = i128::from(i64::MAX) / size;
    let round = |sample: f64| ((sample / size as f64).round() as i128).clamp(min_block, max_block);
    let anchor = round(anchor);
    let current = round(current);
    let mut start = anchor.min(current);
    let mut end = anchor.max(current);
    if start == end {
        if toward_left && start > min_block || end == max_block {
            start -= 1;
        } else {
            end += 1;
        }
    }
    ((start * size) as i64..(end * size) as i64).into()
}

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
