//! Pending channel-pairing state for diffing two files: which channels of file A are compared
//! against which channels of file B. Shown by the view as a routing-matrix dialog.

use crate::wav::{self, read::ChIx};

/// A diff of two files awaiting channel-pair selection by the user.
///
/// Rows are channels of `file_a`, columns are channels of `file_b`. Any combination of cells may
/// be checked (many-to-many); each checked cell will produce one diff track.
#[derive(Debug)]
pub struct PendingDiffPairing {
    pub file_a: wav::ReadConfig,
    pub file_b: wav::ReadConfig,
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
        let checked = (0..ch_ixs_a.len())
            .map(|row| (0..ch_ixs_b.len()).map(|col| row == col).collect())
            .collect();
        Self {
            file_a,
            file_b,
            ch_ixs_a,
            ch_ixs_b,
            checked,
        }
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
