//! RocksDB-backed `StorageEngine` (enabled with the `rocks` feature).
//!
//! Provides durability (WAL + fsync on commit) and crash recovery via RocksDB.
//! A transaction buffers writes and applies them atomically as a `WriteBatch`
//! on commit. Reads are read-committed (latest) plus read-your-own staged
//! writes; full historical MVCC versioning (as in [`crate::MemoryEngine`]) is a
//! follow-up — `FactStore` only needs short-lived transactions, so this is
//! sufficient for M1.

use crate::engine::{Key, KeyRange, Record, RecordIter, StorageEngine};
use crate::mvcc::Snapshot;
use crate::txn::{Txn, TxnId};
use chronos_common::{Error, Result, Timestamp};
use rocksdb::{Direction, IteratorMode, Options, WriteBatch, WriteOptions, DB};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct RocksEngine {
    db: DB,
    next_txn: AtomicU64,
    commit_seq: AtomicU64,
}

impl RocksEngine {
    /// Open (creating if needed) a RocksDB-backed engine at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).map_err(|e| Error::Storage(e.to_string()))?;
        Ok(Self {
            db,
            next_txn: AtomicU64::new(1),
            commit_seq: AtomicU64::new(0),
        })
    }

    fn now_ts() -> Timestamp {
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        Timestamp::from_millis(ms)
    }
}

impl StorageEngine for RocksEngine {
    fn begin(&self) -> Result<Txn> {
        let id = TxnId(self.next_txn.fetch_add(1, Ordering::SeqCst));
        let read_seq = self.commit_seq.load(Ordering::SeqCst);
        Ok(Txn::new(id, Self::now_ts(), read_seq))
    }

    fn get(&self, txn: &Txn, key: &Key) -> Result<Option<Record>> {
        if let Some(staged) = txn.staged_get(key) {
            return Ok(staged);
        }
        self.db.get(key).map_err(|e| Error::Storage(e.to_string()))
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
        let mut merged: std::collections::BTreeMap<Key, Option<Vec<u8>>> = Default::default();
        let iter = self
            .db
            .iterator(IteratorMode::From(&range.start, Direction::Forward));
        for item in iter {
            let (k, v) = item.map_err(|e| Error::Storage(e.to_string()))?;
            let key = k.to_vec();
            if key >= range.end {
                break;
            }
            merged.insert(key, Some(v.to_vec()));
        }
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
        let mut batch = WriteBatch::default();
        for (key, value) in txn.staged() {
            match value {
                Some(v) => batch.put(key, v),
                None => batch.delete(key),
            }
        }
        let mut wo = WriteOptions::default();
        wo.set_sync(true); // fsync the WAL: survive crashes.
        self.db
            .write_opt(batch, &wo)
            .map_err(|e| Error::Storage(e.to_string()))?;
        self.commit_seq.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn snapshot(&self) -> Result<Snapshot> {
        Ok(Snapshot::new(TxnId(self.commit_seq.load(Ordering::SeqCst))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("chronos-rocks-{tag}-{nanos}"));
        p
    }

    #[test]
    fn durable_across_reopen() {
        let path = temp_path("reopen");
        {
            let engine = RocksEngine::open(&path).unwrap();
            let mut t = engine.begin().unwrap();
            engine.put(&mut t, b"k".to_vec(), b"v".to_vec()).unwrap();
            engine.commit(t).unwrap();
        }
        // Reopen: data must survive (WAL/SST recovery).
        {
            let engine = RocksEngine::open(&path).unwrap();
            let t = engine.begin().unwrap();
            assert_eq!(engine.get(&t, &b"k".to_vec()).unwrap(), Some(b"v".to_vec()));
        }
        let _ = DB::destroy(&Options::default(), &path);
    }
}
