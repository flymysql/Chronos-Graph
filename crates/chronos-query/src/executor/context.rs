//! Graph-to-text serialization: turn a selected subgraph into deduplicated,
//! linearized, citation-tagged text that an LLM can consume directly.

use chronos_common::Result;
use chronos_graph_model::Subgraph;
use chronos_provenance::Citation;

/// LLM-ready context block with attached citations.
#[derive(Debug, Default, Clone)]
pub struct ContextBlock {
    pub text: String,
    pub citations: Vec<Citation>,
}

pub trait ContextSerializer {
    fn as_context(&self, subgraph: &Subgraph, cite: bool) -> Result<ContextBlock>;
}
