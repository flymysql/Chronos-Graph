//! Bitemporal core of Chronos-Graph.
//!
//! This crate owns the *fact* (a bitemporal edge with provenance), the rules
//! for invalidating contradictory facts, and point-in-time (`as-of`) evaluation.

pub mod as_of;
pub mod bitemporal;
pub mod fact;
pub mod invalidation;

pub use as_of::AsOfResolver;
pub use fact::{Fact, FactView};
pub use invalidation::{ConflictPolicy, UpsertOutcome};

// Re-export the shared bitemporal primitives for ergonomic use.
pub use chronos_common::{AsOf, BitemporalSpan, Timestamp};
