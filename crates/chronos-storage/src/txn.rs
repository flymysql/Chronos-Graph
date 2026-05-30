//! Transactions. A `Txn` carries the snapshot it reads from and buffers writes
//! until commit. WAL-backed durability is provided by the RocksDB backend; the
//! in-memory backend applies buffered writes atomically under a commit lock.

use chronos_common::Timestamp;

/// Monotonic transaction id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TxnId(pub u64);

/// A staged write: `Some(bytes)` is a put, `None` is a delete (tombstone).
pub type StagedWrite = (Vec<u8>, Option<Vec<u8>>);

#[derive(Debug)]
pub struct Txn {
    pub id: TxnId,
    /// Transaction-time stamp assigned at `begin`.
    pub tx_time: Timestamp,
    /// Commit watermark this transaction reads at (MVCC snapshot isolation):
    /// only versions committed with `commit_seq <= read_seq` are visible.
    pub read_seq: u64,
    /// Buffered writes, applied atomically on commit.
    pub(crate) writes: Vec<StagedWrite>,
}

impl Txn {
    pub fn new(id: TxnId, tx_time: Timestamp, read_seq: u64) -> Self {
        Self {
            id,
            tx_time,
            read_seq,
            writes: Vec::new(),
        }
    }

    pub fn stage_put(&mut self, key: Vec<u8>, val: Vec<u8>) {
        self.writes.push((key, Some(val)));
    }

    pub fn stage_delete(&mut self, key: Vec<u8>) {
        self.writes.push((key, None));
    }

    pub fn staged(&self) -> &[StagedWrite] {
        &self.writes
    }

    /// Most recent staged value for `key` (read-your-own-writes), if any.
    /// Returns `Some(None)` when the latest staged op is a delete.
    pub(crate) fn staged_get(&self, key: &[u8]) -> Option<Option<Vec<u8>>> {
        self.writes
            .iter()
            .rev()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    }
}
