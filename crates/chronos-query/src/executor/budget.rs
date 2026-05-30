//! Token-budget subgraph selection.
//!
//! Instead of a fixed top-k truncation, select the subset of a candidate
//! subgraph that maximizes information coverage under a token budget
//! (submodular / greedy selection).

use chronos_common::{Result, TokenBudget};
use chronos_graph_model::Subgraph;

pub trait SubgraphBudgeter {
    /// Trim `candidate` to fit within `budget`, maximizing coverage.
    fn select(&self, candidate: Subgraph, budget: TokenBudget) -> Result<Subgraph>;
}
