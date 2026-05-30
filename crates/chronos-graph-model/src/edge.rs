//! Edges (relationships). Note that the *fact* — an edge carrying a bitemporal
//! span and provenance — is modelled in `chronos-temporal::Fact`. This `Edge`
//! is the structural, time-agnostic view used by traversal.

use chronos_common::{EdgeId, NodeId, PredicateId};

#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub id: EdgeId,
    pub subject: NodeId,
    pub predicate: PredicateId,
    pub object: NodeId,
}

impl Edge {
    pub fn new(id: EdgeId, subject: NodeId, predicate: PredicateId, object: NodeId) -> Self {
        Self {
            id,
            subject,
            predicate,
            object,
        }
    }
}
