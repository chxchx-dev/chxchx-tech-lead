"""Live approvals and questions for Claude Code.

Normally a visual-chat turn cannot be interrupted for an answer. With `--input-format stream-json
--permission-prompt-tool stdio` Claude asks the host instead: it prints a `control_request` and waits
for a `control_response` on stdin. This module turns those requests into the same `permission` events
as the between-messages cards (now with `live` and a `request_id`) plus `question` events for
`AskUserQuestion`, and writes the person's answer back.

  permission: {"type": "permission", "live": true, "request_id": str, ...}  (see agent_permissions)
  question:   {"type": "question", "request_id": str, "id": str,
               "questions": [{"question", "header", "multiSelect", "options": [{"label", "description"}]}]}

Studio answers with a JSON line on `agent chat`'s stdin:
  {"type": "answer", "request_id": str, "behavior": "allow" | "deny"}                  (permission)
  {"type": "answer", "request_id": str, "answers": {"<question text>": "<label>"}}    (question; none = skipped)
"""

from __future__ import annotations

import json
import threading
from typing import IO, Any

from .agent_permissions import claude_live_permission

Event = dict[str, Any]

_DENIED = "El usuario rechazó esta acción desde ChxChx Studio. No la reintentes; sigue sin ella o pregunta qué prefiere."
_UNANSWERED = "El usuario no respondió la pregunta. Continúa con tu mejor criterio o explica qué necesitas."


def user_message(prompt: str) -> str:
    """The turn's prompt as the JSON line `--input-format stream-json` expects."""
    return json.dumps({"type": "user", "message": {"role": "user", "content": prompt}}, ensure_ascii=False) + "\n"


class LiveClaude:
    """Answers Claude's `control_request`s while a turn runs. Safe to call `answer` from another thread."""

    def __init__(self, stdin: IO[str]) -> None:
        self._stdin = stdin
        self._lock = threading.Lock()
        self._pending: dict[str, dict[str, Any]] = {}

    def handle(self, record: dict[str, Any]) -> list[Event] | None:
        """Events for a control record, or None when the record is not one of ours."""
        kind = record.get("type")
        if kind == "control_cancel_request":
            self._pending.pop(str(record.get("request_id")), None)
            return []
        if kind != "control_request":
            return None
        request = record.get("request") if isinstance(record.get("request"), dict) else {}
        request_id = str(record.get("request_id") or "")
        if request.get("subtype") != "can_use_tool" or not request_id:
            self._respond(request_id, {"subtype": "error", "error": "Solicitud no soportada"}, ok=False)
            return []
        name = str(request.get("tool_name") or "")
        data = request.get("input") if isinstance(request.get("input"), dict) else {}
        self._pending[request_id] = data
        identifier = str(request.get("tool_use_id") or request_id)
        if name == "AskUserQuestion":
            return [_question_event(request_id, identifier, data)]
        event = claude_live_permission(identifier, name, data)
        return [{**event, "live": True, "request_id": request_id}]

    def answer(self, message: dict[str, Any]) -> None:
        request_id = str(message.get("request_id") or "")
        original = self._pending.pop(request_id, None)
        if original is None:
            return
        answers = message.get("answers")
        if isinstance(answers, dict):
            if not answers:
                decision: dict[str, Any] = {"behavior": "deny", "message": _UNANSWERED}
            else:
                decision = {"behavior": "allow", "updatedInput": {**original, "answers": {str(k): str(v) for k, v in answers.items()}}}
        elif message.get("behavior") == "allow":
            decision = {"behavior": "allow", "updatedInput": original}
        else:
            decision = {"behavior": "deny", "message": _DENIED}
        self._respond(request_id, decision)

    def deny_all(self) -> None:
        """Nobody is going to answer any more (the host closed): refuse what is still waiting."""
        for request_id in list(self._pending):
            self.answer({"request_id": request_id, "behavior": "deny"})

    def finish(self) -> None:
        try:
            self._stdin.close()
        except OSError:
            pass

    def _respond(self, request_id: str, payload: dict[str, Any], *, ok: bool = True) -> None:
        body = {"subtype": "success", "request_id": request_id, "response": payload} if ok else {**payload, "request_id": request_id}
        line = json.dumps({"type": "control_response", "response": body}, ensure_ascii=False) + "\n"
        with self._lock:
            try:
                self._stdin.write(line)
                self._stdin.flush()
            except (OSError, ValueError):
                pass  # the agent already exited


def _question_event(request_id: str, identifier: str, data: dict[str, Any]) -> Event:
    questions = []
    for item in data.get("questions") if isinstance(data.get("questions"), list) else []:
        if not isinstance(item, dict) or not item.get("question"):
            continue
        options = [
            {"label": str(option.get("label", "")), "description": str(option.get("description", ""))}
            for option in item.get("options", []) if isinstance(option, dict) and option.get("label")
        ]
        questions.append({"question": str(item["question"]), "header": str(item.get("header", "")),
                          "multiSelect": bool(item.get("multiSelect")), "options": options})
    return {"type": "question", "request_id": request_id, "id": identifier, "questions": questions}
