from __future__ import annotations

from pathlib import Path

from chxchx_tech_lead.workspace.context_compaction import auto_compact_enabled, compact_context


def _state_file(root: Path, sections: int) -> Path:
    path = root / ".ai" / "CURRENT_STATE.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    recent = "".join(f"## Cambio {index}\n\n- Detalle {index}.\n\n" for index in range(sections, 0, -1))
    path.write_text(
        "# Current State\n\n"
        + recent
        + "## Estado actual\n\n- Canónico.\n\n"
        + "```md\n## No es una sección\n```\n\n"
        + "<!-- chxchx-tech:start workspace-handoff -->\n## Contexto administrado\n<!-- chxchx-tech:end workspace-handoff -->\n",
        encoding="utf-8",
    )
    return path


def test_dry_run_reports_without_writing(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    path = _state_file(tmp_path, 9)
    before = path.read_text(encoding="utf-8")

    result = compact_context(tmp_path, keep=3, dry_run=True)

    assert result.files[0].moved == tuple(f"Cambio {index}" for index in range(6, 0, -1))
    assert result.backup is None
    assert path.read_text(encoding="utf-8") == before
    assert not (tmp_path / ".ai" / "history").exists()


def test_archives_old_sections_and_keeps_pinned_ones(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    path = _state_file(tmp_path, 9)

    result = compact_context(tmp_path, keep=3)
    content = path.read_text(encoding="utf-8")
    archive = (tmp_path / ".ai" / "history" / "CURRENT_STATE.md").read_text(encoding="utf-8")

    assert result.backup is not None and (result.backup / ".ai" / "CURRENT_STATE.md").is_file()
    assert [line for line in content.splitlines() if line.startswith("## Cambio")] == [
        "## Cambio 9", "## Cambio 8", "## Cambio 7"
    ]
    assert "## Estado actual" in content and "## No es una sección" in content
    assert "<!-- chxchx-tech:start workspace-handoff -->" in content and "## Contexto administrado" in content
    assert "`.ai/history/CURRENT_STATE.md`" in content
    assert archive.index("## Cambio 6") < archive.index("## Cambio 1")
    assert not compact_context(tmp_path, keep=3).changed


def test_later_compaction_prepends_newer_sections_to_archive(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    path = _state_file(tmp_path, 4)
    compact_context(tmp_path, keep=2)
    content = path.read_text(encoding="utf-8")
    path.write_text(content.replace("## Cambio 4", "## Cambio 6\n\n- Nuevo.\n\n## Cambio 5\n\n- Nuevo.\n\n## Cambio 4", 1), encoding="utf-8")

    compact_context(tmp_path, keep=2)
    archive = (tmp_path / ".ai" / "history" / "CURRENT_STATE.md").read_text(encoding="utf-8")

    headings = [line for line in archive.splitlines() if line.startswith("## ")]
    assert headings == ["## Cambio 4", "## Cambio 3", "## Cambio 2", "## Cambio 1"]
    assert archive.count("# Historial de CURRENT_STATE") == 1


def test_budget_archives_large_sections_but_keeps_the_newest(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))
    path = tmp_path / ".ai" / "HANDOFF.md"
    path.parent.mkdir(parents=True)
    path.write_text("# Handoff\n\n## Nuevo\n\n" + "x" * 3000 + "\n\n## Viejo\n\n- y\n", encoding="utf-8")

    result = compact_context(tmp_path, budget=1000)

    assert result.files[0].moved == ("Viejo",)
    assert "## Nuevo" in path.read_text(encoding="utf-8")


def test_projects_can_opt_out_of_automatic_compaction(tmp_path: Path) -> None:
    assert auto_compact_enabled(tmp_path)
    config = tmp_path / ".ai" / "chxchx-tech.toml"
    config.parent.mkdir()
    config.write_text("auto_compact_context = false\n", encoding="utf-8")
    assert not auto_compact_enabled(tmp_path)
