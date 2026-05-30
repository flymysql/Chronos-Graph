//! Storage engine for Chronos-Graph.
//!
//! The append-heavy bitemporal write pattern (invalidation = close span +
//! append new version) maps naturally onto an LSM tree. The skeleton defines
//! the `StorageEngine` trait boundary plus an in-memory implementation; the
//! RocksDB backend is planned behind this boundary (see `rocks`).

pub mod codec;
pub mod engine;
pub mod interval_index;
pub mod mvcc;
pub mod rocks;
pub mod txn;

pub use engine::{Key, KeyRange, Record, RecordIter, StorageEngine};
pub use interval_index::IntervalIndex;
pub use mvcc::Snapshot;
pub use txn::Txn;
