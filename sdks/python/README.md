# Chronos-Graph Python SDK

Stdlib-only (no dependencies) REST client for the Chronos-Graph M3 API.

## Usage

Start the server:

```bash
cargo run -p chronos-server   # listens on 127.0.0.1:8080
```

Then:

```python
from chronos_graph import ChronosClient

client = ChronosClient("http://127.0.0.1:8080")
client.add_memory("Alice", "lives_in", "Beijing", valid_from=1000, doc=10, chunk=1)
client.add_memory("Alice", "lives_in", "Shanghai", valid_from=2000, doc=20, chunk=1)

# Current view -> Shanghai, with a citation.
print(client.search('WHERE SIMILAR(x, "Alice lives") RETURN CONTEXT(cite = true)'))

# Point-in-time view -> Beijing.
print(client.search('WHERE SIMILAR(x, "Alice") AS OF VALID TIME 1500 RETURN CONTEXT(cite = true)'))
```

A native (pyo3) embedded binding is planned so the engine can run in-process.
