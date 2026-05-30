//! The key/value storage abstraction that backends implement.

use crate::mvcc::Snapshot;
use crate::txn::Txn;
use chronos_common::Result;

/// Opaque, ordered key. Concrete encodings live in `codec`.
pub type Key = Vec<u8>;
/// Raw stored value bytes.
pub type Record = Vec<u8>;

/// An inclusive-start, exclusive-end key range for scans.
#[derive(Debug, Clone)]
pub struct KeyRange {
    pub start: Key,
    pub end: Key,
}

/// Iterator over `(key, value)` records returned by a scan.
pub type RecordIter = Box<dyn Iterator<Item = (Key, Record)>>;

/// The storage engine contract. Implementations must provide MVCC snapshot
/// isolation so reads are consistent with a point-in-time view.
pub trait StorageEngine: Send + Sync {
    fn begin(&self) -> Result<Txn>;
    fn get(&self, txn: &Txn, key: &Key) -> Result<Option<Record>>;
    fn put(&self, txn: &mut Txn, key: Key, val: Record) -> Result<()>;
    fn scan(&self, txn: &Txn, range: KeyRange) -> Result<RecordIter>;
    fn commit(&self, txn: Txn) -> Result<()>;
    fn snapshot(&self) -> Result<Snapshot>;
}
