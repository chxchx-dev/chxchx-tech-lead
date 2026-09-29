from pathlib import Path

from chxchx_tech_lead.workspace.memory_history import list_memory_notes


def _write_note(root: Path, name: str, content: str) -> Path:
    path = root / ".ai" / "memory" / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    return path


def test_lists_only_notes_from_selected_project(tmp_path: Path):
    bokana = tmp_path / "bokana"
    security = tmp_path / "security"
    _write_note(bokana, "api.md", "# API decisions\nBokana-only context.")
    _write_note(security, "security.md", "# Security review\nSecurity-only context.")

    notes = list_memory_notes(bokana)

    assert [note.title for note in notes] == ["API decisions"]
    assert "Bokana-only" in notes[0].content
    assert notes[0].path.is_relative_to(bokana.resolve())


def test_filters_notes_by_title_or_body_case_insensitively(tmp_path: Path):
    project = tmp_path / "project"
    _write_note(project, "architecture.md", "# Architecture\nUses SQLite for persistence.")
    _write_note(project, "handoff.md", "# Handoff\nNext: validate the CLI.")

    notes = list_memory_notes(project, query="sqlite")

    assert len(notes) == 1
    assert notes[0].title == "Architecture"
    assert "SQLite" in notes[0].content


def test_empty_or_missing_project_memory_returns_no_notes(tmp_path: Path):
    assert list_memory_notes(tmp_path / "missing") == []

    project = tmp_path / "project"
    (project / ".ai" / "memory").mkdir(parents=True)
    assert list_memory_notes(project) == []



def test_lists_local_chat_history_for_selected_project(tmp_path: Path, monkeypatch):
    import json

    from chxchx_tech_lead.integrations.chat_history import list_conversations

    project = tmp_path / "demo"
    project.mkdir()
    codex_home = tmp_path / "codex"
    rollout = codex_home / "sessions" / "2026" / "09" / "29" / "rollout-test.jsonl"
    rollout.parent.mkdir(parents=True)
    rollout.write_text(
        "\n".join(
            [
                json.dumps({"type": "session_meta", "payload": {"id": "codex-1", "cwd": str(project)}}),
                json.dumps({"type": "event_msg", "payload": {"type": "user_message", "message": "Fix React Native startup"}}),
                json.dumps({"type": "response_item", "payload": {"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "Use pnpm start."}]}}),
            ]
        ),
        encoding="utf-8",
    )
    claude_dir = tmp_path / "claude" / "projects" / str(project).replace("/", "-")
    claude_dir.mkdir(parents=True)
    transcript = claude_dir / "claude-1.jsonl"
    transcript.write_text(
        "\n".join(
            [
                json.dumps({"type": "user", "message": {"role": "user", "content": "Review the auth flow"}}),
                json.dumps({"type": "assistant", "message": {"role": "assistant", "content": [{"type": "text", "text": "I found two issues."}]}}),
            ]
        ),
        encoding="utf-8",
    )
    monkeypatch.setenv("CODEX_HOME", str(codex_home))
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", str(tmp_path / "claude"))

    conversations = list_conversations(project)
    assert {item.provider for item in conversations} == {"Codex", "Claude"}
    assert {item.title for item in conversations} == {"Fix React Native startup", "Review the auth flow"}
    assert len(list_conversations(project, query="pnpm")) == 1
    assert len(list_conversations(project, query="AUTH")) == 1
    codex = next(item for item in conversations if item.provider == "Codex")
    assert codex.messages[-1].text == "Use pnpm start."
