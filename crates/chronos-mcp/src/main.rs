//! Chronos-Graph MCP server binary (stdio JSON-RPC).

use chronos_embedded::FactStore;
use chronos_mcp::{run_stdio, McpState};
use std::sync::Arc;

fn main() -> std::io::Result<()> {
    let state = McpState::new(Arc::new(FactStore::new()));
    run_stdio(&state)
}
