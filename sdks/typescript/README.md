# Chronos-Graph TypeScript SDK

Dependency-free REST client (uses global `fetch`, Node 18+ / Deno / browser).

## Usage

Start the server:

```bash
cargo run -p chronos-server   # listens on 127.0.0.1:8080
```

Then:

```ts
import { ChronosClient } from "./client";

const client = new ChronosClient("http://127.0.0.1:8080");
await client.addMemory({ subject: "Alice", predicate: "lives_in", object: "Shanghai", validFrom: 2000, doc: 20, chunk: 1 });

const result = await client.search(
  'WHERE SIMILAR(x, "Alice lives") RETURN CONTEXT(cite = true)',
);
console.log(result.text);       // "- Alice lives_in Shanghai"
console.log(result.citations);  // [{ doc: 20, chunk: 1, snippet: ... }]
```

A gRPC-web client is planned once the gRPC surface lands.
