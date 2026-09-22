//! Pending channel-pairing state for diffing two files: which channels of file A are compared
//! against which channels of file B. Shown by the view as a routing-matrix dialog.

use crate::{
    model::jobs::OffsetDetectionMode,
    wav::{self, read::ChIx},
};

/// A diff of two files awaiting channel-pair selection by the user.
///
/// Rows are channels of `file_a`, columns are channels of `file_b`. Any combination of cells may
/// be checked (many-to-many); each checked cell will produce one diff track.
#[derive(Debug)]
pub struct PendingDiffPairing {
    pub file_a: wav::ReadConfig,
    pub file_b: wav::ReadConfig,
    /// Optional automatic offset detection. Manual offsets remain stored as silence fallback.
    pub offset_detection_a: Option<OffsetDetectionMode>,
    pub offset_detection_b: Option<OffsetDetectionMode>,
    /// Channel indices shown as rows (A) / columns (B): the config's explicit selection if any,
    /// otherwise all channels in the file.
    pub ch_ixs_a: Vec<ChIx>,
    pub ch_ixs_b: Vec<ChIx>,
    /// checked[row][col] for row into `ch_ixs_a`, col into `ch_ixs_b`.
    pub checked: Vec<Vec<bool>>,
}

impl PendingDiffPairing {
    /// The diagonal (same position in the row/col lists) starts checked.
    pub fn new(
        file_a: wav::ReadConfig,
        file_b: wav::ReadConfig,
        ch_ixs_a: Vec<ChIx>,
        ch_ixs_b: Vec<ChIx>,
    ) -> Self {
        let checked = Self::default_pairing(ch_ixs_a.len(), ch_ixs_b.len());
        Self {
            file_a,
            file_b,
            offset_detection_a: None,
            offset_detection_b: None,
            ch_ixs_a,
            ch_ixs_b,
            checked,
        }
    }

    pub fn toggle_offset_detection_a(&mut self, mode: OffsetDetectionMode) {
        self.offset_detection_a = toggle_offset_detection(self.offset_detection_a, mode);
    }

    pub fn toggle_offset_detection_b(&mut self, mode: OffsetDetectionMode) {
        self.offset_detection_b = toggle_offset_detection(self.offset_detection_b, mode);
    }

    /// The checked cells as `(ch_ix_a, ch_ix_b)` pairs, in row-major order.
    pub fn selected_pairs(&self) -> Vec<(ChIx, ChIx)> {
        let mut pairs = Vec::new();
        for (row, ch_a) in self.ch_ixs_a.iter().enumerate() {
            for (col, ch_b) in self.ch_ixs_b.iter().enumerate() {
                if self.checked[row][col] {
                    pairs.push((*ch_a, *ch_b));
                }
            }
        }
        pairs
    }

    pub fn any_checked(&self) -> bool {
        self.checked.iter().flatten().any(|checked| *checked)
    }

    pub fn set_default_pairing(&mut self) {
        let a_len = self.ch_ixs_a.len();
        let b_len = self.ch_ixs_b.len();
        self.checked = Self::default_pairing(a_len, b_len);
    }

    fn default_pairing(a_len: usize, b_len: usize) -> Vec<Vec<bool>> {
        (0..a_len)
            .map(|row| (0..b_len).map(|col| row == col).collect())
            .collect()
    }

    pub fn clear(&mut self) {
        for row in &mut self.checked {
            for cell in row {
                *cell = false;
            }
        }
    }
}

fn toggle_offset_detection(
    current: Option<OffsetDetectionMode>,
    mode: OffsetDetectionMode,
) -> Option<OffsetDetectionMode> {
    (current != Some(mode)).then_some(mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairing(ch_ixs_a: Vec<ChIx>, ch_ixs_b: Vec<ChIx>) -> PendingDiffPairing {
        PendingDiffPairing::new(
            wav::ReadConfig::new("a.wav"),
            wav::ReadConfig::new("b.wav"),
            ch_ixs_a,
            ch_ixs_b,
        )
    }

    #[test]
    fn file_offsets_survive_pairing_setup() {
        let pairing = PendingDiffPairing::new(
            wav::ReadConfig::new("a.wav").with_sample_ix_offset(-12),
            wav::ReadConfig::new("b.wav").with_sample_ix_offset(34),
            vec![0],
            vec![0],
        );

        assert_eq!(pairing.file_a.sample_ix_offset, -12);
        assert_eq!(pairing.file_b.sample_ix_offset, 34);
    }

    #[test]
    fn offset_detection_selects_replaces_and_toggles_off() {
        let mut pairing = pairing(vec![0], vec![0]);

        pairing.toggle_offset_detection_a(OffsetDetectionMode::FirstNonZero);
        assert_eq!(
            pairing.offset_detection_a,
            Some(OffsetDetectionMode::FirstNonZero)
        );

        pairing.toggle_offset_detection_a(OffsetDetectionMode::LastLeadingZero);
        assert_eq!(
            pairing.offset_detection_a,
            Some(OffsetDetectionMode::LastLeadingZero)
        );

        pairing.toggle_offset_detection_a(OffsetDetectionMode::LastLeadingZero);
        assert_eq!(pairing.offset_detection_a, None);
    }

    #[test]
    fn offset_detection_keeps_manual_offsets_as_silence_fallback() {
        let mut pairing = PendingDiffPairing::new(
            wav::ReadConfig::new("a.wav").with_sample_ix_offset(12),
            wav::ReadConfig::new("b.wav").with_sample_ix_offset(-4),
            vec![0],
            vec![0],
        );

        pairing.toggle_offset_detection_a(OffsetDetectionMode::FirstNonZero);
        pairing.toggle_offset_detection_b(OffsetDetectionMode::LastLeadingZero);

        assert_eq!(pairing.file_a.sample_ix_offset, 12);
        assert_eq!(pairing.file_b.sample_ix_offset, -4);
        assert_eq!(
            pairing.offset_detection_b,
            Some(OffsetDetectionMode::LastLeadingZero)
        );
    }

    #[test]
    fn diagonal_is_checked_by_default() {
        let pairing = pairing(vec![0, 1], vec![0, 1, 2]);

        assert_eq!(pairing.selected_pairs(), vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn diagonal_uses_position_not_raw_channel_index() {
        let pairing = pairing(vec![2, 5], vec![1, 3]);

        assert_eq!(pairing.selected_pairs(), vec![(2, 1), (5, 3)]);
    }

    #[test]
    fn selected_pairs_reflects_toggled_cells() {
        let mut pairing = pairing(vec![0, 1, 2], vec![0, 1, 2]);
        pairing.checked[1][1] = false;
        pairing.checked[0][2] = true;

        assert_eq!(pairing.selected_pairs(), vec![(0, 0), (0, 2), (2, 2)]);
    }

    #[test]
    fn any_checked_reports_empty_selection() {
        let mut pairing = pairing(vec![0], vec![0]);
        assert!(pairing.any_checked());

        pairing.checked[0][0] = false;
        assert!(!pairing.any_checked());
    }
}
