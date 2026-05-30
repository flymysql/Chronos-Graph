//! MVCC snapshots for consistent reads.

use crate::txn::TxnId;

/// A read snapshot pinned at a transaction watermark. Reads see only versions
/// committed at or before `watermark`.
#[derive(Debug, Clone, Copy)]
pub struct Snapshot {
    pub watermark: TxnId,
}

impl Snapshot {
    pub fn new(watermark: TxnId) -> Self {
        Self { watermark }
    }
}
