//! In-memory MVCC storage engine.
//!
//! A correct, dependency-free `StorageEngine` used by the embedded form factor,
//! tests, and as the reference implementation the RocksDB backend is checked
//! against. Each key maps to an append-only list of versions tagged with the
//! commit sequence that produced them; reads honor a transaction's `read_seq`
//! watermark for snapshot isolation, and `commit` applies all buffered writes
//! atomically under a single lock.

use crate::engine::{Key, KeyRange, Record, RecordIter, StorageEngine};
use crate::mvcc::Snapshot;
use crate::txn::{Txn, TxnId};
use chronos_common::{Error, Result, Timestamp};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
struct Version {
    commit_seq: u64,
    /// `None` is a tombstone (deleted at this version).
    value: Option<Vec<u8>>,
}

#[derive(Default)]
struct Store {
    /// Key -> versions in ascending `commit_seq` order.
    data: BTreeMap<Key, Vec<Version>>,
}

pub struct MemoryEngine {
    store: RwLock<Store>,
    /// Highest committed sequence number (the global MVCC clock).
    commit_seq: AtomicU64,
    next_txn: AtomicU64,
    /// Serializes commits so applied writes get a single, monotonic seq.
    commit_lock: Mutex<()>,
}

impl Default for MemoryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryEngine {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(Store::default()),
            commit_seq: AtomicU64::new(0),
            next_txn: AtomicU64::new(1),
            commit_lock: Mutex::new(()),
        }
    }

    fn now_ts() -> Timestamp {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Timestamp::from_millis(ms)
    }

    /// Read the value visible at `read_seq` for `key` (ignores staged writes).
    fn committed_at(&self, key: &Key, read_seq: u64) -> Option<Vec<u8>> {
        let store = self.store.read().expect("store poisoned");
        let versions = store.data.get(key)?;
        versions
            .iter()
            .rev()
            .find(|v| v.commit_seq <= read_seq)
            .and_then(|v| v.value.clone())
    }
}

impl StorageEngine for MemoryEngine {
    fn begin(&self) -> Result<Txn> {
        let id = TxnId(self.next_txn.fetch_add(1, Ordering::SeqCst));
        let read_seq = self.commit_seq.load(Ordering::SeqCst);
        Ok(Txn::new(id, Self::now_ts(), read_seq))
    }

    fn get(&self, txn: &Txn, key: &Key) -> Result<Option<Record>> {
        if let Some(staged) = txn.staged_get(key) {
            return Ok(staged);
        }
        Ok(self.committed_at(key, txn.read_seq))
    }

    fn put(&self, txn: &mut Txn, key: Key, val: Record) -> Result<()> {
        txn.stage_put(key, val);
        Ok(())
    }

    fn delete(&self, txn: &mut Txn, key: Key) -> Result<()> {
        txn.stage_delete(key);
        Ok(())
    }

    fn scan(&self, txn: &Txn, range: KeyRange) -> Result<RecordIter> {
        let store = self.store.read().expect("store poisoned");
        let mut merged: BTreeMap<Key, Option<Vec<u8>>> = BTreeMap::new();

        for (key, versions) in store.data.range(range.start.clone()..range.end.clone()) {
            if let Some(v) = versions.iter().rev().find(|v| v.commit_seq <= txn.read_seq) {
                merged.insert(key.clone(), v.value.clone());
            }
        }
        // Overlay this transaction's own staged writes (read-your-writes).
        for (key, val) in txn.staged() {
            if *key >= range.start && *key < range.end {
                merged.insert(key.clone(), val.clone());
            }
        }

        let items: Vec<(Key, Record)> = merged
            .into_iter()
            .filter_map(|(k, v)| v.map(|val| (k, val)))
            .collect();
        Ok(Box::new(items.into_iter()))
    }

    fn commit(&self, txn: Txn) -> Result<()> {
        let _guard = self
            .commit_lock
            .lock()
            .map_err(|_| Error::Storage("commit lock poisoned".to_string()))?;
        let seq = self.commit_seq.load(Ordering::SeqCst) + 1;
        {
            let mut store = self.store.write().expect("store poisoned");
            for (key, value) in txn.writes {
                store.data.entry(key).or_default().push(Version {
                    commit_seq: seq,
                    value,
                });
            }
        }
        // Publish the new watermark only after all writes are applied.
        self.commit_seq.store(seq, Ordering::SeqCst);
        Ok(())
    }

    fn snapshot(&self) -> Result<Snapshot> {
        Ok(Snapshot::new(TxnId(self.commit_seq.load(Ordering::SeqCst))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_isolation_hides_uncommitted_and_later_writes() {
        let engine = MemoryEngine::new();

        // T1 writes and commits k=v1.
        let mut t1 = engine.begin().unwrap();
        engine.put(&mut t1, b"k".to_vec(), b"v1".to_vec()).unwrap();
        engine.commit(t1).unwrap();

        // T2 begins (snapshot after v1).
        let t2 = engine.begin().unwrap();

        // T3 overwrites k=v2 and commits.
        let mut t3 = engine.begin().unwrap();
        engine.put(&mut t3, b"k".to_vec(), b"v2".to_vec()).unwrap();
        engine.commit(t3).unwrap();

        // T2 still sees v1 (snapshot isolation).
        assert_eq!(
            engine.get(&t2, &b"k".to_vec()).unwrap(),
            Some(b"v1".to_vec())
        );
        // A fresh transaction sees v2.
        let t4 = engine.begin().unwrap();
        assert_eq!(
            engine.get(&t4, &b"k".to_vec()).unwrap(),
            Some(b"v2".to_vec())
        );
    }

    #[test]
    fn read_your_own_writes_before_commit() {
        let engine = MemoryEngine::new();
        let mut t = engine.begin().unwrap();
        engine
            .put(&mut t, b"k".to_vec(), b"staged".to_vec())
            .unwrap();
        assert_eq!(
            engine.get(&t, &b"k".to_vec()).unwrap(),
            Some(b"staged".to_vec())
        );
    }

    #[test]
    fn scan_returns_committed_range() {
        let engine = MemoryEngine::new();
        let mut t = engine.begin().unwrap();
        engine.put(&mut t, b"a1".to_vec(), b"1".to_vec()).unwrap();
        engine.put(&mut t, b"a2".to_vec(), b"2".to_vec()).unwrap();
        engine.put(&mut t, b"b1".to_vec(), b"3".to_vec()).unwrap();
        engine.commit(t).unwrap();

        let t2 = engine.begin().unwrap();
        let got: Vec<_> = engine.scan(&t2, KeyRange::prefix(b"a")).unwrap().collect();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, b"a1".to_vec());
        assert_eq!(got[1].0, b"a2".to_vec());
    }
}
