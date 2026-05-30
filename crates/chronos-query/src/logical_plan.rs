//! Logical plan: a tree of relational/graph operators independent of physical
//! execution strategy.

use crate::ast::Query;
use chronos_common::{AsOf, Result, TokenBudget};

#[derive(Debug, Clone)]
pub enum LogicalOp {
    /// Seed retrieval (vector / full-text / scan), to be fused by the optimizer.
    Seed { similar_to: Option<String> },
    /// Semantic graph expansion up to `depth`.
    Expand { depth: u32 },
    /// Restrict to facts active at a bitemporal coordinate.
    AsOfFilter { at: AsOf },
    /// Token-budget subgraph selection.
    Budget { budget: TokenBudget },
    /// Serialize to LLM-ready context.
    Context { cite: bool },
}

#[derive(Debug, Default, Clone)]
pub struct LogicalPlan {
    pub ops: Vec<LogicalOp>,
}

pub fn build(q: &Query) -> Result<LogicalPlan> {
    let mut ops = vec![LogicalOp::Seed {
        similar_to: q.similar_to.clone(),
    }];
    if let Some(depth) = q.max_depth {
        ops.push(LogicalOp::Expand { depth });
    }
    if let Some(at) = q.as_of {
        ops.push(LogicalOp::AsOfFilter { at });
    }
    if let Some(budget) = q.budget {
        ops.push(LogicalOp::Budget { budget });
    }
    if q.return_context {
        ops.push(LogicalOp::Context { cite: q.cite });
    }
    Ok(LogicalPlan { ops })
}
