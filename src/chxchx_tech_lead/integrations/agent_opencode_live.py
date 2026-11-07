"""Live approvals and questions with OpenCode, through `opencode serve` (HTTP + server-sent events).

`opencode run` rejects whatever is set to `ask`. The server instead publishes `permission.asked` and
`question.asked` events and waits for `POST /permission/{id}/reply` / `POST /question/{id}/reply`. One
server serves one chat turn, on a random local port, protected by a per-turn password (basic auth), so no
other local process can drive it. Events become the same `permission` / `question` shapes as the other
live agents (see `agent_live`); message parts reuse the mapping of `agent_stream_opencode`.

Shapes checked against opencode 1.19.5 (`GET /doc` on a running server).
"""

from __future__ import annotations

import base64
import json
import mimetypes
import queue
import re
import threading
import urllib.error
import urllib.request
from collections.abc import Iterator
from pathlib import Path
from typing import IO, Any

from .agent_stream_opencode import _step_finish, _tool

Event = dict[str, Any]

_LISTENING = re.compile(r"listening on (http://127\.0\.0\.1:\d+)")
_STARTUP_SECONDS = 40
_EVENT_STREAM_TIMEOUT = 6 * 3600  # the event stream is quiet while the person decides; the server ending closes it
_STALL_SECONDS = 600  # nobody answers and nothing happens: give up instead of leaving the server running
_PERMISSION_LABELS = {"bash": "comandos", "edit": "ediciones", "webfetch": "la web", "external_directory": "carpetas fuera del proyecto"}


class LiveOpenCode:
    def __init__(self, process: Any, *, password: str, cwd: Path, session_id: str | None, agent: str, model: str | None,
                 effort: str | None, images: list[Path] | None = None) -> None:
        self._process = process
        self._auth = "Basic " + base64.b64encode(f"opencode:{password}".encode()).decode()
        self._session = session_id or ""
        self._agent, self._model, self._effort = agent, model, effort
        self._images = list(images or [])
        self._prompt = ""
        self._base = ""
        self._events: queue.Queue[dict[str, Any]] = queue.Queue()
        self._subscribed = threading.Event()
        self._roles: dict[str, str] = {}
        self._emitted: set[tuple[str, str]] = set()
        self._pending: dict[str, str] = {}  # request id -> "permission" | "question"
        self._questions: dict[str, list[str]] = {}
        self.done = False
        self.stopped_by_us = False

    # -- driver interface -------------------------------------------------------------------------------------------------
    def start(self, prompt: str) -> None:
        self._prompt = prompt
        try:
            self._process.stdin.close()  # the server takes no input from stdin
        except (OSError, AttributeError):
            pass

    def records(self, stdout: IO[str]) -> Iterator[dict[str, Any]]:
        """Waits for the server, opens the session and the event stream, sends the prompt and yields events."""
        ready = threading.Event()

        def read_stdout() -> None:
            for line in stdout:
                match = _LISTENING.search(line)
                if match and not self._base:
                    self._base = match.group(1)
                    ready.set()

        threading.Thread(target=read_stdout, daemon=True).start()
        if not ready.wait(_STARTUP_SECONDS) or not self._base:
            yield {"type": "chxchx.error", "message": "OpenCode no arrancó su servidor local a tiempo."}
            return
        try:
            self._open_session()
        except (urllib.error.URLError, OSError, ValueError) as exc:
            yield {"type": "chxchx.error", "message": f"No pude hablar con el servidor de OpenCode: {exc}"}
            return
        yield {"type": "chxchx.session", "session_id": self._session}
        threading.Thread(target=self._listen, daemon=True).start()
        # The server does not replay events: the prompt may only go once the subscription is registered.
        if not self._subscribed.wait(_STARTUP_SECONDS):
            yield {"type": "chxchx.error", "message": "No pude suscribirme a los eventos del servidor de OpenCode."}
            return
        try:
            self._post(f"/session/{self._session}/prompt_async", self._body())
        except (urllib.error.URLError, OSError, ValueError) as exc:
            yield {"type": "chxchx.error", "message": f"OpenCode no aceptó el mensaje: {exc}"}
            return
        idle = 0
        while not self.done:
            try:
                yield self._events.get(timeout=1)
                idle = 0
            except queue.Empty:
                if self._process.poll() is not None:
                    return  # cancelled or crashed: the caller reports a non-zero exit
                if self._pending:
                    continue  # waiting for the person is not a stall; closing the chat cancels the turn
                idle += 1
                if idle >= _STALL_SECONDS:
                    yield {"type": "chxchx.error", "message": "OpenCode dejó de responder."}
                    return

    def handle(self, record: dict[str, Any]) -> list[Event]:
        kind, props = str(record.get("type")), record.get("properties") if isinstance(record.get("properties"), dict) else {}
        if kind == "chxchx.session":
            return [{"type": "session", "session_id": record["session_id"]}]
        if kind == "chxchx.error":
            self.done = True
            return [{"type": "error", "message": record["message"]}]
        if kind == "permission.asked":
            return self._permission(props)
        if kind == "question.asked":
            return self._question(props)
        if kind in {"session.idle"} or (kind == "session.status" and (props.get("status") or {}).get("type") == "idle"):
            self.done = True
        elif kind == "session.error":
            self.done = True
            error = props.get("error") if isinstance(props.get("error"), dict) else {}
            data = error.get("data") if isinstance(error.get("data"), dict) else {}
            return [{"type": "error", "message": str(data.get("message") or error.get("name") or "OpenCode terminó el turno con error")}]
        elif kind == "message.updated":
            info = props.get("info") if isinstance(props.get("info"), dict) else {}
            self._roles[str(info.get("id"))] = str(info.get("role"))
        elif kind == "message.part.updated" and isinstance(props.get("part"), dict):
            return self._part(props["part"])
        return []

    def answer(self, message: dict[str, Any]) -> None:
        request_id = str(message.get("request_id") or "")
        what = self._pending.pop(request_id, None)
        if what is None:
            return
        try:
            if what == "permission":
                reply = "once" if message.get("behavior") == "allow" else "reject"
                self._post(f"/permission/{request_id}/reply", {"reply": reply})
            elif isinstance(message.get("answers"), dict) and message["answers"]:
                order = self._questions.pop(request_id, [])
                answers = [[part for part in str(message["answers"].get(text, "")).split(", ") if part] for text in order]
                self._post(f"/question/{request_id}/reply", {"answers": answers})
            else:
                self._post(f"/question/{request_id}/reject", {})
        except (urllib.error.URLError, OSError, ValueError):
            pass  # the server went away with the turn

    def deny_all(self) -> None:
        for request_id in list(self._pending):
            self.answer({"request_id": request_id, "behavior": "deny"})

    def finish(self) -> None:
        self.stopped_by_us = True
        try:
            self._process.terminate()
        except OSError:
            pass

    # -- plumbing ---------------------------------------------------------------------------------------------------------------
    def _request(self, method: str, path: str, body: Any = None) -> Any:
        data = json.dumps(body).encode() if body is not None else None
        request = urllib.request.Request(self._base + path, data=data, method=method,
                                         headers={"Authorization": self._auth, "Content-Type": "application/json"})
        with urllib.request.urlopen(request, timeout=30) as response:
            raw = response.read()
        return json.loads(raw) if raw else None

    def _post(self, path: str, body: Any) -> Any:
        return self._request("POST", path, body)

    def _open_session(self) -> None:
        if self._session:
            self._request("GET", f"/session/{self._session}")  # fails with 404 when the session is gone
        else:
            self._session = str(self._request("POST", "/session", {"title": "ChxChx Studio"})["id"])

    def _body(self) -> dict[str, Any]:
        parts: list[dict[str, Any]] = [{"type": "text", "text": self._prompt}]
        for image in self._images:
            mime = mimetypes.guess_type(image.name)[0] or "application/octet-stream"
            parts.append({"type": "file", "mime": mime, "filename": image.name, "url": image.resolve().as_uri()})
        body: dict[str, Any] = {"parts": parts, "agent": self._agent}
        if self._model and "/" in self._model:
            provider, _, model = self._model.partition("/")
            body["model"] = {"providerID": provider, "modelID": model}
        if self._effort:
            body["variant"] = self._effort
        return body

    def _listen(self) -> None:
        request = urllib.request.Request(self._base + "/event", headers={"Authorization": self._auth})
        try:
            with urllib.request.urlopen(request, timeout=_EVENT_STREAM_TIMEOUT) as response:
                for raw in response:
                    line = raw.decode("utf-8", "replace").strip()
                    if not line.startswith("data:"):
                        continue
                    try:
                        event = json.loads(line[5:])
                    except json.JSONDecodeError:
                        continue
                    props = event.get("properties") if isinstance(event.get("properties"), dict) else {}
                    if event.get("type") == "server.connected":
                        self._subscribed.set()
                    if str(event.get("type", "")).startswith(("plugin.", "server.", "tui.", "message.part.delta")):
                        continue
                    # Permissions and questions of child sessions (sub-agents) must be answered too.
                    if str(event.get("type", "")).endswith(".asked") or props.get("sessionID") in (self._session, None):
                        self._events.put(event)
        except (urllib.error.URLError, OSError, ValueError):
            if not self.done and self._process.poll() is None:
                self._events.put({"type": "chxchx.error", "message": "Se perdió la conexión con el servidor de OpenCode."})

    # -- events -> chxchx events ------------------------------------------------------------------------------------------------
    def _once(self, kind: str, identifier: str) -> bool:
        key = (kind, identifier)
        if key in self._emitted:
            return False
        self._emitted.add(key)
        return True

    def _part(self, part: dict[str, Any]) -> list[Event]:
        kind, identifier = part.get("type"), str(part.get("id") or "")
        if kind == "text" and part.get("time", {}).get("end") and self._roles.get(str(part.get("messageID")), "assistant") == "assistant":
            return [{"type": "message", "role": "assistant", "text": str(part["text"])}] if part.get("text") and self._once("text", identifier) else []
        if kind == "reasoning" and part.get("time", {}).get("end") and part.get("text"):
            return [{"type": "reasoning", "text": str(part["text"])}] if self._once("reasoning", identifier) else []
        if kind == "tool":
            status = str((part.get("state") or {}).get("status"))
            if status != "pending" and self._once("tool:" + str(part.get("callID") or identifier), status):
                return [_tool(part)]
        if kind == "step-finish" and self._once("step", identifier):
            return _step_finish(part)
        return []

    def _permission(self, props: dict[str, Any]) -> list[Event]:
        request_id = str(props.get("id") or "")
        self._pending[request_id] = "permission"
        name = str(props.get("permission") or "tool")
        patterns = [str(p) for p in props.get("patterns", []) if p]
        metadata = props.get("metadata") if isinstance(props.get("metadata"), dict) else {}
        target = str(metadata.get("command") or metadata.get("filepath") or metadata.get("url") or ", ".join(patterns))
        titles = {"bash": "Ejecutar un comando", "edit": "Editar archivos", "webfetch": "Leer una página web",
                  "external_directory": "Acceder fuera del proyecto"}
        choices: list[dict[str, Any]] = [{"label": "Permitir esta vez", "allow": [], "network": False, "mode": None, "persist": False}]
        for always in [str(p) for p in props.get("always", []) if p][:1]:
            rule = name if always == "*" else f"{name}:{always}"
            label = _PERMISSION_LABELS.get(name, name)
            choices.append({"label": f"Permitir «{always}» en este chat" if always != "*" else f"Permitir {label} en este chat",
                            "allow": [rule], "network": False, "mode": None, "persist": True})
        tool = props.get("tool") if isinstance(props.get("tool"), dict) else {}
        return [{"type": "permission", "id": str(tool.get("callID") or request_id), "request_id": request_id, "live": True,
                 "tool": name, "summary": f"{titles.get(name, name)}: {target}"[:300], "detail": "", "certain": True, "choices": choices}]

    def _question(self, props: dict[str, Any]) -> list[Event]:
        request_id = str(props.get("id") or "")
        self._pending[request_id] = "question"
        questions = []
        for item in props.get("questions", []):
            if not isinstance(item, dict) or not item.get("question"):
                continue
            options = [{"label": str(o.get("label", "")), "description": str(o.get("description", ""))}
                       for o in item.get("options", []) if isinstance(o, dict) and o.get("label")]
            questions.append({"question": str(item["question"]), "header": str(item.get("header", "")),
                              "multiSelect": bool(item.get("multiple")), "options": options})
        self._questions[request_id] = [q["question"] for q in questions]
        return [{"type": "question", "request_id": request_id, "id": request_id, "questions": questions}]
