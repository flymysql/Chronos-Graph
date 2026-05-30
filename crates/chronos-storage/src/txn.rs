//! Transactions. A `Txn` carries the snapshot it reads from and buffers writes
//! until commit. WAL-backed durability is planned with the RocksDB backend.

use chronos_common::Timestamp;

/// Monotonic transaction id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TxnId(pub u64);

#[derive(Debug)]
pub struct Txn {
    pub id: TxnId,
    /// Transaction-time stamp assigned at `begin`.
    pub tx_time: Timestamp,
    /// Buffered writes, applied atomically on commit.
    pub(crate) writes: Vec<(Vec<u8>, Vec<u8>)>,
}

impl Txn {
    pub fn new(id: TxnId, tx_time: Timestamp) -> Self {
        Self {
            id,
            tx_time,
            writes: Vec::new(),
        }
    }

    pub fn stage(&mut self, key: Vec<u8>, val: Vec<u8>) {
        self.writes.push((key, val));
    }

    pub fn staged(&self) -> &[(Vec<u8>, Vec<u8>)] {
        &self.writes
    }
}
