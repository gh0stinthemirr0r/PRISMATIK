//! Purged + embargoed walk-forward splitter (`P4-QM-09`).
//!
//! For a sample at index `i` with fixed label horizon `h`, the label uses
//! information on `[i, i + h]` (inclusive). A training sample is **purged**
//! when that interval overlaps the test window `[test.start, test.end)`.
//!
//! An **embargo** of `e` bars drops training samples in
//! `[test.end, test.end + e)` (buffer immediately after the test window).
//!
//! Methodology attribution: purge/embargo follow López de Prado overlapping-label
//! hygiene. This module is PRISMATIK-owned — not a third-party product façade.

use crate::BacktestError;

/// Half-open index range `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexRange {
    /// Inclusive start.
    pub start: usize,
    /// Exclusive end.
    pub end: usize,
}

impl IndexRange {
    /// Create a range; returns `None` if `end < start`.
    pub fn new(start: usize, end: usize) -> Option<Self> {
        if end < start {
            None
        } else {
            Some(Self { start, end })
        }
    }

    /// Number of indices in the range.
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the range is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Whether `i` is in `[start, end)`.
    pub fn contains(&self, i: usize) -> bool {
        i >= self.start && i < self.end
    }
}

/// One purged + embargoed walk-forward fold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalkForwardFold {
    /// Contiguous train segments after purge + embargo holes.
    pub train: Vec<IndexRange>,
    /// Contiguous test window.
    pub test: IndexRange,
    /// Embargo window after test (`[test.end, test.end + embargo)` clipped).
    pub embargo: IndexRange,
}

/// Expanding walk-forward splits with purge + post-test embargo.
///
/// * Fold `k` uses test `[train_bars + k * test_bars, train_bars + (k+1) * test_bars)`.
/// * Candidate train = every index **not** in the test window (both sides), so
///   the post-test embargo is meaningful on a known small series.
/// * Purge drops train `i` when label interval `[i, i + label_horizon]` overlaps
///   the test window.
/// * Embargo drops train indices in `[test.end, test.end + embargo_bars)`.
///
/// Returns an empty `Vec` when no complete test window fits.
pub fn purged_embargoed_walk_forward(
    n_bars: usize,
    train_bars: usize,
    test_bars: usize,
    label_horizon: usize,
    embargo_bars: usize,
) -> Result<Vec<WalkForwardFold>, BacktestError> {
    if test_bars == 0 {
        return Err(BacktestError::InvalidWalkForward);
    }
    if train_bars == 0 {
        return Err(BacktestError::InvalidWalkForward);
    }
    if train_bars + test_bars > n_bars {
        return Ok(Vec::new());
    }

    let mut folds = Vec::new();
    let mut test_start = train_bars;
    while test_start + test_bars <= n_bars {
        let test_end = test_start + test_bars;
        let test = IndexRange {
            start: test_start,
            end: test_end,
        };
        let fold = purged_embargoed_split(n_bars, test, label_horizon, embargo_bars)?;
        folds.push(fold);
        test_start = test_end;
    }
    Ok(folds)
}

/// Single purged + embargoed split for a fixed test window.
///
/// Candidate train = `{0..n_bars} \ test`. Then purge overlapping labels and
/// drop the post-test embargo buffer from train.
pub fn purged_embargoed_split(
    n_bars: usize,
    test: IndexRange,
    label_horizon: usize,
    embargo_bars: usize,
) -> Result<WalkForwardFold, BacktestError> {
    if test.end > n_bars || test.start > test.end || test.is_empty() {
        return Err(BacktestError::InvalidWalkForward);
    }

    let embargo_end = test.end.saturating_add(embargo_bars).min(n_bars);
    let embargo = IndexRange {
        start: test.end.min(n_bars),
        end: embargo_end,
    };

    let mut keep = vec![true; n_bars];
    for slot in keep.iter_mut().take(test.end).skip(test.start) {
        *slot = false;
    }

    // Purge: label [i, i + label_horizon] overlaps [test.start, test.end).
    for (i, slot) in keep.iter_mut().enumerate() {
        if !*slot {
            continue;
        }
        if label_overlaps_test(i, label_horizon, test) {
            *slot = false;
        }
    }

    // Embargo: drop buffer immediately after the test window.
    for slot in keep.iter_mut().take(embargo.end).skip(embargo.start) {
        *slot = false;
    }

    let train = coalesce_ranges(&keep);
    Ok(WalkForwardFold {
        train,
        test,
        embargo,
    })
}

fn label_overlaps_test(i: usize, label_horizon: usize, test: IndexRange) -> bool {
    // Label covers [i, i + h] inclusive. Overlap with [test.start, test.end)
    // iff i < test.end && i + h >= test.start.
    let label_end = i.saturating_add(label_horizon);
    i < test.end && label_end >= test.start
}

fn coalesce_ranges(keep: &[bool]) -> Vec<IndexRange> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < keep.len() {
        if !keep[i] {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        while i < keep.len() && keep[i] {
            i += 1;
        }
        out.push(IndexRange { start, end: i });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known small series (n = 12):
    /// train_bars=4, test_bars=3, label_horizon=2, embargo_bars=1
    ///
    /// Fold 0: test = [4, 7)
    ///   candidates outside test: [0,4) ∪ [7,12)
    ///   purge: i with i+2 >= 4 and i < 7 → i in {2,3,4,5,6}; of candidates drop 2,3
    ///   embargo: [7, 8) dropped
    ///   train → [0, 2) ∪ [8, 12)
    ///
    /// Fold 1: test = [7, 10)
    ///   candidates: [0,7) ∪ [10,12)
    ///   purge: i+2 >= 7 and i < 10 → i in {5,6,7,8,9}; drop 5,6 from candidates
    ///   embargo: [10, 11)
    ///   train → [0, 5) ∪ [11, 12)
    #[test]
    fn known_small_series_purge_and_embargo() {
        let folds = purged_embargoed_walk_forward(12, 4, 3, 2, 1).unwrap();
        assert_eq!(folds.len(), 2);

        assert_eq!(folds[0].test, IndexRange { start: 4, end: 7 });
        assert_eq!(folds[0].embargo, IndexRange { start: 7, end: 8 });
        assert_eq!(
            folds[0].train,
            vec![
                IndexRange { start: 0, end: 2 },
                IndexRange { start: 8, end: 12 },
            ]
        );

        assert_eq!(folds[1].test, IndexRange { start: 7, end: 10 });
        assert_eq!(folds[1].embargo, IndexRange { start: 10, end: 11 });
        assert_eq!(
            folds[1].train,
            vec![
                IndexRange { start: 0, end: 5 },
                IndexRange { start: 11, end: 12 },
            ]
        );

        // No train index's label overlaps its fold's test window.
        for fold in &folds {
            for seg in &fold.train {
                for i in seg.start..seg.end {
                    assert!(
                        !label_overlaps_test(i, 2, fold.test),
                        "train {i} overlaps test {:?}",
                        fold.test
                    );
                    assert!(
                        !fold.embargo.contains(i),
                        "train {i} inside embargo {:?}",
                        fold.embargo
                    );
                }
            }
        }
    }

    #[test]
    fn zero_horizon_only_embargo_removes_post_test() {
        let fold = purged_embargoed_split(10, IndexRange { start: 4, end: 6 }, 0, 2).unwrap();
        // horizon 0: purge only exact test indices (already excluded).
        // embargo [6, 8) dropped from post-test train.
        assert_eq!(
            fold.train,
            vec![
                IndexRange { start: 0, end: 4 },
                IndexRange { start: 8, end: 10 },
            ]
        );
        assert_eq!(fold.embargo, IndexRange { start: 6, end: 8 });
    }

    #[test]
    fn rejects_zero_test_bars() {
        let err = purged_embargoed_walk_forward(10, 4, 0, 1, 1).unwrap_err();
        assert_eq!(err, BacktestError::InvalidWalkForward);
    }
}
