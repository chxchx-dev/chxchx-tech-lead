"""Live approvals and questions with Codex, through `codex app-server` (JSON-RPC over stdio).

`codex exec` never asks; the app-server does. One process serves one chat turn: initialize, start or
resume the thread, start the turn, then stream. Server requests (command and file approvals, extra
permissions, `request_user_input`) become the same `permission` / `question` events as Claude's live
mode (see `agent_live`), and Studio's answers go back as JSON-RPC responses. Everything else is mapped
to the `chxchx.agent-event` v1 shapes of `agent_stream`.

The protocol is marked experimental by Codex; the shapes used here were checked against codex-cli 0.166.0
(`codex app-server generate-json-schema`).
"""

from __future__ import annotations

import json
import threading
from collections.abc import Sequence
from pathlib import Path
from typing import IO, Any

from .agent_access import Access, codex_server_settings
from .agent_permissions import codex_block

Event = dict[str, Any]

_MAX_OUTPUT_CHARS = 4000
_DENIED_QUESTION = {"answers": {}}


def server_command(command: Sequence[str]) -> list[str]:
    """`codex app-server`, keeping only the `-c key=value` overrides of the agent's configured command."""
    argv = [command[0], "app-server"]
    configured = list(command[1:])
    for index, argument in enumerate(configured[:-1]):
        if argument in {"-c", "--config"}:
            argv += [argument, configured[index + 1]]
    return argv


def configured_model(command: Sequence[str]) -> str | None:
    configured = list(command[1:])
    for index, argument in enumerate(configured[:-1]):
        if argument in {"-m", "--model"}:
            return configured[index + 1]
    return None


class LiveCodex:
    """Drives one turn. `handle` consumes every record; `answer` may be called from another thread."""

    def __init__(self, stdin: IO[str], *, cwd: Path, access: Access, session_id: str | None, model: str | None,
                 effort: str | None, images: Sequence[Path] = ()) -> None:
        self._stdin = stdin
        self._lock = threading.Lock()
        self._cwd, self._access, self._session_id = str(cwd), access, session_id
        self._model, self._effort, self._images = model, effort, [str(path) for path in images]
        self._next_id = 0
        self._roles: dict[int, str] = {}
        self._pending: dict[str, tuple[Any, str, dict[str, Any]]] = {}  # request_id -> (rpc id, method, params)
        self._prompt = ""
        self._thread = ""
        self._baseline: dict[str, int] | None = None
        self._latest: dict[str, Any] = {}
        self.done = False

    # -- driver interface -------------------------------------------------------------------------------------------------
    def start(self, prompt: str) -> None:
        self._prompt = prompt
        self._request("initialize", {"clientInfo": {"name": "chxchx-studio", "title": "ChxChx Studio", "version": "1"}}, "init")

    def handle(self, record: dict[str, Any]) -> list[Event]:
        if "method" in record and "id" in record:
            return self._server_request(record)
        if "method" in record:
            return self._notification(str(record["method"]), record.get("params") if isinstance(record.get("params"), dict) else {})
        return self._response(record)

    def answer(self, message: dict[str, Any]) -> None:
        pending = self._pending.pop(str(message.get("request_id") or ""), None)
        if pending is None:
            return
        rpc_id, method, params = pending
        allow = message.get("behavior") == "allow"
        scope = "session" if message.get("scope") == "session" else "once"
        if "answers" in message:
            result: dict[str, Any] = self._question_result(params, message["answers"])
        elif method == "item/permissions/requestApproval":
            result = {"permissions": params.get("permissions") if allow else {}, "scope": "session" if scope == "session" else "turn"}
        elif allow:
            result = {"decision": "acceptForSession" if scope == "session" else "accept"}
        else:
            result = {"decision": "decline"}
        self._send({"id": rpc_id, "result": result})

    def deny_all(self) -> None:
        for request_id in list(self._pending):
            self.answer({"request_id": request_id, "behavior": "deny"})

    def finish(self) -> None:
        try:
            self._stdin.close()
        except OSError:
            pass

    # -- plumbing ---------------------------------------------------------------------------------------------------------------
    def _send(self, payload: dict[str, Any]) -> None:
        with self._lock:
            try:
                self._stdin.write(json.dumps(payload, ensure_ascii=False) + "\n")
                self._stdin.flush()
            except (OSError, ValueError):
                pass

    def _request(self, method: str, params: dict[str, Any], role: str) -> None:
        self._next_id += 1
        self._roles[self._next_id] = role
        self._send({"id": self._next_id, "method": method, "params": params})

    def _response(self, record: dict[str, Any]) -> list[Event]:
        role = self._roles.pop(record.get("id"), "")  # type: ignore[arg-type]
        if "error" in record:
            error = record["error"] if isinstance(record["error"], dict) else {}
            self.done = True
            return [{"type": "error", "message": str(error.get("message") or "Codex rechazó la solicitud")}]
        result = record.get("result") if isinstance(record.get("result"), dict) else {}
        settings = codex_server_settings(self._access)
        if role == "init":
            self._send({"method": "initialized"})
            params: dict[str, Any] = {"cwd": self._cwd, "sandbox": settings["sandbox"], "approvalPolicy": settings["approvalPolicy"]}
            if self._model:
                params["model"] = self._model
            if self._session_id:
                self._request("thread/resume", {"threadId": self._session_id, **params}, "thread")
            else:
                self._request("thread/start", {**params, "ephemeral": False}, "thread")
        elif role == "thread":
            thread = result.get("thread") if isinstance(result.get("thread"), dict) else {}
            self._thread = str(thread.get("id") or self._session_id or "")
            items: list[dict[str, Any]] = [{"type": "text", "text": self._prompt, "text_elements": []}]
            items += [{"type": "localImage", "path": path} for path in self._images]
            turn: dict[str, Any] = {"threadId": self._thread, "input": items, "approvalPolicy": settings["approvalPolicy"],
                                    "sandboxPolicy": settings["sandboxPolicy"]}
            if self._model:
                turn["model"] = self._model
            if self._effort:
                turn["effort"] = self._effort
            self._request("turn/start", turn, "turn")
            return [{"type": "session", "session_id": self._thread}]
        return []

    # -- notifications -> chxchx events ---------------------------------------------------------------------------------------
    def _notification(self, method: str, params: dict[str, Any]) -> list[Event]:
        if method in {"item/started", "item/completed"} and isinstance(params.get("item"), dict):
            return _item_events(params["item"], completed=method == "item/completed")
        if method == "thread/tokenUsage/updated" and isinstance(params.get("tokenUsage"), dict):
            self._track_usage(params["tokenUsage"])
        elif method == "error":
            error = params.get("error") if isinstance(params.get("error"), dict) else params
            if not params.get("willRetry"):
                return [{"type": "error", "message": str(error.get("message") or "Codex terminó el turno con error")}]
        elif method == "turn/completed":
            return self._turn_completed(params.get("turn") if isinstance(params.get("turn"), dict) else {})
        return []

    def _track_usage(self, usage: dict[str, Any]) -> None:
        total = usage.get("total") if isinstance(usage.get("total"), dict) else {}
        last = usage.get("last") if isinstance(usage.get("last"), dict) else {}
        if self._baseline is None:  # what the thread had spent before this turn's first model call
            self._baseline = {key: _count(total.get(key)) - _count(last.get(key)) for key in ("inputTokens", "outputTokens")}
        self._latest = {"total": total, "last": last, "window": usage.get("modelContextWindow")}

    def _turn_completed(self, turn: dict[str, Any]) -> list[Event]:
        self.done = True
        events: list[Event] = []
        total, last, base = self._latest.get("total", {}), self._latest.get("last", {}), self._baseline or {}
        if total:
            events.append({"type": "usage", "input_tokens": max(0, _count(total.get("inputTokens")) - base.get("inputTokens", 0)),
                           "output_tokens": max(0, _count(total.get("outputTokens")) - base.get("outputTokens", 0)), "cost_usd": None})
            window = self._latest.get("window")
            events.append({"type": "limits", "context_tokens": _count(last.get("inputTokens")) or None,
                           "context_window": window if isinstance(window, int) else None,
                           "primary": None, "secondary": None, "plan": None})
        if turn.get("status") == "failed":
            error = turn.get("error") if isinstance(turn.get("error"), dict) else {}
            events.append({"type": "error", "message": str(error.get("message") or "Codex terminó el turno con error")})
        return events

    # -- server requests -> permission / question events ------------------------------------------------------------------
    def _server_request(self, record: dict[str, Any]) -> list[Event]:
        method = str(record["method"])
        params = record.get("params") if isinstance(record.get("params"), dict) else {}
        request_id = f"codex-{record['id']}"
        if method == "item/tool/requestUserInput":
            self._pending[request_id] = (record["id"], method, params)
            return [_question_event(request_id, params)]
        if method in {"item/commandExecution/requestApproval", "item/fileChange/requestApproval", "item/permissions/requestApproval"}:
            self._pending[request_id] = (record["id"], method, params)
            return [_permission_event(request_id, method, params)]
        if method == "mcpServer/elicitation/request":
            self._send({"id": record["id"], "result": {"action": "decline"}})
        else:
            self._send({"id": record["id"], "error": {"code": -32601, "message": "No soportado por ChxChx Studio"}})
        return []

    @staticmethod
    def _question_result(params: dict[str, Any], answers: Any) -> dict[str, Any]:
        if not isinstance(answers, dict) or not answers:
            return dict(_DENIED_QUESTION)
        by_text = {str(q.get("question")): str(q.get("id")) for q in params.get("questions", []) if isinstance(q, dict)}
        return {"answers": {by_text[text]: {"answers": [str(value)]} for text, value in answers.items() if text in by_text}}


def _count(value: Any) -> int:
    return value if isinstance(value, int) and not isinstance(value, bool) else 0


def _strip(command: str) -> str:
    from .agent_stream import _strip_shell

    return _strip_shell(command)


def _tool(identifier: str, kind: str, title: str, status: str, output: str = "") -> Event:
    return {"type": "tool", "id": identifier, "kind": kind, "title": title, "status": status,
            "output": output if len(output) <= _MAX_OUTPUT_CHARS else output[:_MAX_OUTPUT_CHARS] + "\n…"}


def _status(raw: Any) -> str:
    return {"completed": "completed", "failed": "failed", "declined": "failed"}.get(str(raw), "running")


def _item_events(item: dict[str, Any], *, completed: bool) -> list[Event]:
    kind, identifier = item.get("type"), str(item.get("id") or "")
    if kind == "agentMessage":
        return [{"type": "message", "role": "assistant", "text": str(item["text"])}] if completed and item.get("text") else []
    if kind == "reasoning":
        parts = [str(part) for part in (item.get("summary") or item.get("content") or []) if part]
        return [{"type": "reasoning", "text": "\n".join(parts)}] if completed and parts else []
    if kind == "commandExecution":
        status = _status(item.get("status"))
        if status == "completed" and item.get("exitCode") not in (0, None):
            status = "failed"
        output = str(item.get("aggregatedOutput") or "")
        events = [_tool(identifier, "command", _strip(str(item.get("command") or "")), status, output)]
        if completed:
            events += codex_block({"type": "command_execution", "id": identifier, "command": _strip(str(item.get("command") or "")),
                                   "aggregated_output": output})
        return events
    if kind == "fileChange":
        changes = [c for c in item.get("changes", []) if isinstance(c, dict)]
        title = ", ".join(f"{(c.get('kind') or {}).get('type', 'update')} {c.get('path', '')}" for c in changes) or "Cambios de archivos"
        return [_tool(identifier, "file_change", title, _status(item.get("status")))]
    if kind == "mcpToolCall":
        return [_tool(identifier, "mcp", f"{item.get('server', 'mcp')}.{item.get('tool', '')}", _status(item.get("status")))]
    if kind == "webSearch":
        return [_tool(identifier, "search", str(item.get("query") or "Búsqueda web"), "completed" if completed else "running")]
    return []


def _choices(*, session_label: str) -> list[dict[str, Any]]:
    return [
        {"label": "Permitir esta vez", "allow": [], "network": False, "mode": None, "persist": False},
        {"label": session_label, "allow": [], "network": False, "mode": None, "persist": False, "scope": "session"},
    ]


def _permission_event(request_id: str, method: str, params: dict[str, Any]) -> Event:
    reason = str(params.get("reason") or "")
    if method == "item/commandExecution/requestApproval":
        command = _strip(str(params.get("command") or "")) or "un comando"
        summary, tool = f"Ejecutar un comando: {command}", "Bash"
        label = "Permitir mientras dure este turno"
    elif method == "item/fileChange/requestApproval":
        root = str(params.get("grantRoot") or "")
        summary, tool = "Modificar archivos" + (f" en {root}" if root else ""), "Edit"
        label = "Permitir los cambios de este turno"
    else:
        perms = params.get("permissions") if isinstance(params.get("permissions"), dict) else {}
        wanted = []
        if isinstance(perms.get("network"), dict) and perms["network"].get("enabled"):
            wanted.append("acceso a la red")
        if perms.get("fileSystem"):
            wanted.append("acceso a archivos fuera del proyecto")
        summary, tool = "Pide permisos extra: " + (", ".join(wanted) or "ampliar el sandbox"), "Permissions"
        label = "Permitir mientras dure este turno"
    return {"type": "permission", "id": str(params.get("itemId") or request_id), "request_id": request_id, "live": True,
            "tool": tool, "summary": summary, "detail": reason[:300], "certain": True, "choices": _choices(session_label=label)}


def _question_event(request_id: str, params: dict[str, Any]) -> Event:
    questions = []
    for item in params.get("questions", []):
        if not isinstance(item, dict) or not item.get("question"):
            continue
        options = [{"label": str(o.get("label", "")), "description": str(o.get("description", ""))}
                   for o in (item.get("options") or []) if isinstance(o, dict) and o.get("label")]
        questions.append({"question": str(item["question"]), "header": str(item.get("header", "")), "multiSelect": False,
                          "options": options})
    return {"type": "question", "request_id": request_id, "id": str(params.get("itemId") or request_id), "questions": questions}
