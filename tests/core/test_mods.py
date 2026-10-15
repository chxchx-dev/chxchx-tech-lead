from pathlib import Path

import pytest

from chxchx_tech_lead.core.mods import (
    ModRegistry,
    apply_mods,
    has_claude_project,
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
