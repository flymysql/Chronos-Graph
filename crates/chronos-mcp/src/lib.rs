//! Built-in MCP server.
//!
//! Exposes the engine's memory operations as agent-callable tools. Multi-hop
//! retrieval is completed server-side to cut down agentic loop round-trips.

use chronos_common::{AsOf, Result, TokenBudget};
use chronos_graph_model::Episode;
use chronos_query::ContextBlock;

/// Tools surfaced to agents over MCP.
pub trait McpTools: Send + Sync {
    /// Ingest a new memory episode (message / document chunk).
    fn add_memory(&self, episode: Episode) -> Result<()>;

    /// Retrieve LLM-ready context for `query`, completing multi-hop server-side.
    fn search_memory(&self, query: &str, budget: TokenBudget, at: AsOf) -> Result<ContextBlock>;
}
