# Chronos-Graph TypeScript SDK (planned)

Client SDK for Chronos-Graph. Planned delivery in milestone **M3** (see
[../../docs/implementation.md](../../docs/implementation.md)).

Intended surface:

- Connect to a Chronos-Graph server over gRPC-web / REST.
- `addMemory(episode)`, `searchMemory(query, budget, asOf)` returning LLM-ready, cited context.
- First-class MCP client helpers for agent integrations.
