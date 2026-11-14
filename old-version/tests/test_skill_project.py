from pathlib import Path

from chxchx_tech_lead.skills import (
    SkillRegistry,
    enabled_skills,
    set_skill_enabled,
    sync_project_skills,
)


def _create_python_skill(root: Path) -> SkillRegistry:
    folder = root / "python-engineering"
    folder.mkdir(parents=True)
    (folder / "skill.toml").write_text(
        'name = "python-engineering"\n'
        'description = "Python conventions."\n'
        'stacks = ["python"]\n'
        'languages = ["python"]\n',
        encoding="utf-8",
    )
    (folder / "SKILL.md").write_text("Prefer focused Python modules.", encoding="utf-8")
    return SkillRegistry((root,))


def test_enable_dry_run_does_not_create_project_configuration(tmp_path: Path) -> None:
    project = tmp_path / "project"
    project.mkdir()
    registry = _create_python_skill(tmp_path / "library")

    result = set_skill_enabled(project, "python-engineering", True, registry=registry, dry_run=True)

    assert result.changed is True
    assert result.enabled == ("python-engineering",)
    assert not (project / ".ai").exists()


def test_enable_and_sync_are_idempotent_and_preserve_user_content(
    tmp_path: Path, monkeypatch
) -> None:
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "chxchx-home"))
    project = tmp_path / "project"
    ai = project / ".ai"
    ai.mkdir(parents=True)
    (ai / "SKILLS.md").write_text("My local skill notes.\n", encoding="utf-8")
    (project / "AGENTS.md").write_text("Local project rules.\n", encoding="utf-8")
    registry = _create_python_skill(tmp_path / "library")

    enabled = set_skill_enabled(project, "python-engineering", True, registry=registry)
    first_sync = sync_project_skills(project, registry=registry)
    context = (ai / "SKILLS.md").read_text(encoding="utf-8")
    agents = (project / "AGENTS.md").read_text(encoding="utf-8")

    assert enabled.changed and enabled.backup is not None
    assert enabled_skills(project) == ["python-engineering"]
    assert first_sync.changed and first_sync.backup is not None
    assert "My local skill notes." in context
    assert "Prefer focused Python modules." in context
    assert "Local project rules." in agents
    assert sync_project_skills(project, registry=registry).changed is False

    set_skill_enabled(project, "python-engineering", False, registry=registry)
    sync_project_skills(project, registry=registry)

    assert enabled_skills(project) == []
    assert "Prefer focused Python modules." not in (ai / "SKILLS.md").read_text(encoding="utf-8")
    assert "Local project rules." in (project / "AGENTS.md").read_text(encoding="utf-8")
