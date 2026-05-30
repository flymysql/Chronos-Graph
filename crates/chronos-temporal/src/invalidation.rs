//! Contradiction detection and fact invalidation.
//!
//! The `UPSERT_FACT` write operator (implemented in `chronos-query`) uses this
//! to atomically: detect a conflict, close the old fact's span, and append the
//! new one — all within a single engine transaction.

use crate::fact::Fact;
use chronos_common::Timestamp;

/// How to decide whether a new fact contradicts an existing one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    /// No conflict detection; always append.
    AppendOnly,
    /// `(subject, predicate)` must be unique among open facts; a new value
    /// invalidates the previous one.
    UniqueSubjectPredicate,
    /// Defer to an external (e.g. LLM-backed) semantic conflict judge.
    SemanticJudge,
}

/// Outcome of an upsert.
#[derive(Debug, Clone)]
pub struct UpsertOutcome {
    /// The newly written fact.
    pub written: Fact,
    /// Facts that were invalidated (span closed) as a result.
    pub invalidated: Vec<Fact>,
}

/// Decide which existing open facts a new fact invalidates, per policy.
///
/// This is pure decision logic; the actual span-closing write happens in the
/// transactional operator. `now` is the transaction timestamp.
pub fn facts_to_invalidate(
    new_fact: &Fact,
    existing_open: &[Fact],
    policy: ConflictPolicy,
    _now: Timestamp,
) -> Vec<Fact> {
    match policy {
        ConflictPolicy::AppendOnly => Vec::new(),
        ConflictPolicy::UniqueSubjectPredicate => existing_open
            .iter()
            .filter(|f| f.subject == new_fact.subject && f.predicate == new_fact.predicate)
            .cloned()
            .collect(),
        // Semantic judging is delegated upstream; engine-side this is a no-op
        // until a judge hook is wired in.
        ConflictPolicy::SemanticJudge => Vec::new(),
    }
}
