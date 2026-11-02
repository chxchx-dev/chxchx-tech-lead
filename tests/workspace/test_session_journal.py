from __future__ import annotations

from datetime import datetime, timedelta, timezone
from pathlib import Path

from chxchx_tech_lead.integrations.chat_history import Conversation, ConversationMessage
from chxchx_tech_lead.workspace import session_journal


def _conversation(root: Path, session_id: str, modified_at: datetime, provider: str = "Claude") -> Conversation:
    return Conversation(
        provider,
        session_id,
        "titulo",
        modified_at,
        root,
        root / f"{session_id}.jsonl",
        (
            ConversationMessage("user", "<ide_selection>ruido</ide_selection> Arregla la memoria por chat"),
            ConversationMessage("assistant", "[Herramienta: Bash]"),
            ConversationMessage("user", "Ahora agrega pruebas"),
            ConversationMessage("user", "Mi token es abc123"),
            ConversationMessage("assistant", "Listo: 257 pruebas pasan.\n[Herramienta: Edit]"),
        ),
    )


def test_summarizes_session_used_after_start(tmp_path: Path, monkeypatch) -> None:
    started = datetime(2026, 11, 1, 12, tzinfo=timezone.utc)
    conversations = [
        _conversation(tmp_path, "codex-1", started + timedelta(minutes=9), "Codex"),
        _conversation(tmp_path, "claude-1", started + timedelta(minutes=5)),
    ]
    monkeypatch.setattr(session_journal, "list_conversations", lambda root, limit: conversations)

    summary = session_journal.summarize_latest_session(tmp_path, provider="Claude", started_at=started)

    assert summary is not None and summary.session_id == "claude-1"
    assert summary.opening == "Arregla la memoria por chat"
    assert summary.requests == ("Ahora agrega pruebas", "(omitido: posible secreto)")
    assert summary.outcome == "Listo: 257 pruebas pasan."


def test_ignores_transcripts_older_than_the_session(tmp_path: Path, monkeypatch) -> None:
    started = datetime(2026, 11, 1, 12, tzinfo=timezone.utc)
    old = _conversation(tmp_path, "claude-old", started - timedelta(hours=2))
    monkeypatch.setattr(session_journal, "list_conversations", lambda root, limit: [old])

    assert session_journal.summarize_latest_session(tmp_path, provider="Claude", started_at=started) is None


def test_records_newest_first_refreshes_resumed_sessions_and_caps(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr(session_journal, "MAX_ENTRIES", 3)
    for index in range(4):
        session_journal.record_session(
            tmp_path,
            session_journal.SessionSummary("Claude", f"s{index}", "2026-11-01", f"Pedido {index}", (), ""),
        )
    session_journal.record_session(
        tmp_path, session_journal.SessionSummary("Claude", "s2", "2026-11-01", "Pedido 2 reanudado", (), "")
    )
    content = (tmp_path / session_journal.JOURNAL).read_text(encoding="utf-8")

    assert [line.split("`")[1] for line in content.splitlines() if line.startswith("## ")] == ["s2", "s3", "s1"]
    assert "Pedido 2 reanudado" in content and "- Inicio: Pedido 2\n" not in content

    prompt = session_journal.recent_sessions_prompt(tmp_path, count=2)
    assert "Pedido 2 reanudado" in prompt and "Pedido 3" in prompt and "Pedido 1" not in prompt
    assert "<!-- session:" not in prompt


def test_no_journal_means_no_prompt(tmp_path: Path) -> None:
    assert session_journal.recent_sessions_prompt(tmp_path) == ""


def test_opening_skips_the_injected_context_preamble(tmp_path: Path, monkeypatch) -> None:
    from chxchx_tech_lead.workspace.agent_commands import CHAT_PANEL_REQUEST_MARKER, new_chat_prompt

    started = datetime(2026, 11, 1, 12, tzinfo=timezone.utc)
    terminal = Conversation("Codex", "c1", "t", started + timedelta(minutes=1), tmp_path, tmp_path / "c1.jsonl", (
        ConversationMessage("user", new_chat_prompt(str(tmp_path))),
        ConversationMessage("user", "Arregla el login"),
    ))
    panel = Conversation("Claude", "c2", "t", started + timedelta(minutes=1), tmp_path, tmp_path / "c2.jsonl", (
        ConversationMessage("user", f"{new_chat_prompt(str(tmp_path))}\n\n{CHAT_PANEL_REQUEST_MARKER}\n¿Qué imprime app.py?"),
    ))
    monkeypatch.setattr(session_journal, "list_conversations", lambda root, limit: [terminal, panel])

    codex = session_journal.summarize_latest_session(tmp_path, provider="Codex", started_at=started)
    claude = session_journal.summarize_latest_session(tmp_path, provider="Claude", started_at=started)

    assert codex is not None and codex.opening == "Arregla el login"
    assert claude is not None and claude.opening == "¿Qué imprime app.py?"


def test_opening_skips_agents_instructions_injected_by_codex(tmp_path: Path, monkeypatch) -> None:
    started = datetime(2026, 11, 2, 6, tzinfo=timezone.utc)
    conversation = Conversation("Codex", "c3", "t", started + timedelta(minutes=1), tmp_path, tmp_path / "c3.jsonl", (
        ConversationMessage("user", f"# AGENTS.md instructions for {tmp_path}\n\n<INSTRUCTIONS>\nReglas\n</INSTRUCTIONS>"),
        ConversationMessage("user", "Hola, dame un resumen del proyecto"),
    ))
    monkeypatch.setattr(session_journal, "list_conversations", lambda root, limit: [conversation])

    summary = session_journal.summarize_latest_session(tmp_path, provider="Codex", started_at=started)

    assert summary is not None and summary.opening == "Hola, dame un resumen del proyecto"
