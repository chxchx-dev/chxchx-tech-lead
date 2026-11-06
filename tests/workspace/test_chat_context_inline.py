"""The visual chat hands the context files over inside the prompt: no shell command, no permission prompt."""

from __future__ import annotations

from pathlib import Path

from chxchx_tech_lead.integrations.explicit_memory import user_visible_text
from chxchx_tech_lead.workspace.agent_commands import CHAT_PANEL_REQUEST_MARKER
from chxchx_tech_lead.workspace.chat_context import inline_project_context


def project(tmp_path: Path) -> Path:
    (tmp_path / ".ai" / "memory").mkdir(parents=True)
    (tmp_path / "AGENTS.md").write_text("Reglas del repo\n", encoding="utf-8")
    (tmp_path / ".ai" / "CURRENT_STATE.md").write_text("# Estado\nTodo bien\n", encoding="utf-8")
    (tmp_path / ".ai" / "memory" / "PROJECT_MEMORY.md").write_text("Recuerda que el puerto es 8080\n", encoding="utf-8")
    return tmp_path


def test_existing_files_are_included_and_missing_ones_are_skipped(tmp_path: Path) -> None:
    text = inline_project_context(project(tmp_path), provider="Claude")
    assert '<archivo-de-contexto ruta="AGENTS.md">' in text and "Reglas del repo" in text
    assert '<archivo-de-contexto ruta=".ai/CURRENT_STATE.md">' in text and "Todo bien" in text
    assert "HANDOFF.md" not in text and "PROJECT.md\"" not in text
    assert "No ejecutes comandos" in text


def test_codex_skips_agents_md_because_it_loads_it_itself(tmp_path: Path) -> None:
    assert "AGENTS.md" not in inline_project_context(project(tmp_path), provider="Codex")


def test_nothing_to_include_adds_nothing(tmp_path: Path) -> None:
    assert inline_project_context(tmp_path, provider="Claude") == ""


def test_long_files_are_clipped_with_a_note_and_the_total_is_bounded(tmp_path: Path) -> None:
    root = project(tmp_path)
    for name in ("PROJECT.md", "HANDOFF.md"):
        (root / ".ai" / name).write_text("x" * 50_000, encoding="utf-8")
    text = inline_project_context(root, provider="Claude")
    assert "recortado" in text and len(text) < 60_000


def test_a_symlinked_context_file_is_not_followed(tmp_path: Path) -> None:
    root = project(tmp_path)
    secret = tmp_path.parent / f"{tmp_path.name}-secret.txt"
    secret.write_text("TOKEN=abc", encoding="utf-8")
    (root / ".ai" / "HANDOFF.md").symlink_to(secret)
    assert "TOKEN=abc" not in inline_project_context(root, provider="Claude")


def test_memory_capture_ignores_the_inlined_notes(tmp_path: Path) -> None:
    # The first user message carries the context; project notes must not look like "remember this" requests.
    message = inline_project_context(project(tmp_path), provider="Claude") + f"\n\n{CHAT_PANEL_REQUEST_MARKER}\nhola"
    assert "Recuerda que el puerto" not in user_visible_text(message)
