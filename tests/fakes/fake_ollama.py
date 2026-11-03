"""A small HTTP server that speaks the parts of Ollama's API ChxChx uses.

It is real HTTP on a local port, so the adapter's `urllib` code, timeouts,
streaming and error handling run for real. State lives in memory and every
request is recorded for assertions.
"""

from __future__ import annotations

import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any

GIB = 1024**3


class _QuietServer(ThreadingHTTPServer):
    daemon_threads = True

    def handle_error(self, request: Any, client_address: Any) -> None:
        """A client that cancels mid-stream breaks the pipe; that is expected, not noise."""


class FakeOllama:
    def __init__(self) -> None:
        self.models: dict[str, int] = {}
        self.loaded: list[dict[str, Any]] = []
        self.requests: list[tuple[str, str, Any]] = []
        self.pull_error: str | None = None
        self.delete_status: int | None = None
        self.pull_size = 100
        self._server = _QuietServer(("127.0.0.1", 0), self._handler())
        self._thread = threading.Thread(target=self._server.serve_forever, daemon=True)

    @property
    def host(self) -> str:
        return f"127.0.0.1:{self._server.server_address[1]}"

    def start(self) -> "FakeOllama":
        self._thread.start()
        return self

    def stop(self) -> None:
        self._server.shutdown()
        self._server.server_close()

    def calls(self, method: str, path: str) -> list[Any]:
        return [body for verb, route, body in self.requests if (verb, route) == (method, path)]

    def _handler(self) -> type[BaseHTTPRequestHandler]:
        fake = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args: Any) -> None:  # keep test output quiet
                pass

            def _body(self) -> Any:
                length = int(self.headers.get("Content-Length") or 0)
                return json.loads(self.rfile.read(length)) if length else None

            def _send(self, status: int, payload: Any) -> None:
                data = json.dumps(payload).encode()
                self.send_response(status)
                self.send_header("Content-Type", "application/json")
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                self.wfile.write(data)

            def do_GET(self) -> None:
                fake.requests.append(("GET", self.path, None))
                if self.path == "/api/version":
                    self._send(200, {"version": "0.12.6"})
                elif self.path == "/api/tags":
                    self._send(200, {"models": [{"name": name, "size": size} for name, size in fake.models.items()]})
                elif self.path == "/api/ps":
                    self._send(200, {"models": fake.loaded})
                else:
                    self._send(404, {"error": "not found"})

            def do_POST(self) -> None:
                body = self._body()
                fake.requests.append(("POST", self.path, body))
                if self.path == "/api/create":
                    fake.models[body["model"]] = fake.models.get(body.get("from", ""), 1)
                    self._send(200, {"status": "success"})
                elif self.path == "/api/pull":
                    self._pull(body["model"])
                else:
                    self._send(404, {"error": "not found"})

            def _pull(self, model: str) -> None:
                self.send_response(200)
                self.send_header("Content-Type", "application/x-ndjson")
                self.end_headers()
                if fake.pull_error:
                    events: list[dict[str, Any]] = [{"status": "pulling manifest"}, {"error": fake.pull_error}]
                else:
                    total = fake.pull_size
                    events = [{"status": "pulling manifest"},
                              {"status": "pulling 3f1a", "digest": "sha256:3f1a", "total": total, "completed": total // 2},
                              {"status": "pulling 3f1a", "digest": "sha256:3f1a", "total": total, "completed": total},
                              {"status": "success"}]
                    fake.models[model] = total
                for event in events:
                    self.wfile.write((json.dumps(event) + "\n").encode())
                    self.wfile.flush()

            def do_DELETE(self) -> None:
                body = self._body()
                fake.requests.append(("DELETE", self.path, body))
                if fake.delete_status:
                    self._send(fake.delete_status, {"error": "boom"})
                elif body["model"] in fake.models:
                    del fake.models[body["model"]]
                    self._send(200, {})
                else:
                    self._send(404, {"error": "model not found"})

        return Handler
