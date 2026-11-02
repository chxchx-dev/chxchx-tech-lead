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


def _conversation(tmp_path: Path, session_id: str, *texts: str) -> Conversation:
    return Conversation(
        "Claude",
        session_id,
        "test",
        datetime(2026, 11, 1, tzinfo=timezone.utc),
        tmp_path,
        tmp_path / f"{session_id}.jsonl",
        tuple(ConversationMessage("user", text) for text in texts),
    )


def test_ignores_everyday_words_that_only_look_like_memory(tmp_path: Path, monkeypatch) -> None:
    conversation = _conversation(
        tmp_path,
        "session-5",
        "Revisa el uso de memoria RAM del proceso.",
        "guarda el archivo y compila",
        "Siento que la memoria por chat no sirve bien.",
        "Claude no recuerda la frase que le di.",
        "<system-reminder>Recuerda que AGENTS.md manda.</system-reminder> Arregla el bug.",
        "Ok, recuerda que uso pnpm y no npm.",
        "Guarda en memoria que el puerto es 8080.",
        "Remember that staging uses Postgres 16.",
    )
    monkeypatch.setattr(explicit_memory, "list_conversations", lambda root, limit: [conversation])
    monkeypatch.setattr(
        "chxchx_tech_lead.integrations.opencode_memory.explicit_user_messages",
        lambda root, executable=None: [],
    )

    memories = explicit_memory.recover_explicit_memories(tmp_path)

    assert [item.message for item in memories] == [
        "Ok, recuerda que uso pnpm y no npm.",
        "Guarda en memoria que el puerto es 8080.",
        "Remember that staging uses Postgres 16.",
    ]


def test_same_request_from_another_chat_is_stored_once(tmp_path: Path) -> None:
    text = "Recuerda que la rama base es dev."
    explicit_memory.persist_explicit_memories(
        tmp_path,
        [
            explicit_memory.ExplicitMemory("Claude", "session-a", "2026-11-01", text),
            explicit_memory.ExplicitMemory("Codex", "session-b", "2026-11-01", text),
        ],
    )

    content = (tmp_path / ".ai/memory/PROJECT_MEMORY.md").read_text(encoding="utf-8")
    assert content.count("Petición explícita de memoria") == 1


def _noisy_memory_file(project: Path) -> Path:
    path = project / ".ai/memory/PROJECT_MEMORY.md"
    path.parent.mkdir(parents=True)
    path.write_text(
        "# Memoria del proyecto\n\n## Notas propias\n\nUsamos Qt 6.8.\n",
        encoding="utf-8",
    )
    explicit_memory.persist_explicit_memories(
        project,
        [
            explicit_memory.ExplicitMemory("Codex", "s1", "2026-10-01", "Revisa el uso de memoria RAM."),
            explicit_memory.ExplicitMemory("Codex", "s1", "2026-10-01", "Recuerda que el deploy es los viernes."),
            explicit_memory.ExplicitMemory("Claude", "s2", "2026-10-02", "guarda el archivo y compila"),
        ],
    )
    with path.open("a", encoding="utf-8") as stream:
        stream.write("\n## Más notas\n\nNo tocar.\n")
    return path


def test_prune_dry_run_reports_without_writing(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    path = _noisy_memory_file(tmp_path)
    before = path.read_text(encoding="utf-8")

    result = explicit_memory.prune_explicit_memories(tmp_path, dry_run=True)

    assert result.removed == ("Revisa el uso de memoria RAM.", "guarda el archivo y compila")
    assert result.backup is None
    assert path.read_text(encoding="utf-8") == before


def test_prune_keeps_hand_written_notes_and_backs_up(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    path = _noisy_memory_file(tmp_path)
    before = path.read_text(encoding="utf-8")

    result = explicit_memory.prune_explicit_memories(tmp_path)
    content = path.read_text(encoding="utf-8")

    assert len(result.removed) == 2
    assert result.backup is not None and result.backup.read_text(encoding="utf-8") == before
    assert "memoria RAM" not in content and "guarda el archivo" not in content
    assert "Recuerda que el deploy es los viernes." in content
    assert "Usamos Qt 6.8." in content and "No tocar." in content
    assert explicit_memory.prune_explicit_memories(tmp_path).removed == ()


def test_agent_pane_new_chat_compacts_and_injects_recent_sessions(tmp_path: Path, monkeypatch) -> None:
    from chxchx_tech_lead.workspace import agent_pane, session_journal

    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    state = tmp_path / ".ai" / "HANDOFF.md"
    state.parent.mkdir(parents=True)
    state.write_text("# Handoff\n\n" + "".join(f"## Paso {i}\n\n- x\n\n" for i in range(10, 0, -1)), encoding="utf-8")
    session_journal.record_session(
        tmp_path, session_journal.SessionSummary("Claude", "prev", "2026-11-01", "Arreglar memoria", (), "Hecho")
    )
    launched: list[list[str]] = []
    recorded: list[str] = []
    monkeypatch.setattr(
        "sys.argv",
        ["agent_pane", "--name", "claude", "--capture-explicit-memory", "--recover-explicit-memory",
         "--project-root", str(tmp_path), "--", "claude", "PROMPT"],
    )
    monkeypatch.setattr(agent_pane, "recover_explicit_memories", lambda root, **options: [])
    monkeypatch.setattr(
        agent_pane.subprocess, "run",
        lambda command, **kwargs: launched.append(command) or type("Result", (), {"returncode": 0})(),
    )
    monkeypatch.setattr(
        agent_pane, "summarize_latest_session",
        lambda root, provider, started_at: recorded.append(provider) or None,
    )

    assert agent_pane.main() == 0
    assert launched[0][-1].startswith("PROMPT") and "Arreglar memoria" in launched[0][-1]
    assert (tmp_path / ".ai" / "history" / "HANDOFF.md").is_file()
    assert recorded == ["Claude"]
