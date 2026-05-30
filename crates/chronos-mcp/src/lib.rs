//! Built-in MCP server.
//!
//! Exposes the engine's memory operations as agent-callable tools over JSON-RPC
//! 2.0 (stdio transport). Multi-hop retrieval is completed server-side, cutting
//! down agentic loop round-trips.
//!
//! Implemented methods: `initialize`, `tools/list`, `tools/call`
//! (tools: `add_memory`, `search_memory`).

use chronos_common::{ChunkId, DocId, Timestamp};
use chronos_embedded::{FactStore, MemoryRetriever};
use chronos_temporal::ConflictPolicy;
use serde_json::{json, Value};
use std::sync::Arc;

pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Server state shared across requests.
#[derive(Clone)]
pub struct McpState {
    pub store: Arc<FactStore>,
}

impl McpState {
    pub fn new(store: Arc<FactStore>) -> Self {
        Self { store }
    }
}

/// Tool descriptors advertised by `tools/list`.
fn tool_descriptors() -> Value {
    json!([
        {
            "name": "add_memory",
            "description": "Ingest a fact (subject predicate object) valid from a real-world time.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "subject": {"type": "string"},
                    "predicate": {"type": "string"},
                    "object": {"type": "string"},
                    "valid_from": {"type": "integer"},
                    "doc": {"type": "integer"},
                    "chunk": {"type": "integer"}
                },
                "required": ["subject", "predicate", "object"]
            }
        },
        {
            "name": "search_memory",
            "description": "Retrieve cited, point-in-time context for a query in extended Cypher.",
            "inputSchema": {
                "type": "object",
                "properties": { "query": {"type": "string"} },
                "required": ["query"]
            }
        },
        {
            "name": "list_communities",
            "description": "List level-0 entity communities with templated summaries (global view).",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "resolve_entities",
            "description": "Merge surface-variant entities above a name-similarity threshold (0..1).",
            "inputSchema": {
                "type": "object",
                "properties": { "threshold": {"type": "number"} }
            }
        }
    ])
}

/// Handle a single JSON-RPC request value and produce the response value.
pub fn handle_request(state: &McpState, req: &Value) -> Value {
    let id = req.get("id").cloned().unwrap_or(Value::Null);
    let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

    match method {
        "initialize" => ok(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "serverInfo": {"name": "chronos-mcp", "version": env!("CARGO_PKG_VERSION")},
                "capabilities": {"tools": {}}
            }),
        ),
        "tools/list" => ok(id, json!({ "tools": tool_descriptors() })),
        "tools/call" => handle_tool_call(state, id, req),
        other => err(id, -32601, &format!("method not found: {other}")),
    }
}

fn handle_tool_call(state: &McpState, id: Value, req: &Value) -> Value {
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or(json!({}));

    match name {
        "add_memory" => match tool_add_memory(state, &args) {
            Ok(text) => ok(id, tool_text(&text)),
            Err(e) => err(id, -32602, &e),
        },
        "search_memory" => match tool_search_memory(state, &args) {
            Ok(text) => ok(id, tool_text(&text)),
            Err(e) => err(id, -32602, &e),
        },
        "list_communities" => match tool_list_communities(state) {
            Ok(text) => ok(id, tool_text(&text)),
            Err(e) => err(id, -32602, &e),
        },
        "resolve_entities" => match tool_resolve_entities(state, &args) {
            Ok(text) => ok(id, tool_text(&text)),
            Err(e) => err(id, -32602, &e),
        },
        other => err(id, -32602, &format!("unknown tool: {other}")),
    }
}

fn tool_add_memory(state: &McpState, args: &Value) -> Result<String, String> {
    let get_str = |k: &str| args.get(k).and_then(|v| v.as_str()).map(|s| s.to_string());
    let subject = get_str("subject").ok_or("missing 'subject'")?;
    let predicate = get_str("predicate").ok_or("missing 'predicate'")?;
    let object = get_str("object").ok_or("missing 'object'")?;
    let valid_from = args.get("valid_from").and_then(|v| v.as_i64()).unwrap_or(0);
    let doc = args.get("doc").and_then(|v| v.as_u64()).unwrap_or(0);
    let chunk = args.get("chunk").and_then(|v| v.as_u64()).unwrap_or(0);

    let edge = state
        .store
        .ingest(
            &subject,
            &predicate,
            &object,
            Timestamp::from_millis(valid_from),
            DocId::new(doc),
            ChunkId::new(chunk),
            ConflictPolicy::UniqueSubjectPredicate,
        )
        .map_err(|e| e.to_string())?;
    Ok(format!("ingested fact as edge {}", edge.raw()))
}

fn tool_search_memory(state: &McpState, args: &Value) -> Result<String, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or("missing 'query'")?;
    let retriever = MemoryRetriever::new(&state.store);
    let block = retriever.answer(query).map_err(|e| e.to_string())?;
    Ok(block.text)
}

fn tool_list_communities(state: &McpState) -> Result<String, String> {
    let comms = state
        .store
        .community_summaries()
        .map_err(|e| e.to_string())?;
    if comms.is_empty() {
        return Ok("no communities yet".to_string());
    }
    Ok(comms
        .into_iter()
        .map(|c| c.summary)
        .collect::<Vec<_>>()
        .join("\n"))
}

fn tool_resolve_entities(state: &McpState, args: &Value) -> Result<String, String> {
    let threshold = args
        .get("threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.9) as f32;
    let merged = state
        .store
        .auto_resolve(threshold)
        .map_err(|e| e.to_string())?;
    Ok(format!("merged {merged} entities"))
}

/// MCP tool results carry a `content` array of typed parts.
fn tool_text(text: &str) -> Value {
    json!({ "content": [{ "type": "text", "text": text }] })
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn err(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Run the stdio JSON-RPC loop (one JSON object per line).
pub fn run_stdio(state: &McpState) -> std::io::Result<()> {
    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(req) => handle_request(state, &req),
            Err(e) => err(Value::Null, -32700, &format!("parse error: {e}")),
        };
        writeln!(stdout, "{response}")?;
        stdout.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> McpState {
        McpState::new(Arc::new(FactStore::new()))
    }

    #[test]
    fn tools_list_advertises_both_tools() {
        let resp = handle_request(
            &state(),
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
        );
        let tools = &resp["result"]["tools"];
        let names: Vec<&str> = tools
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"add_memory"));
        assert!(names.contains(&"search_memory"));
        assert!(names.contains(&"list_communities"));
        assert!(names.contains(&"resolve_entities"));
    }

    #[test]
    fn add_then_search_via_tools() {
        let s = state();
        for (obj, vf, doc) in [("Beijing", 1000, 10), ("Shanghai", 2000, 20)] {
            let req = json!({
                "jsonrpc":"2.0","id":1,"method":"tools/call",
                "params": {"name":"add_memory","arguments":{
                    "subject":"Alice","predicate":"lives_in","object":obj,
                    "valid_from":vf,"doc":doc,"chunk":1}}
            });
            let resp = handle_request(&s, &req);
            assert!(resp.get("error").is_none(), "got {resp}");
        }

        let req = json!({
            "jsonrpc":"2.0","id":2,"method":"tools/call",
            "params": {"name":"search_memory","arguments":{
                "query":"WHERE SIMILAR(x, \"Alice\") RETURN CONTEXT(cite = true)"}}
        });
        let resp = handle_request(&s, &req);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Shanghai"), "got {text}");
    }

    #[test]
    fn list_communities_via_tools() {
        let s = state();
        let add = json!({
            "jsonrpc":"2.0","id":1,"method":"tools/call",
            "params": {"name":"add_memory","arguments":{
                "subject":"Alice","predicate":"lives_in","object":"Beijing",
                "valid_from":1000,"doc":10,"chunk":1}}
        });
        handle_request(&s, &add);

        let req = json!({
            "jsonrpc":"2.0","id":2,"method":"tools/call",
            "params": {"name":"list_communities","arguments":{}}
        });
        let resp = handle_request(&s, &req);
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("Alice") && text.contains("Beijing"),
            "got {text}"
        );
    }

    #[test]
    fn resolve_entities_via_tools() {
        let s = state();
        for (subj, doc) in [("OpenAI Inc.", 10), ("OpenAI", 20)] {
            handle_request(
                &s,
                &json!({
                    "jsonrpc":"2.0","id":1,"method":"tools/call",
                    "params": {"name":"add_memory","arguments":{
                        "subject":subj,"predicate":"based_in","object":"SF",
                        "valid_from":1000,"doc":doc,"chunk":1}}
                }),
            );
        }
        let resp = handle_request(
            &s,
            &json!({
                "jsonrpc":"2.0","id":2,"method":"tools/call",
                "params": {"name":"resolve_entities","arguments":{"threshold":0.9}}
            }),
        );
        let text = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("merged 1"), "got {text}");
    }

    #[test]
    fn unknown_method_errors() {
        let resp = handle_request(&state(), &json!({"jsonrpc":"2.0","id":1,"method":"nope"}));
        assert_eq!(resp["error"]["code"].as_i64(), Some(-32601));
    }
}
