from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path

from chxchx_tech_lead.integrations import explicit_memory
from chxchx_tech_lead.integrations.chat_history import Conversation, ConversationMessage


def test_recovers_only_direct_requests_from_project_chats(tmp_path: Path, monkeypatch) -> None:
    conversation = Conversation(
        "Codex",
        "session-1",
        "test",
        datetime(2026, 10, 7, tzinfo=timezone.utc),
        tmp_path,
        tmp_path / "chat.jsonl",
        (
            ConversationMessage("user", "Recuerda esta frase: el colibrí azul cruza el patio."),
            ConversationMessage("user", "Quiero que te acuerdes de esta frase: la ventana mira al sur."),
            ConversationMessage("user", "Recuérdame la frase que te di hace un rato."),
            ConversationMessage("user", "Recuerda mi contraseña: no-guardar-esta."),
            ConversationMessage(
                "user",
                "Ando probando y le puse que se acordara una frase, pero no se acuerda de la frase.",
            ),
            ConversationMessage("assistant", "Recordaré todo."),
        ),
    )
    monkeypatch.setattr(explicit_memory, "list_conversations", lambda root, limit: [conversation])

    memories = explicit_memory.recover_explicit_memories(tmp_path)

    assert [item.message for item in memories] == [
        "Recuerda esta frase: el colibrí azul cruza el patio.",
        "Quiero que te acuerdes de esta frase: la ventana mira al sur.",
    ]


def test_persists_without_duplicate_and_formats_exact_request(tmp_path: Path) -> None:
    memory = explicit_memory.ExplicitMemory(
        "Claude", "session-2", "2026-10-07", "Recuerda: la frase exacta es ‘luna de cobre’."
    )

    explicit_memory.persist_explicit_memories(tmp_path, [memory, memory])
    explicit_memory.persist_explicit_memories(tmp_path, [memory])
    content = (tmp_path / ".ai/memory/PROJECT_MEMORY.md").read_text(encoding="utf-8")

    assert content.count("Petición explícita de memoria") == 1
    assert "luna de cobre" in content
    assert "luna de cobre" in explicit_memory.format_recovered_memories([memory])


def test_persists_multiple_requests_from_one_chat(tmp_path: Path) -> None:
    memories = [
        explicit_memory.ExplicitMemory("Codex", "session-4", "2026-10-07", text)
        for text in ("Recuerda el color: azul.", "Recuerda la ciudad: Cali.")
    ]

    explicit_memory.persist_explicit_memories(tmp_path, memories)
    content = (tmp_path / ".ai/memory/PROJECT_MEMORY.md").read_text(encoding="utf-8")

    assert "azul" in content
    assert "Cali" in content


def test_agent_pane_captures_memory_after_codex_exits(tmp_path: Path, monkeypatch) -> None:
    from chxchx_tech_lead.workspace import agent_pane

    memory = explicit_memory.ExplicitMemory(
        "Codex", "session-3", "2026-10-07", "Recuerda esta frase: el río cruza el valle."
    )
    saved: list[Path] = []
    monkeypatch.setattr(
        "sys.argv",
        [
            "agent_pane",
            "--name",
            "codex",
            "--capture-explicit-memory",
            "--project-root",
            str(tmp_path),
            "--",
            "codex",
        ],
    )
    monkeypatch.setattr(agent_pane.subprocess, "run", lambda *args, **kwargs: type("Result", (), {"returncode": 0})())
    monkeypatch.setattr(agent_pane, "recover_explicit_memories", lambda root: [memory])
    monkeypatch.setattr(agent_pane, "persist_explicit_memories", lambda root, items: saved.append(root))

    result = agent_pane.main()

    assert result == 0
    assert saved == [tmp_path.resolve()]
