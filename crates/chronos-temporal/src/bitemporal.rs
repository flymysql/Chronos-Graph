//! Helpers over the shared `BitemporalSpan`. The primitive type lives in
//! `chronos-common` to keep storage and temporal layers free of cycles.

use chronos_common::{BitemporalSpan, Timestamp};

/// Do two facts' valid-time windows overlap? Used by uniqueness-style conflict
/// policies to detect contradictions.
pub fn valid_overlaps(a: &BitemporalSpan, b: &BitemporalSpan) -> bool {
    let a_hi = a.valid_to.unwrap_or(Timestamp::MAX);
    let b_hi = b.valid_to.unwrap_or(Timestamp::MAX);
    a.valid_from < b_hi && b.valid_from < a_hi
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(vf: i64, vt: Option<i64>) -> BitemporalSpan {
        BitemporalSpan {
            valid_from: Timestamp::from_millis(vf),
            valid_to: vt.map(Timestamp::from_millis),
            tx_from: Timestamp::from_millis(0),
            tx_to: None,
        }
    }

    #[test]
    fn overlapping_windows_detected() {
        assert!(valid_overlaps(&span(0, Some(100)), &span(50, Some(150))));
        assert!(!valid_overlaps(&span(0, Some(100)), &span(100, Some(200))));
        assert!(valid_overlaps(&span(0, None), &span(1000, None)));
    }
}
