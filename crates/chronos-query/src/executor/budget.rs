//! Token-budget subgraph selection.
//!
//! Instead of a fixed top-k truncation, select the prefix of a candidate
//! subgraph (assumed pre-ranked by relevance) that fits within a token budget.
//! Token cost is approximated by the word count of the names involved in each
//! edge. A fuller submodular coverage objective is future work.

use chronos_common::{Result, TokenBudget};
use chronos_graph_model::Subgraph;
use std::collections::HashMap;

pub trait SubgraphBudgeter {
    /// Trim `candidate` to fit within `budget`.
    fn select(&self, candidate: Subgraph, budget: TokenBudget) -> Result<Subgraph>;
}

/// Greedy budgeter: keeps edges (and their endpoint nodes) in order until the
/// running token estimate would exceed the budget.
#[derive(Default)]
pub struct GreedyBudgeter;

fn word_count(s: &str) -> usize {
    s.split_whitespace().count().max(1)
}

impl SubgraphBudgeter for GreedyBudgeter {
    fn select(&self, candidate: Subgraph, budget: TokenBudget) -> Result<Subgraph> {
        let name_tokens: HashMap<u64, usize> = candidate
            .nodes
            .iter()
            .map(|n| (n.id.raw(), word_count(&n.name)))
            .collect();

        let mut used = 0usize;
        let mut kept_edges = Vec::new();
        let mut kept_node_ids = std::collections::BTreeSet::new();

        for e in candidate.edges.into_iter() {
            let cost = name_tokens.get(&e.subject.raw()).copied().unwrap_or(1)
                + name_tokens.get(&e.object.raw()).copied().unwrap_or(1)
                + 1; // predicate
            if used + cost > budget.0 {
                break;
            }
            used += cost;
            kept_node_ids.insert(e.subject.raw());
            kept_node_ids.insert(e.object.raw());
            kept_edges.push(e);
        }

        let kept_nodes = candidate
            .nodes
            .into_iter()
            .filter(|n| kept_node_ids.contains(&n.id.raw()))
            .collect();

        Ok(Subgraph {
            nodes: kept_nodes,
            edges: kept_edges,
            episodes: Vec::new(),
            communities: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chronos_common::{EdgeId, NodeId, PredicateId};
    use chronos_graph_model::{Edge, Node};

    #[test]
    fn budget_truncates_low_ranked_tail() {
        let mut sg = Subgraph::default();
        for i in 1..=4u64 {
            sg.nodes.push(Node::new(NodeId::new(i), "x"));
        }
        // 2 edges, each cost = 1 + 1 + 1 = 3 tokens.
        sg.edges.push(Edge::new(
            EdgeId::new(1),
            NodeId::new(1),
            PredicateId::new(1),
            NodeId::new(2),
        ));
        sg.edges.push(Edge::new(
            EdgeId::new(2),
            NodeId::new(3),
            PredicateId::new(1),
            NodeId::new(4),
        ));

        let trimmed = GreedyBudgeter.select(sg, TokenBudget::new(3)).unwrap();
        assert_eq!(trimmed.edges.len(), 1);
        assert_eq!(trimmed.nodes.len(), 2);
    }
}
