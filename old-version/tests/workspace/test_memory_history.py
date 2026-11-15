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
    project_key = str(project).replace("\\", "-").replace(":", "-").replace("/", "-")
    claude_dir = tmp_path / "claude" / "projects" / project_key
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


def _write_claude_transcript(folder: Path, name: str, cwd: Path, text: str) -> None:
    import json

    folder.mkdir(parents=True, exist_ok=True)
    (folder / name).write_text(
        json.dumps({"type": "user", "cwd": str(cwd), "message": {"role": "user", "content": text}}),
        encoding="utf-8",
    )


def test_finds_claude_chats_for_paths_with_dots_and_underscores(tmp_path: Path, monkeypatch):
    import re

    from chxchx_tech_lead.integrations.chat_history import list_conversations

    project = tmp_path / "my_app.v2"
    project.mkdir()
    folder = tmp_path / "claude" / "projects" / re.sub(r"[^A-Za-z0-9]", "-", str(project))
    _write_claude_transcript(folder, "c1.jsonl", project, "Recuerda que usamos pnpm")
    monkeypatch.setenv("CODEX_HOME", str(tmp_path / "codex"))
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", str(tmp_path / "claude"))

    assert [item.title for item in list_conversations(project)] == ["Recuerda que usamos pnpm"]


def test_ignores_claude_chats_from_a_colliding_project_folder(tmp_path: Path, monkeypatch):
    import re

    from chxchx_tech_lead.integrations.chat_history import list_conversations

    project = tmp_path / "my_app"
    other = tmp_path / "my-app"
    project.mkdir()
    other.mkdir()
    folder = tmp_path / "claude" / "projects" / re.sub(r"[^A-Za-z0-9]", "-", str(project))
    _write_claude_transcript(folder, "mine.jsonl", project, "Chat propio")
    _write_claude_transcript(folder, "other.jsonl", other, "Chat de otro proyecto")
    monkeypatch.setenv("CODEX_HOME", str(tmp_path / "codex"))
    monkeypatch.setenv("CLAUDE_CONFIG_DIR", str(tmp_path / "claude"))

    assert [item.title for item in list_conversations(project)] == ["Chat propio"]
