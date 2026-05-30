"""Minimal Python client for the Chronos-Graph REST API.

Stdlib-only (urllib) so it has no dependencies. Targets the M3 endpoints:
``POST /v1/memory`` and ``POST /v1/search``.

Example
-------
>>> client = ChronosClient("http://127.0.0.1:8080")
>>> client.add_memory("Alice", "lives_in", "Beijing", valid_from=1000, doc=10, chunk=1)
>>> client.add_memory("Alice", "lives_in", "Shanghai", valid_from=2000, doc=20, chunk=1)
>>> result = client.search(
...     'MATCH (n) WHERE SIMILAR(n, "Alice lives") RETURN CONTEXT(cite = true)'
... )
>>> print(result["text"])       # "- Alice lives_in Shanghai"
>>> print(result["citations"])  # [{"doc": 20, "chunk": 1, "snippet": ...}]
"""

from __future__ import annotations

import json
import urllib.request
from typing import Any, Optional


class ChronosError(RuntimeError):
    """Raised when the server returns a non-2xx response."""


class ChronosClient:
    def __init__(self, base_url: str = "http://127.0.0.1:8080", timeout: float = 10.0):
        self.base_url = base_url.rstrip("/")
        self.timeout = timeout

    def _post(self, path: str, payload: dict[str, Any]) -> Any:
        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            f"{self.base_url}{path}",
            data=data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as resp:
                return json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as e:  # noqa: PERF203
            body = e.read().decode("utf-8", "replace")
            raise ChronosError(f"{e.code}: {body}") from e

    def add_memory(
        self,
        subject: str,
        predicate: str,
        object: str,
        valid_from: int,
        doc: int,
        chunk: int,
        policy: Optional[str] = None,
    ) -> int:
        """Ingest a fact. Returns the new edge id."""
        payload: dict[str, Any] = {
            "subject": subject,
            "predicate": predicate,
            "object": object,
            "valid_from": valid_from,
            "doc": doc,
            "chunk": chunk,
        }
        if policy is not None:
            payload["policy"] = policy
        return self._post("/v1/memory", payload)["edge_id"]

    def search(self, query: str) -> dict[str, Any]:
        """Run an extended-Cypher query; returns {"text", "citations"}."""
        return self._post("/v1/search", {"query": query})


if __name__ == "__main__":
    client = ChronosClient()
    client.add_memory("Alice", "lives_in", "Beijing", valid_from=1000, doc=10, chunk=1)
    client.add_memory("Alice", "lives_in", "Shanghai", valid_from=2000, doc=20, chunk=1)
    print(client.search('WHERE SIMILAR(x, "Alice lives") RETURN CONTEXT(cite = true)'))
