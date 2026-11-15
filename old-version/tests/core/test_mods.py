from pathlib import Path

import pytest

from chxchx_tech_lead.core.mods import (
    ModRegistry,
    apply_mods,
    has_claude_project,
    skill_agents,
    remove_mods,
)
from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.core.preparation import prepare_project


def test_catalog_recommends_general_and_detected_native_mods(tmp_path: Path) -> None:
    (tmp_path / "main.cpp").write_text("int main() {}", encoding="utf-8")
    (tmp_path / "CMakeLists.txt").write_text("find_package(Qt6 REQUIRED)", encoding="utf-8")

    recommendations = {item.mod.name: item for item in ModRegistry().recommendations(tmp_path)}

    assert "chxchx-model-advisor" in recommendations
    assert "chxchx-review-before-merge" in recommendations
    assert "chxchx-native-qt" in recommendations
    assert "qt" in " ".join(recommendations["chxchx-native-qt"].reasons)


def test_every_catalog_mod_is_well_formed_and_stack_mods_follow_the_project(tmp_path: Path) -> None:
    mods = ModRegistry().list()
    assert len({mod.name for mod in mods}) == len(mods) and len(mods) >= 20
    assert all(mod.name.startswith("chxchx-") and mod.description and len(mod.instructions) > 200 for mod in mods)

    (tmp_path / "app.py").write_text("print(1)", encoding="utf-8")
    (tmp_path / "pyproject.toml").write_text("[project]\nname='x'\n", encoding="utf-8")
    (tmp_path / "Dockerfile").write_text("FROM python:3.12\n", encoding="utf-8")
    (tmp_path / ".github" / "workflows").mkdir(parents=True)
    (tmp_path / ".github" / "workflows" / "ci.yml").write_text("on: push\n", encoding="utf-8")

    names = {item.mod.name for item in ModRegistry().recommendations(tmp_path)}

    assert {"chxchx-python-quality", "chxchx-docker-hygiene", "chxchx-github-actions", "chxchx-security-review"} <= names
    assert not names & {"chxchx-native-qt", "chxchx-web-react", "chxchx-rust-safety", "chxchx-dotnet-csharp"}


def test_apply_is_project_local_and_idempotent(tmp_path: Path) -> None:
    names = ("chxchx-model-advisor",)

    preview = apply_mods(tmp_path, names, dry_run=True)
    target = tmp_path / ".claude" / "skills" / names[0] / "SKILL.md"
    assert preview.installed == names
    assert not target.exists()

    applied = apply_mods(tmp_path, names)
    assert applied.installed == names
    assert "<!-- chxchx-mod:chxchx-model-advisor -->" in target.read_text(encoding="utf-8")
    assert "`/model sonnet`" in target.read_text(encoding="utf-8")
    assert apply_mods(tmp_path, names).unchanged == names


def test_apply_refuses_unmanaged_file_without_overwriting(tmp_path: Path) -> None:
    target = tmp_path / ".claude" / "skills" / "chxchx-model-advisor" / "SKILL.md"
    target.parent.mkdir(parents=True)
    target.write_text("user-owned instructions", encoding="utf-8")

    with pytest.raises(ValueError, match="no está administrado"):
        apply_mods(tmp_path, ("chxchx-model-advisor",))

    assert target.read_text(encoding="utf-8") == "user-owned instructions"


def test_update_managed_mod_keeps_backup(tmp_path: Path) -> None:
    name = "chxchx-model-advisor"
    target = tmp_path / ".claude" / "skills" / name / "SKILL.md"
    target.parent.mkdir(parents=True)
    target.write_text(f"<!-- chxchx-mod:{name} -->\ncustomized", encoding="utf-8")

    result = apply_mods(tmp_path, (name,))

    assert len(result.backups) == 1
    assert Path(result.backups[0]).read_text(encoding="utf-8").endswith("customized")
    assert "Asesor de modelo" in target.read_text(encoding="utf-8")


def test_remove_requires_chxchx_marker_and_saves_rollback_copy(tmp_path: Path) -> None:
    name = "chxchx-model-advisor"
    installed = apply_mods(tmp_path, (name,))
    target = tmp_path / ".claude" / "skills" / name / "SKILL.md"
    original = target.read_text(encoding="utf-8")

    preview = remove_mods(tmp_path, (name,), dry_run=True)
    assert preview.installed == (name,)
    assert target.is_file()

    removed = remove_mods(tmp_path, (name,))
    assert not target.exists()
    assert Path(removed.backups[0]).read_text(encoding="utf-8") == original
    assert installed.installed == (name,)


def test_claude_project_detection(tmp_path: Path) -> None:
    assert not has_claude_project(tmp_path)
    (tmp_path / "CLAUDE.md").write_text("project rules", encoding="utf-8")
    assert has_claude_project(tmp_path)


def test_preparation_only_includes_mods_for_claude_projects(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setattr("chxchx_tech_lead.core.bootstrap.basic_memory_available", lambda: False)
    project = ProjectInfo(root=tmp_path, name=tmp_path.name)

    plain = prepare_project(project, dry_run=True)
    assert plain.mods == ()
    assert not (tmp_path / ".claude").exists()

    (tmp_path / "CLAUDE.md").write_text("Claude project instructions", encoding="utf-8")
    claude = prepare_project(project, dry_run=True)
    assert "chxchx-model-advisor" in claude.mods
    assert not (tmp_path / ".claude").exists()


def _agents_config(project: Path, *commands: str) -> None:
    (project / ".ai").mkdir(exist_ok=True)
    body = "".join(f'[[workspace.agents]]\nid = "{c}"\ncommand = ["{c}"]\ncwd = "."\n' for c in commands)
    (project / ".ai" / "chxchx-tech.toml").write_text(body, encoding="utf-8")


def test_codex_projects_also_get_the_mods_where_codex_reads_skills(tmp_path: Path) -> None:
    _agents_config(tmp_path, "claude", "codex")
    names = ("chxchx-test-strategy",)

    first = apply_mods(tmp_path, names)
    again = apply_mods(tmp_path, names)

    claude, codex = (tmp_path / root / "skills" / names[0] / "SKILL.md" for root in (".claude", ".agents"))
    assert first.installed == names and again.unchanged == names and not again.installed
    assert claude.read_text(encoding="utf-8") == codex.read_text(encoding="utf-8")
    assert skill_agents(tmp_path) == ("claude", "codex")


def test_opencode_only_projects_use_claude_skills_and_no_codex_copy(tmp_path: Path) -> None:
    _agents_config(tmp_path, "opencode")
    assert skill_agents(tmp_path) == ("opencode",)

    apply_mods(tmp_path, ("chxchx-test-strategy",))

    assert (tmp_path / ".claude" / "skills" / "chxchx-test-strategy" / "SKILL.md").is_file()
    assert not (tmp_path / ".agents").exists()


def test_removal_retires_every_copy_with_a_backup_and_never_touches_foreign_files(tmp_path: Path) -> None:
    _agents_config(tmp_path, "codex")
    apply_mods(tmp_path, ("chxchx-test-strategy",))

    preview = remove_mods(tmp_path, ("chxchx-test-strategy",), dry_run=True)
    removed = remove_mods(tmp_path, ("chxchx-test-strategy",))

    assert preview.installed == removed.installed == ("chxchx-test-strategy",) and len(removed.backups) == 2
    assert not (tmp_path / ".agents" / "skills" / "chxchx-test-strategy").exists()
    foreign = tmp_path / ".agents" / "skills" / "chxchx-test-strategy" / "SKILL.md"
    foreign.parent.mkdir(parents=True)
    foreign.write_text("mío", encoding="utf-8")
    with pytest.raises(ValueError, match="no está administrado"):
        remove_mods(tmp_path, ("chxchx-test-strategy",))
    with pytest.raises(ValueError, match="no se sobrescribió"):
        apply_mods(tmp_path, ("chxchx-test-strategy",))
    assert foreign.read_text(encoding="utf-8") == "mío"


def test_the_json_listing_studio_reads_is_never_wrapped(tmp_path: Path) -> None:
    """A console wraps long lines at 80 columns, even inside the strings of a JSON document."""
    import json

    from typer.testing import CliRunner

    from chxchx_tech_lead.cli import app

    result = CliRunner().invoke(app, ["mod", "list", "--path", str(tmp_path), "--json"])

    assert result.stdout.count("\n") == 1
    assert len(json.loads(result.stdout)["mods"]) >= 20
