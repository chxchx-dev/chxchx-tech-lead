from __future__ import annotations

from pathlib import Path

import pytest

from chxchx_tech_lead.integrations.ci_workflow_files import (
    current_workflow_state,
    remove_workflow,
    write_workflow,
)
from chxchx_tech_lead.integrations.ci_workflows import plan_ci_workflow


def _python_project(root: Path) -> None:
    (root / "pyproject.toml").write_text('[project]\nname = "fixture"\n', encoding="utf-8")


def test_ci_workflow_dry_run_write_update_and_remove_keep_backups(tmp_path: Path, monkeypatch) -> None:
    project = tmp_path / "project"
    project.mkdir()
    _python_project(project)
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "chxchx-home"))
    plan = plan_ci_workflow(project)

    preview = write_workflow(project, plan, dry_run=True)
    assert preview.changed is True
    assert not preview.path.exists()

    first = write_workflow(project, plan)
    assert first.changed is True
    assert first.backup is None
    generated = first.path.read_text(encoding="utf-8")
    assert generated.endswith(plan.content.rstrip() + "\n")
    assert current_workflow_state(project).integrity_ok is True
    assert write_workflow(project, plan).changed is False

    updated_plan = type(plan)(plan.path, plan.stacks, plan.content + "# managed update\n", plan.warnings)
    updated = write_workflow(project, updated_plan)
    assert updated.changed is True
    assert updated.backup is not None and updated.backup.is_file()
    assert "chxchx-tech-managed" in updated.backup.read_text(encoding="utf-8")

    remove_preview = remove_workflow(project, dry_run=True)
    assert remove_preview.changed is True
    assert remove_preview.path.exists()
    removed = remove_workflow(project)
    assert removed.changed is True
    assert removed.backup is not None and removed.backup.is_file()
    assert not removed.path.exists()


def test_ci_workflow_never_overwrites_unmanaged_or_modified_files(tmp_path: Path) -> None:
    _python_project(tmp_path)
    plan = plan_ci_workflow(tmp_path)
    target = tmp_path / plan.path
    target.parent.mkdir(parents=True)
    target.write_text("name: user workflow\n", encoding="utf-8")
    with pytest.raises(ValueError, match="no está administrado"):
        write_workflow(tmp_path, plan)
    assert target.read_text(encoding="utf-8") == "name: user workflow\n"

    target.unlink()
    write_workflow(tmp_path, plan)
    target.write_text(target.read_text(encoding="utf-8") + "# user edit\n", encoding="utf-8")
    with pytest.raises(ValueError, match="cambió fuera"):
        write_workflow(tmp_path, plan)
    with pytest.raises(ValueError, match="no coincide"):
        remove_workflow(tmp_path)
