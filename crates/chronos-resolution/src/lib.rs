//! [Phase 2] Entity resolution.
//!
//! Merges surface variants ("OpenAI" / "Open AI" / "OpenAI Inc.") into one
//! canonical node. Merging must consistently reconcile the merged entities'
//! edges, bitemporal spans, and provenance links.

use chronos_common::{NodeId, Result};

pub trait EntityResolver: Send + Sync {
    /// Candidate node ids that likely refer to the same entity as `node`.
    fn candidates(&self, node: NodeId) -> Result<Vec<NodeId>>;
    /// Merge `from` into `into`, reconciling edges/spans/provenance.
    fn merge(&mut self, into: NodeId, from: NodeId) -> Result<()>;
}
