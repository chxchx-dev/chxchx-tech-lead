from __future__ import annotations

from pathlib import Path

import pytest

from chxchx_tech_lead.integrations.ci_workflows import plan_ci_workflow


def test_ci_plan_covers_python_node_and_cross_platform_qt(tmp_path: Path) -> None:
    (tmp_path / "pyproject.toml").write_text(
        '[project]\nname = "fixture"\nrequires-python = ">=3.12,<3.14"\n', encoding="utf-8"
    )
    (tmp_path / "tests").mkdir()
    (tmp_path / "uv.lock").write_text("version = 1\n", encoding="utf-8")
    (tmp_path / "CMakeLists.txt").write_text(
        "find_package(Qt6 6.8 REQUIRED COMPONENTS Core5Compat)\n", encoding="utf-8"
    )
    (tmp_path / "package.json").write_text(
        '{"scripts":{"test":"vitest run","typecheck":"tsc --noEmit"}}', encoding="utf-8"
    )
    (tmp_path / "package-lock.json").write_text("{}", encoding="utf-8")

    plan = plan_ci_workflow(tmp_path)

    assert plan.stacks == ("Python", "CMake + Qt", "Node.js")
    assert "python-version: ['3.12', '3.13']" in plan.content
    assert "uv sync --locked --all-groups" in plan.content
    assert "uv run --locked --with pytest pytest" in plan.content
    assert "windows-2025, macos-15" in plan.content
    assert "modules: qt5compat" in plan.content
    assert "npm ci" in plan.content
    assert "npm run typecheck --if-present" in plan.content
    assert "permissions:\n  contents: read" in plan.content
    assert "workflow_dispatch" in plan.content
    assert "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1" in plan.content
    assert "version: '6.12.0'" in plan.content


def test_ci_plan_uses_requirements_and_reports_unlocked_npm(tmp_path: Path) -> None:
    (tmp_path / "requirements.txt").write_text("requests\n", encoding="utf-8")
    (tmp_path / "package.json").write_text('{"scripts":{}}', encoding="utf-8")

    plan = plan_ci_workflow(tmp_path)

    assert "uv pip install --system --requirements requirements.txt" in plan.content
    assert "npm install" in plan.content
    assert any("lockfile de npm" in warning for warning in plan.warnings)
    assert any("no declara scripts" in warning for warning in plan.warnings)


def test_ci_plan_rejects_unrecognized_projects(tmp_path: Path) -> None:
    with pytest.raises(ValueError, match="No encontré un build compatible"):
        plan_ci_workflow(tmp_path)


def test_ci_plan_includes_current_python_and_respects_older_supported_ranges(tmp_path: Path) -> None:
    (tmp_path / "pyproject.toml").write_text(
        '[project]\nname = "fixture"\nrequires-python = ">=3.9,<3.11"\n', encoding="utf-8"
    )

    plan = plan_ci_workflow(tmp_path)

    assert "python-version: ['3.9', '3.10']" in plan.content

    (tmp_path / "pyproject.toml").write_text(
        '[project]\nname = "fixture"\nrequires-python = ">=3.15"\n', encoding="utf-8"
    )
    plan = plan_ci_workflow(tmp_path)
    assert "python-version: ['3.15']" in plan.content
