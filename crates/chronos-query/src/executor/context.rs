//! Graph-to-text serialization: turn a selected subgraph into deduplicated,
//! linearized, citation-tagged text that an LLM can consume directly.

use chronos_common::Result;
use chronos_graph_model::Subgraph;
use chronos_provenance::Citation;
use std::collections::HashMap;

/// LLM-ready context block with attached citations.
#[derive(Debug, Default, Clone)]
pub struct ContextBlock {
    pub text: String,
    pub citations: Vec<Citation>,
}

pub trait ContextSerializer {
    fn as_context(&self, subgraph: &Subgraph, cite: bool) -> Result<ContextBlock>;
}

/// Default serializer: renders each edge as `subject predicate object`, using
/// node names resolved from the subgraph. Lines are deduplicated and rendered
/// in subgraph edge order. Predicates are rendered by id (`rel#<n>`) since the
/// structural `Edge` does not carry a label; richer verbalization with named
/// predicates is done by the embedded pipeline that has the full registry.
#[derive(Default)]
pub struct DefaultContextSerializer;

impl ContextSerializer for DefaultContextSerializer {
    fn as_context(&self, subgraph: &Subgraph, _cite: bool) -> Result<ContextBlock> {
        let names: HashMap<u64, &str> = subgraph
            .nodes
            .iter()
            .map(|n| (n.id.raw(), n.name.as_str()))
            .collect();

        let mut lines: Vec<String> = Vec::new();
        for e in &subgraph.edges {
            let s = names.get(&e.subject.raw()).copied().unwrap_or("?");
            let o = names.get(&e.object.raw()).copied().unwrap_or("?");
            let line = format!("- {s} rel#{} {o}", e.predicate.raw());
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
        Ok(ContextBlock {
            text: lines.join("\n"),
            citations: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_common::{EdgeId, NodeId, PredicateId};
    use chronos_graph_model::{Edge, Node};

    #[test]
    fn serializes_edges_with_names() {
        let mut sg = Subgraph::default();
        sg.nodes.push(Node::new(NodeId::new(1), "Alice"));
        sg.nodes.push(Node::new(NodeId::new(2), "Shanghai"));
        sg.edges.push(Edge::new(
            EdgeId::new(1),
            NodeId::new(1),
            PredicateId::new(7),
            NodeId::new(2),
        ));
        let block = DefaultContextSerializer.as_context(&sg, true).unwrap();
        assert_eq!(block.text, "- Alice rel#7 Shanghai");
    }
}
