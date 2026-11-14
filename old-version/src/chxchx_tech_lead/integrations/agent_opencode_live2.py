"""Live approvals and questions with OpenCode 2.x, through `opencode serve` (HTTP `/api` + server-sent events).

Same driver interface and Studio events as `agent_opencode_live` (OpenCode 1.x), on the 2.x API (spec: `GET /openapi.json`
of a running server). One server serves one turn, on a random local port with a per-turn password (basic auth).

  POST /api/session                                    create (agent, model with variant, location)
  POST /api/session/{id}/prompt  {text, files}         send the message
  GET  /api/event                                      server-sent events of the whole server
  POST /api/session/{id}/permission/{rid}/reply        {decision: once | always | reject}
  POST /api/session/{id}/form/{fid}/reply {answer}     a question of the agent (a form); DELETE skips it

Access levels travel as `OPENCODE_CONFIG_CONTENT` (see `agent_opencode_turn`), read when the server starts.
Shapes checked against opencode 2.2.6.
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

from .agent_opencode_events import text_of, usage_events
from .agent_stream_opencode import _kind, _title

Event = dict[str, Any]

_LISTENING = re.compile(r"listening on (http://127\.0\.0\.1:\d+)")
_STARTUP_SECONDS = 40
_LOCAL = urllib.request.build_opener(urllib.request.ProxyHandler({}))  # a local server never goes through a proxy
_EVENT_STREAM_TIMEOUT = 6 * 3600
_STALL_SECONDS = 600
_MAX_OUTPUT_CHARS = 4000
# 2.x action -> (name Studio uses in `--allow`, what it is called in the card, title of the card)
_ACTIONS = {"shell": ("bash", "comandos", "Ejecutar un comando"), "edit": ("edit", "ediciones", "Editar archivos"),
            "webfetch": ("webfetch", "la web", "Leer una página web"), "websearch": ("webfetch", "la web", "Buscar en la web"),
            "external_directory": ("external_directory", "carpetas fuera del proyecto", "Acceder fuera del proyecto")}


class LiveOpenCodeV2:
    def __init__(self, process: Any, *, password: str, cwd: Path, session_id: str | None, agent: str, model: str | None,
                 effort: str | None, images: list[Path] | None = None) -> None:
        self._process = process
        self._auth = "Basic " + base64.b64encode(f"opencode:{password}".encode()).decode()
        self._cwd, self._session = cwd, session_id or ""
        self._agent, self._model, self._effort = agent, model, effort
        self._images = list(images or [])
        self._prompt, self._base = "", ""
        self._events: queue.Queue[dict[str, Any]] = queue.Queue()
        self._subscribed = threading.Event()
        self._tools: dict[str, str] = {}   # tool call id -> name
        self._inputs: dict[str, Any] = {}  # tool call id -> input
        self._pending: dict[str, tuple[str, str]] = {}  # request id -> (kind, session id)
        self._forms: dict[str, list[dict[str, Any]]] = {}  # form id -> fields in order
        self.model_seen: str | None = None
        self._denied = False  # a refusal interrupts the step: that is the answer, not an error
        self.done = False
        self.stopped_by_us = False

    # -- driver interface -------------------------------------------------------------------------------------------------
    def start(self, prompt: str) -> None:
        self._prompt = prompt
        try:
            self._process.stdin.close()
        except (OSError, AttributeError):
            pass

    def records(self, stdout: IO[str]) -> Iterator[dict[str, Any]]:
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
        except (urllib.error.URLError, OSError, ValueError, KeyError) as exc:
            yield {"type": "chxchx.error", "message": f"No pude hablar con el servidor de OpenCode: {exc}"}
            return
        yield {"type": "chxchx.session", "session_id": self._session}
        threading.Thread(target=self._listen, daemon=True).start()
        if not self._subscribed.wait(_STARTUP_SECONDS):
            yield {"type": "chxchx.error", "message": "No pude suscribirme a los eventos del servidor de OpenCode."}
            return
        try:
            self._request("POST", f"/api/session/{self._session}/prompt", self._body())
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
                    return
                if self._pending:
                    continue  # waiting for the person is not a stall
                idle += 1
                if idle >= _STALL_SECONDS:
                    yield {"type": "chxchx.error", "message": "OpenCode dejó de responder."}
                    return

    def handle(self, record: dict[str, Any]) -> list[Event]:
        kind = str(record.get("type"))
        data = record.get("data") if isinstance(record.get("data"), dict) else {}
        if kind == "chxchx.session":
            return [{"type": "session", "session_id": record["session_id"]}]
        if kind == "chxchx.error":
            self.done = True
            return [{"type": "error", "message": record["message"]}]
        if kind == "permission.asked":
            return self._permission(data)
        if kind == "form.created":
            return self._form(data.get("form") if isinstance(data.get("form"), dict) else {})
        if kind == "session.step.started":
            model = data.get("model") if isinstance(data.get("model"), dict) else {}
            if model.get("providerID") and model.get("id"):
                self.model_seen = f"{model['providerID']}/{model['id']}"
        elif kind == "session.reasoning.ended" and data.get("text"):
            return [{"type": "reasoning", "text": str(data["text"])}]
        elif kind == "session.text.ended" and data.get("text"):
            return [{"type": "message", "role": "assistant", "text": str(data["text"])}]
        elif kind == "session.tool.input.started":
            self._tools[str(data.get("id"))] = str(data.get("name") or "tool")
        elif kind == "session.tool.called":
            self._inputs[str(data.get("id"))] = data.get("input")
            return [self._tool(str(data.get("id")), "running", "")]
        elif kind == "session.tool.success":
            return [self._tool(str(data.get("id")), "completed", text_of(data.get("content")))]
        elif kind == "session.tool.failed":
            error = data.get("error") if isinstance(data.get("error"), dict) else {}
            return [self._tool(str(data.get("id")), "failed", str(error.get("message") or text_of(data.get("content"))))]
        elif kind == "session.step.ended":
            return usage_events(data)
        elif kind in {"session.execution.succeeded", "session.execution.interrupted"}:
            self.done = True
        elif kind in {"session.execution.failed", "session.step.failed"}:
            error = data.get("error") if isinstance(data.get("error"), dict) else {}
            self.done = self.done or kind == "session.execution.failed"
            if self._denied:
                return []
            return [{"type": "error", "message": str(error.get("message") or "OpenCode terminó el turno con error")}]
        return []

    def answer(self, message: dict[str, Any]) -> None:
        request_id = str(message.get("request_id") or "")
        pending = self._pending.pop(request_id, None)
        if pending is None:
            return
        what, session = pending
        try:
            if what == "permission":
                decision = "once" if message.get("behavior") == "allow" else "reject"
                self._denied = self._denied or decision == "reject"
                self._request("POST", f"/api/session/{session}/permission/{request_id}/reply", {"decision": decision})
            elif isinstance(message.get("answers"), dict) and message["answers"]:
                self._request("POST", f"/api/session/{session}/form/{request_id}/reply",
                              {"answer": self._form_answer(request_id, message["answers"])})
            else:
                self._request("DELETE", f"/api/session/{session}/form/{request_id}")
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
        with _LOCAL.open(request, timeout=30) as response:
            raw = response.read()
        return json.loads(raw) if raw else None

    def _open_session(self) -> None:
        provider, _, model = (self._model or "").partition("/")
        reference = {"id": model, "providerID": provider, **({"variant": self._effort} if self._effort else {})} if model else None
        if self._session:
            self._request("GET", f"/api/session/{self._session}")  # fails with 404 when the session is gone
            self._request("POST", f"/api/session/{self._session}/agent", {"agent": self._agent})
            if reference:
                self._request("POST", f"/api/session/{self._session}/model", {"model": reference})
            return
        body: dict[str, Any] = {"title": "ChxChx Studio", "agent": self._agent, "location": {"directory": str(self._cwd)}}
        if reference:
            body["model"] = reference
        self._session = str(self._request("POST", "/api/session", body)["data"]["id"])

    def _body(self) -> dict[str, Any]:
        files = [{"uri": image.resolve().as_uri(), "name": image.name,
                  "source": {"type": "uri", "uri": image.resolve().as_uri()},
                  "description": mimetypes.guess_type(image.name)[0] or "application/octet-stream"} for image in self._images]
        return {"text": self._prompt, **({"files": files} if files else {})}

    def _listen(self) -> None:
        request = urllib.request.Request(self._base + "/api/event", headers={"Authorization": self._auth})
        try:
            with _LOCAL.open(request, timeout=_EVENT_STREAM_TIMEOUT) as response:
                for raw in response:
                    line = raw.decode("utf-8", "replace").strip()
                    if not line.startswith("data:"):
                        continue
                    try:
                        event = json.loads(line[5:])
                    except json.JSONDecodeError:
                        continue
                    kind = str(event.get("type", ""))
                    if kind == "server.connected":
                        self._subscribed.set()
                    data = event.get("data") if isinstance(event.get("data"), dict) else {}
                    form = data.get("form") if isinstance(data.get("form"), dict) else {}
                    own = data.get("sessionID") == self._session or form.get("sessionID") == self._session
                    # Requests of child sessions (sub-agents) must be answered too.
                    if kind.startswith(("session.", "form.", "permission.")) and (own or kind in {"permission.asked", "form.created"}):
                        self._events.put(event)
        except (urllib.error.URLError, OSError, ValueError):
            if not self.done and self._process.poll() is None:
                self._events.put({"type": "chxchx.error", "message": "Se perdió la conexión con el servidor de OpenCode."})

    # -- events -> chxchx events ------------------------------------------------------------------------------------------------
    def _tool(self, identifier: str, status: str, output: str) -> Event:
        name = self._tools.get(identifier, "tool")
        state = {"input": self._inputs.get(identifier) if isinstance(self._inputs.get(identifier), dict) else {}}
        return {"type": "tool", "id": identifier, "kind": _kind(name), "title": _title(name, state), "status": status,
                "output": output[:_MAX_OUTPUT_CHARS]}

    def _permission(self, data: dict[str, Any]) -> list[Event]:
        request_id, session = str(data.get("id") or ""), str(data.get("sessionID") or self._session)
        self._pending[request_id] = ("permission", session)
        action = str(data.get("action") or "tool")
        name, label, title = _ACTIONS.get(action, (action, action, action))
        resources = [str(item) for item in data.get("resources", []) if item]
        message = str(data.get("message") or "")
        target = ", ".join(resources) or message
        choices: list[dict[str, Any]] = [{"label": "Permitir esta vez", "allow": [], "network": False, "mode": None, "persist": False}]
        for saved in [str(item) for item in data.get("save", []) if item][:1]:
            rule = name if saved == "*" else f"{name}:{saved}"
            choices.append({"label": f"Permitir «{saved}» en este chat" if saved != "*" else f"Permitir {label} en este chat",
                            "allow": [rule], "network": False, "mode": None, "persist": True})
        source = data.get("source") if isinstance(data.get("source"), dict) else {}
        return [{"type": "permission", "id": str(source.get("id") or request_id), "request_id": request_id, "live": True,
                 "tool": name, "summary": f"{title}: {target}"[:300], "detail": "", "certain": True, "choices": choices}]

    def _form(self, form: dict[str, Any]) -> list[Event]:
        request_id = str(form.get("id") or "")
        session = str(form.get("sessionID") or self._session)
        fields = [field for field in form.get("fields", []) if isinstance(field, dict) and field.get("key")]
        self._pending[request_id] = ("question", session)
        self._forms[request_id] = fields
        questions = []
        for field in fields:
            options = [{"label": str(o.get("label", o.get("value", ""))), "description": str(o.get("description", ""))}
                       for o in field.get("options", []) if isinstance(o, dict)]
            questions.append({"question": str(field.get("description") or field.get("title") or field["key"]),
                              "header": str(field.get("title") or ""), "multiSelect": field.get("type") == "multiselect",
                              "options": options})
        return [{"type": "question", "request_id": request_id, "id": request_id, "questions": questions}]

    def _form_answer(self, form_id: str, answers: dict[str, Any]) -> dict[str, Any]:
        """Studio answers by question text with the chosen labels joined by ", "; the form wants values by field key."""
        result: dict[str, Any] = {}
        for field in self._forms.pop(form_id, []):
            text = str(field.get("description") or field.get("title") or field["key"])
            chosen = [part for part in str(answers.get(text, "")).split(", ") if part]
            values = {str(o.get("label", o.get("value"))): o.get("value") for o in field.get("options", []) if isinstance(o, dict)}
            picked = [values.get(label, label) for label in chosen]  # free text (`custom`) passes through
            result[field["key"]] = picked if field.get("type") == "multiselect" else (picked[0] if picked else "")
        return result

