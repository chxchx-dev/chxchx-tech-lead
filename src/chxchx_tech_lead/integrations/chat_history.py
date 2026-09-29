from __future__ import annotations

import json
import os
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any


@dataclass(frozen=True, slots=True)
class ConversationMessage:
    role: str
    text: str


@dataclass(frozen=True, slots=True)
class Conversation:
    provider: str
    session_id: str
    title: str
    modified_at: datetime
    project_root: Path
    transcript_path: Path
    messages: tuple[ConversationMessage, ...]

    @property
    def preview(self) -> str:
        return next((message.text.replace("\n", " ")[:180] for message in reversed(self.messages) if message.role == "user"), "")


def list_conversations(project_root: Path, query: str = "", limit: int = 200) -> list[Conversation]:
    """Read local Codex and Claude Code transcripts for one project."""
    root = project_root.resolve()
    normalized_query = query.strip().casefold()
    conversations: list[Conversation] = []
    conversations.extend(_codex_conversations(root))
    conversations.extend(_claude_conversations(root))
    if normalized_query:
        conversations = [
            item for item in conversations if normalized_query in _searchable_text(item)
        ]
    return sorted(conversations, key=lambda item: item.modified_at, reverse=True)[:max(0, limit)]


def _codex_conversations(project_root: Path) -> list[Conversation]:
    codex_home = Path(os.environ.get("CODEX_HOME", Path.home() / ".codex")).expanduser()
    sessions_root = codex_home / "sessions"
    if not sessions_root.is_dir():
        return []
    try:
        candidates = sorted(
            sessions_root.rglob("rollout-*.jsonl"),
            key=_modified_at,
            reverse=True,
        )[:500]
    except OSError:
        return []

    found: list[Conversation] = []
    for path in candidates:
        metadata = _codex_metadata(path)
        if metadata is None or not _same_path(Path(metadata[0]), project_root):
            continue
        try:
            conversation = _read_codex(path, cwd=metadata[0], session_id=metadata[1])
        except OSError:
            continue
        if conversation:
            found.append(conversation)
    return found


def _codex_metadata(path: Path) -> tuple[str, str] | None:
    session_id = path.stem.removeprefix("rollout-")
    try:
        with path.open(encoding="utf-8") as stream:
            for _ in range(100):
                line = stream.readline()
                if not line:
                    break
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if isinstance(record, dict) and record.get("type") == "session_meta":
                    payload = record.get("payload")
                    if not isinstance(payload, dict) or not isinstance(payload.get("cwd"), str):
                        return None
                    return payload["cwd"], str(payload.get("id") or session_id)
    except (OSError, UnicodeError):
        return None
    return None


def _read_codex(path: Path, *, cwd: str, session_id: str) -> Conversation | None:
    messages: list[ConversationMessage] = []
    try:
        with path.open(encoding="utf-8") as stream:
            for line in stream:
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if not isinstance(record, dict):
                    continue
                payload = record.get("payload")
                if not isinstance(payload, dict):
                    continue
                record_type = record.get("type")
                if record_type == "event_msg" and payload.get("type") == "user_message":
                    _append_message(messages, "user", payload.get("message"))
                elif record_type == "event_msg" and payload.get("type") in {"agent_message", "assistant_message"}:
                    _append_message(messages, "assistant", payload.get("message"))
                elif record_type == "response_item":
                    item = payload.get("item") if isinstance(payload.get("item"), dict) else payload
                    if item.get("type") == "message" and item.get("role") in {"user", "assistant"}:
                        _append_message(messages, str(item["role"]), item.get("content"))
    except (OSError, UnicodeError):
        return None
    if not messages:
        return None
    try:
        modified_at = datetime.fromtimestamp(path.stat().st_mtime).astimezone()
    except OSError:
        return None
    title = next((message.text for message in messages if message.role == "user"), "Chat Codex")
    return Conversation("Codex", session_id, _compact(title, 90), modified_at, Path(cwd), path, tuple(messages))


def _claude_conversations(project_root: Path) -> list[Conversation]:
    config_dir = Path(os.environ.get("CLAUDE_CONFIG_DIR", Path.home() / ".claude")).expanduser()
    folder_name = str(project_root).replace("\\", "/").replace(":", "-").replace("/", "-")
    project_dir = config_dir / "projects" / folder_name
    if not project_dir.is_dir():
        return []
    found: list[Conversation] = []
    try:
        candidates = sorted(project_dir.glob("*.jsonl"), key=lambda path: path.stat().st_mtime, reverse=True)[:500]
    except OSError:
        return []
    for path in candidates:
        conversation = _read_claude(path, project_root)
        if conversation:
            found.append(conversation)
    return found


def _read_claude(path: Path, project_root: Path) -> Conversation | None:
    messages: list[ConversationMessage] = []
    session_id = path.stem
    custom_title: str | None = None
    try:
        with path.open(encoding="utf-8") as stream:
            for line in stream:
                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if not isinstance(record, dict):
                    continue
                if record.get("type") == "custom-title":
                    custom_title = str(record.get("customTitle") or record.get("title") or "") or custom_title
                    continue
                role = record.get("type")
                if role not in {"user", "assistant"} or record.get("isSidechain"):
                    continue
                message = record.get("message")
                if isinstance(message, dict):
                    role = message.get("role", role)
                    content = message.get("content")
                else:
                    content = message
                if role in {"user", "assistant"}:
                    _append_message(messages, str(role), content)
    except (OSError, UnicodeError):
        return None
    if not messages:
        return None
    try:
        modified_at = datetime.fromtimestamp(path.stat().st_mtime).astimezone()
    except OSError:
        return None
    title = custom_title or next((message.text for message in messages if message.role == "user"), "Chat Claude")
    return Conversation("Claude", session_id, _compact(title, 90), modified_at, project_root, path, tuple(messages))


def _append_message(messages: list[ConversationMessage], role: str, content: Any) -> None:
    if isinstance(content, str):
        text = content
    elif isinstance(content, list):
        parts = []
        for item in content:
            if isinstance(item, str):
                parts.append(item)
            elif isinstance(item, dict):
                if isinstance(item.get("text"), str):
                    parts.append(item["text"])
                elif item.get("type") == "tool_use" and item.get("name"):
                    parts.append(f"[Herramienta: {item['name']}]")
        text = "\n".join(parts)
    else:
        return
    text = text.strip()
    if text and (not messages or (messages[-1].role, messages[-1].text) != (role, text)):
        messages.append(ConversationMessage(role, text))


def _searchable_text(conversation: Conversation) -> str:
    parts = [conversation.title, conversation.preview, conversation.provider]
    parts.extend(message.text for message in conversation.messages)
    return "\n".join(parts).casefold()


def _modified_at(path: Path) -> float:
    try:
        return path.stat().st_mtime
    except OSError:
        return 0.0


def _same_path(raw_path: Path, project_root: Path) -> bool:
    try:
        return raw_path.expanduser().resolve() == project_root
    except OSError:
        return False


def _compact(value: str, maximum: int) -> str:
    normalized = " ".join(value.split())
    return normalized if len(normalized) <= maximum else normalized[: maximum - 1].rstrip() + "…"
