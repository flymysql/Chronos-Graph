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

> Status: **M1–M6 functional (v0.0.1)**. Beyond the workspace skeleton, the
> bitemporal core, query layer, service layer, incremental communities, entity
> resolution, multi-tenancy, real BM25/vector indexes and a durable RocksDB
> backend are functional and tested (60+ tests across the workspace):
>
> - `chronos-storage`: a complete in-memory **MVCC** `StorageEngine` (snapshot
>   isolation, read-your-writes, atomic commit) plus a durable **RocksDB**
>   backend behind the `rocks` feature (WAL + crash recovery).
> - `chronos-embedded`: a transactional `FactStore` implementing the atomic
>   **`UPSERT_FACT`** operator (contradiction detection + valid-time
>   supersession, non-lossy) and **point-in-time `as_of`** queries over both
>   timelines, verified with property-based tests.
> - `chronos-index` (M5): real in-memory secondary indexes — **Okapi BM25**
>   (generic over key type, used to rank `SIMILAR(...)` over fact text) and an
>   exact **brute-force cosine vector index** (the reference an HNSW backend
>   will be validated against). The retriever now ranks with BM25 instead of
>   ad-hoc substring matching.
> - `chronos-query` (M2): a real lexer + recursive-descent parser for the
>   extended Cypher subset (`AS OF [VALID|TRANSACTION] TIME`, `SIMILAR(...)`,
>   `TRAVERSE SEMANTIC(...)`, `RETURN CONTEXT(cite=...)`), plus
>   `GreedyBudgeter` and `DefaultContextSerializer` operators.
> - `MemoryRetriever` (M2): wires `FactStore` into the query layer and provides
>   an end-to-end **"question -> cited, point-in-time context"** pipeline.
> - `chronos-server` (M3): a runnable **HTTP/REST** service (axum/tokio) with
>   `POST /v1/memory`, `POST /v1/search`, `GET /v1/communities` and
>   `POST /v1/resolve`, integration-tested in-process.
> - `chronos-mcp` (M3): a built-in **MCP server** (JSON-RPC over stdio) exposing
>   `add_memory` / `search_memory` / `list_communities` / `resolve_entities`
>   tools to agents.
> - `chronos-community` (M4): **incrementally maintained communities** — a
>   union-find unions each fact's endpoints in near-constant time, so a new fact
>   only touches the two affected components instead of forcing a full rebuild
>   (the cost advantage over batch GraphRAG). Level-0 (connected-component)
>   communities surface templated, current-fact summaries for global queries;
>   hierarchical Leiden roll-ups are future work.
> - `chronos-resolution` (M4): **entity resolution** — dependency-free lexical
>   blocking detects surface variants ("OpenAI" / "OpenAI Inc."), and the engine
>   merges them transactionally: facts are rewritten onto the canonical node
>   (preserving each fact's bitemporal span and provenance) and exact duplicates
>   are deduped. Exposed via `POST /v1/resolve` and the `resolve_entities` tool.
> - **Multi-tenancy / ACL push-down** (M4): every fact carries a `TenantId`, and
>   retrieval, community summaries and resolution push the tenant filter into
>   the scan — a tenant-scoped retriever can never observe another tenant's
>   facts, and contradiction detection never crosses the boundary. The HTTP
>   layer reads the tenant from the `X-Tenant-Id` header.
> - **Durable engine** (M6): `FactStore` is storage-engine-agnostic
>   (`Box<dyn StorageEngine>`). With the `rocks` feature, `FactStore::open_rocks`
>   / `Chronos::open` back the engine with RocksDB and **recover all state**
>   (facts, node/predicate names, tenant assignments, interval index,
>   provenance, per-tenant communities and the BM25 index) from disk on open;
>   tenant assignments are persisted atomically with their fact.
> - `sdks/`: dependency-free **Python** and **TypeScript** REST clients.
>
> ```cypher
> MATCH (n) WHERE SIMILAR(n, "Alice lives")
> AS OF VALID TIME 1500
> RETURN CONTEXT(cite = true)
> ```
>
> Run it:
>
> ```bash
> cargo run -p chronos-server          # REST on 127.0.0.1:8080
> cargo run -p chronos-mcp             # MCP server on stdio
> ```
>
> (gRPC is planned once `protoc` is wired into CI; the REST surface is the
> current external contract.)
>
> See [docs/implementation.md](docs/implementation.md) for the engineering plan
> and [docs/design.md](docs/design.md) for the architecture rationale.

## Documentation

SDK & API docs (REST, Python, TypeScript, MCP, embedded) are a static site in
[`site/`](site/), published to GitHub Pages at
`https://flymysql.github.io/Chronos-Graph/`. Preview locally with
`cd site && python3 -m http.server 8000`.

## Workspace layout

```text
crates/
  chronos-common       shared types: ids, time, bitemporal span, errors
  chronos-storage      storage engine: record codec, MVCC, txn, interval index
  chronos-graph-model  graph model: nodes/edges/properties, 3-tier subgraphs
  chronos-index        secondary indexes: in-memory BM25 + brute-force vector (HNSW: planned)
  chronos-temporal     bitemporal core: validity, invalidation, as-of
  chronos-provenance   triple<->chunk<->document links + source invalidation
  chronos-query        query language + planner + optimizer + executors
  chronos-community    incremental connected-component communities (Leiden roll-ups: planned)
  chronos-resolution   entity resolution: lexical candidate detection + engine merge
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

# Durable RocksDB backend (slower first build; pulls librocksdb-sys):
cargo test -p chronos-embedded --features rocks
CHRONOS_DATA_DIR=./data cargo run -p chronos-server --features rocks
```

## Implemented vs planned

| Capability | Status |
| --- | --- |
| Bitemporal storage (valid + transaction time) | ✅ implemented |
| MVCC in-memory engine | ✅ implemented |
| Durable RocksDB engine + full state recovery | ✅ implemented (`rocks`) |
| Atomic `UPSERT_FACT` + non-lossy invalidation | ✅ implemented |
| Point-in-time `AS OF` over both timelines | ✅ implemented |
| Extended-Cypher lexer / parser / compile | ✅ implemented |
| Token-budget selection + graph-to-text + citations | ✅ implemented |
| BM25 full-text index (wired into `SIMILAR`) | ✅ implemented |
| Brute-force cosine vector index | ✅ implemented |
| Incremental connected-component communities | ✅ implemented |
| Entity resolution (lexical detect + transactional merge) | ✅ implemented |
| Multi-tenancy / tenant filter push-down | ✅ implemented |
| HTTP/REST service + built-in MCP server + SDKs | ✅ implemented |
| HNSW approximate vector index | ⏳ planned |
| `tantivy`-backed full-text | ⏳ planned |
| Hierarchical (Leiden) community roll-ups + LLM summaries | ⏳ planned |
| Background re-embedding pipeline | ⏳ planned |
| Full historical MVCC on the RocksDB backend | ⏳ planned |
| gRPC service (once `protoc` is in CI) | ⏳ planned |
| Per-node ACL push-down (below the tenant boundary) | ⏳ planned |

## License

Apache-2.0.
