//! Storage engine for Chronos-Graph.
//!
//! The append-heavy bitemporal write pattern (invalidation = close span +
//! append new version) maps naturally onto an LSM tree. This crate defines the
//! `StorageEngine` trait boundary, a complete in-memory MVCC implementation
//! ([`MemoryEngine`]), and a RocksDB-backed implementation behind the `rocks`
//! feature.

pub mod codec;
pub mod engine;
pub mod interval_index;
pub mod memory;
pub mod mvcc;
pub mod txn;

#[cfg(feature = "rocks")]
pub mod rocks;

pub use engine::{Key, KeyRange, Record, RecordIter, StorageEngine};
pub use interval_index::{InMemoryIntervalIndex, IntervalIndex};
pub use memory::MemoryEngine;
pub use mvcc::Snapshot;
pub use txn::{Txn, TxnId};
