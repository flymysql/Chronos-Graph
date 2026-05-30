# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and this project adheres to
[Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-05-30

First public release. The bitemporal core, query layer, service layer and
secondary indexes are functional and tested (60+ tests across the workspace).

### Engine
- **Bitemporal storage** — every fact carries both *valid time* and
  *transaction time*; contradictions are *invalidated* (non-lossy), not deleted.
- **Atomic `UPSERT_FACT`** with contradiction detection + valid-time supersession.
- **Point-in-time `AS OF`** queries over both timelines.
- **MVCC in-memory `StorageEngine`** (snapshot isolation, read-your-writes,
  atomic commit) plus a durable **RocksDB** backend behind the `rocks` feature
  with full state recovery on open.
- **Okapi BM25** full-text index wired into `SIMILAR(...)`, and an exact
  brute-force cosine **vector index**.
- **Incremental connected-component communities** and **entity resolution**
  (lexical detection + transactional merge).
- **Multi-tenancy** — every fact carries a `TenantId`; the tenant filter is
  pushed into the scan.

### Interfaces
- **REST service** (`chronos-server`, axum/tokio): `POST /v1/memory`,
  `POST /v1/search`, `GET /v1/communities`, `POST /v1/resolve`.
- **MCP server** (`chronos-mcp`, JSON-RPC over stdio): `add_memory`,
  `search_memory`, `list_communities`, `resolve_entities`.
- **Embedded library** (`chronos-embedded`) for single-node / edge use.
- Dependency-free **Python** and **TypeScript** REST clients.

### Distribution
- Multi-stage **Dockerfile** (durable image, publishes to GHCR).
- **Release workflow** building prebuilt binaries (`chronos-server`,
  `chronos-server-rocks`, `chronos-mcp`) for Linux x86_64 and macOS
  (arm64 + x86_64), attached to the GitHub Release.
- Bilingual (EN / 中文) SDK & API documentation site with architecture diagrams.

### Planned
HNSW approximate vector index, `tantivy`-backed full-text, hierarchical (Leiden)
community roll-ups + LLM summaries, full historical MVCC on RocksDB, gRPC
service, and per-node ACL push-down.

[0.1.0]: https://github.com/flymysql/Chronos-Graph/releases/tag/v0.1.0
