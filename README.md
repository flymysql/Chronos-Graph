# Chronos-Graph

A **bitemporal-native graph database** purpose-built for RAG and AI-agent memory.

Most GraphRAG / agent-memory stacks (e.g. Graphiti/Zep) implement temporal logic,
fact invalidation, hybrid retrieval and provenance **on top of a general-purpose
graph store** (Neo4j / FalkorDB / Kùzu) — at the framework layer. Chronos-Graph
pushes these capabilities **down into the database engine itself**:

- **Bitemporal by design** — every fact carries both *valid time* (when it was true
  in the real world) and *transaction time* (when the system learned it). Contradictory
  facts are *invalidated*, not deleted, enabling point-in-time queries.
- **Unified hybrid retrieval** — vector + BM25 + semantic graph traversal ranked by a
  single engine-level cost model, in one call.
- **Provenance as a first-class structure** — `triple ↔ chunk ↔ document` links, so every
  retrieved answer is citable and source invalidation can cascade.
- **Token-budget subgraph selection + graph-to-text** — retrieval returns LLM-ready
  context, not rows.
- **Agent-native** — built-in MCP server for write-memory / search / multi-hop.

> Status: **early scaffold (v0.0.1)**. This repository currently contains the compiling
> workspace skeleton (crate graph + core trait boundaries). See
> [docs/implementation.md](docs/implementation.md) for the engineering plan and
> [docs/design.md](docs/design.md) for the architecture rationale.

## Workspace layout

```text
crates/
  chronos-common       shared types: ids, time, bitemporal span, errors
  chronos-storage      storage engine: record codec, MVCC, txn, interval index
  chronos-graph-model  graph model: nodes/edges/properties, 3-tier subgraphs
  chronos-index        secondary indexes: vector (HNSW), full-text (BM25)
  chronos-temporal     bitemporal core: validity, invalidation, as-of
  chronos-provenance   triple<->chunk<->document links + source invalidation
  chronos-query        query language + planner + optimizer + executors
  chronos-community    [Phase 2] Leiden + incremental community views
  chronos-resolution   [Phase 2] embedding-based entity resolution
  chronos-server       gRPC/HTTP service, sessions, ACL, multi-tenancy
  chronos-mcp          built-in MCP server (agent tools)
  chronos-embedded     embedded library form factor (single-node / edge)
sdks/                  python / typescript client SDKs (planned)
```

## Build

```bash
cargo build
cargo test
cargo clippy --all-targets
cargo fmt --check
```

## License

Apache-2.0.
