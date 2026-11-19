"""The visual chat hands the context files over inside the prompt: no shell command, no permission prompt."""

from __future__ import annotations

from pathlib import Path

import pytest

from chxchx_tech_lead.integrations.explicit_memory import user_visible_text
from chxchx_tech_lead.workspace.agent_commands import CHAT_PANEL_REQUEST_MARKER
from chxchx_tech_lead.workspace.chat_context import inline_project_context
from chxchx_tech_lead.workspace import chat_first_message


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


def test_a_project_without_context_says_so_instead_of_sending_the_agent_looking(tmp_path: Path) -> None:
    text = inline_project_context(tmp_path, provider="Claude")
    assert "archivo-de-contexto" not in text and "no los busques" in text


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
    try:
        (root / ".ai" / "HANDOFF.md").symlink_to(secret)
    except OSError as error:
        if getattr(error, "winerror", None) != 1314:
            raise
        pytest.skip("Windows no permite crear symlinks sin el privilegio requerido")
    assert "TOKEN=abc" not in inline_project_context(root, provider="Claude")


def test_memory_capture_ignores_the_inlined_notes(tmp_path: Path) -> None:
    # The first user message carries the context; project notes must not look like "remember this" requests.
    message = inline_project_context(project(tmp_path), provider="Claude") + f"\n\n{CHAT_PANEL_REQUEST_MARKER}\nhola"
    assert "Recuerda que el puerto" not in user_visible_text(message)


def test_studio_uses_rust_context_without_rebuilding_or_compacting_it(tmp_path: Path, monkeypatch) -> None:
    notices: list[tuple[str, bool]] = []

    def recovered(
        _root: Path,
        _notify,
        *,
        compact: bool = True,
        include_journal: bool = True,
        include_memories: bool = True,
    ) -> str:
        assert compact is False
        assert include_journal is False
        assert include_memories is False
        return "\nmemoria recuperada"

    def python_rebuild(*_args, **_kwargs):
        raise AssertionError("Studio ya preparó el contexto en Rust")

    monkeypatch.setattr(chat_first_message, "prepare_new_chat", recovered)
    monkeypatch.setattr(chat_first_message, "inline_project_context", python_rebuild)
    monkeypatch.setattr(chat_first_message, "new_chat_prompt", python_rebuild)

    message = chat_first_message.first_message(
        tmp_path,
        "Claude",
        "Continúa",
        lambda text, is_error: notices.append((text, is_error)),
        rust_context="contexto Rust",
        rust_context_compacted_sections=2,
        handoff="estado traspasado",
        recap="recap anterior",
    )

    assert message.startswith("contexto Rust\nmemoria recuperada")
    assert "traspasado" in message and "recap anterior" not in message
    assert message.endswith(f"{CHAT_PANEL_REQUEST_MARKER}\nContinúa")
    assert notices == [("Contexto compactado: 2 sección(es) antiguas movidas a .ai/history/.", False)]
