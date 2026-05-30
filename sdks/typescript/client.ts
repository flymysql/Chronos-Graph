/**
 * Minimal TypeScript client for the Chronos-Graph REST API (M3).
 *
 * Uses the global `fetch` (Node 18+ / browsers / Deno), no dependencies.
 */

export interface Citation {
  doc: number;
  chunk: number;
  snippet: string | null;
}

export interface SearchResult {
  text: string;
  citations: Citation[];
}

export type ConflictPolicy = "unique" | "append";

export class ChronosError extends Error {}

export class ChronosClient {
  constructor(private readonly baseUrl: string = "http://127.0.0.1:8080") {
    this.baseUrl = baseUrl.replace(/\/$/, "");
  }

  private async post<T>(path: string, body: unknown): Promise<T> {
    const resp = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    if (!resp.ok) {
      throw new ChronosError(`${resp.status}: ${await resp.text()}`);
    }
    return (await resp.json()) as T;
  }

  /** Ingest a fact; resolves to the new edge id. */
  async addMemory(args: {
    subject: string;
    predicate: string;
    object: string;
    validFrom: number;
    doc: number;
    chunk: number;
    policy?: ConflictPolicy;
  }): Promise<number> {
    const { edge_id } = await this.post<{ edge_id: number }>("/v1/memory", {
      subject: args.subject,
      predicate: args.predicate,
      object: args.object,
      valid_from: args.validFrom,
      doc: args.doc,
      chunk: args.chunk,
      policy: args.policy,
    });
    return edge_id;
  }

  /** Run an extended-Cypher query and return cited context. */
  async search(query: string): Promise<SearchResult> {
    return this.post<SearchResult>("/v1/search", { query });
  }
}

// Example (run with: `node --loader ts-node/esm client.ts` or compile first):
async function main() {
  const client = new ChronosClient();
  await client.addMemory({ subject: "Alice", predicate: "lives_in", object: "Beijing", validFrom: 1000, doc: 10, chunk: 1 });
  await client.addMemory({ subject: "Alice", predicate: "lives_in", object: "Shanghai", validFrom: 2000, doc: 20, chunk: 1 });
  const result = await client.search('WHERE SIMILAR(x, "Alice lives") RETURN CONTEXT(cite = true)');
  console.log(result.text);
  console.log(result.citations);
}

// Only run when executed directly, not when imported.
if (typeof require !== "undefined" && require.main === module) {
  main().catch((e) => {
    console.error(e);
    process.exit(1);
  });
}
