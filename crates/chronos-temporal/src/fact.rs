//! The `Fact`: a bitemporal, provenance-bearing edge — the atomic unit of
//! knowledge in Chronos-Graph.

use chronos_common::{BitemporalSpan, EdgeId, NodeId, PredicateId, ProvenanceRef, VectorId};

#[derive(Debug, Clone)]
pub struct Fact {
    pub id: EdgeId,
    pub subject: NodeId,
    pub predicate: PredicateId,
    pub object: NodeId,
    pub span: BitemporalSpan,
    pub provenance: ProvenanceRef,
    /// Embedding of the fact's verbalized form, if indexed.
    pub embedding: Option<VectorId>,
}

impl Fact {
    /// Whether this fact is the currently-open (active) version.
    pub fn is_open(&self) -> bool {
        self.span.tx_to.is_none()
    }
}

/// A point-in-time materialized view: facts visible at a chosen `AsOf`.
#[derive(Debug, Default, Clone)]
pub struct FactView {
    pub facts: Vec<Fact>,
}
