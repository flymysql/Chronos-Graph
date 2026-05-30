//! Shared types used across all Chronos-Graph crates.
//!
//! Keeping identifiers, time and the bitemporal span here avoids dependency
//! cycles between the storage and temporal layers (both need them).

pub mod config;
pub mod error;
pub mod ids;
pub mod time;

pub use error::{Error, Result};
pub use ids::{ChunkId, DocId, EdgeId, NodeId, PredicateId, ProvenanceRef, VectorId};
pub use time::{AsOf, BitemporalSpan, Timestamp};

/// A token budget for context assembly (number of LLM tokens).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenBudget(pub usize);

impl TokenBudget {
    pub const fn new(tokens: usize) -> Self {
        Self(tokens)
    }
}
