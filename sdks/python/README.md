# Chronos-Graph Python SDK (planned)

Client SDK for Chronos-Graph. Planned delivery in milestone **M3** (see
[../../docs/implementation.md](../../docs/implementation.md)).

Intended surface:

- Connect to a Chronos-Graph server (gRPC) or open an embedded engine via `pyo3` bindings.
- `add_memory(episode)`, `search_memory(query, budget, as_of)` returning LLM-ready, cited context.
- Extended Cypher with `AS OF VALID TIME`, `SIMILAR(...)`, `TRAVERSE SEMANTIC(...)`, `RETURN CONTEXT(cite=true)`.
