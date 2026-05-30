//! The `UPSERT_FACT` write operator.
//!
//! Atomically, within one engine transaction: detect contradictions, close the
//! spans of invalidated facts, and append the new fact. This removes the
//! read-then-write race that framework-layer implementations suffer from.

use chronos_common::Result;
use chronos_storage::Txn;
use chronos_temporal::{ConflictPolicy, Fact, UpsertOutcome};

pub trait FactWriter {
    fn upsert_fact(
        &self,
        txn: &mut Txn,
        fact: Fact,
        policy: ConflictPolicy,
    ) -> Result<UpsertOutcome>;
}
