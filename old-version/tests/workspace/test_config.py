from pathlib import Path

import pytest

from chxchx_tech_lead.core.config_migrations import migrate_project_config
from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.core.project_config import ensure_project_config
from chxchx_tech_lead.workspace.models import WorkspaceConfig, WorkspaceConfigError


def test_new_project_config_is_v2_and_idempotent(tmp_path: Path):
    info = ProjectInfo(tmp_path, "demo")

    first = migrate_project_config(info)
    second = migrate_project_config(info)

    assert first.changed is True
    assert first.source_version is None
    assert second.changed is False
    assert "version = 2" in (tmp_path / ".ai" / "chxchx-tech.toml").read_text(encoding="utf-8")
    assert "max_agents = 2" in first.content
    assert "[workspace.docker]" not in first.content


def test_legacy_docker_config_is_ignored(tmp_path: Path):
    config = WorkspaceConfig.from_mapping(
        {"name": "demo", "docker": {"enabled": True, "compose_file": "missing.yaml", "auto_start": True}},
        project_root=tmp_path,
    )

    assert not hasattr(config, "docker")


def test_v1_config_migrates_without_executing_commands(tmp_path: Path):
    info = ProjectInfo(tmp_path, "demo")
    target = tmp_path / ".ai" / "chxchx-tech.toml"
    target.parent.mkdir()
    target.write_text(
        'version = 1\nprofile = "python"\nmemory_project = "demo-123456"\nmemory_path = ".ai/memory"\n',
        encoding="utf-8",
    )

    result = migrate_project_config(info, dry_run=True)

    assert result.changed is True
    assert result.source_version == 1
    assert target.read_text(encoding="utf-8").startswith("version = 1")
    assert "version = 2" in result.content
    assert "[[workspace.processes]]" not in result.content

    ensure_project_config(info)
    assert "version = 2" in target.read_text(encoding="utf-8")


def test_existing_v2_config_is_validated_but_not_rewritten(tmp_path: Path):
    info = ProjectInfo(tmp_path, "demo")
    target = tmp_path / ".ai" / "chxchx-tech.toml"
    target.parent.mkdir()
    content = (
        'version = 2\nprofile = "python"\nmemory_project = "demo-123456"\n'
        'memory_path = ".ai/memory"\n\n[workspace]\nname = "custom-name"\n'
        'adapter = "subprocess"\neditor = "sublime"\n\n'
        '[workspace.resources]\nwarn_memory_percent = 70\ncritical_memory_percent = 90\nwarn_swap_percent = 40\n\n'
        '[workspace.docker]\nenabled = false\ncompose_file = "compose.yaml"\nauto_start = false\n'
    )
    target.write_text(content, encoding="utf-8")

    result = migrate_project_config(info)

    assert result.changed is False
    assert target.read_text(encoding="utf-8") == content


def test_resource_config_defaults_and_validates_max_agents(tmp_path: Path):
    config = WorkspaceConfig.from_mapping({"name": "demo"}, project_root=tmp_path)
    limited = WorkspaceConfig.from_mapping(
        {"name": "demo", "resources": {"max_agents": 3}}, project_root=tmp_path
    )

    assert config.resources.warn_memory_percent == 70
    assert config.resources.critical_memory_percent == 85
    assert config.resources.max_agents == 2
    assert limited.resources.max_agents == 3
    with pytest.raises(WorkspaceConfigError, match="max_agents"):
        WorkspaceConfig.from_mapping(
            {"name": "demo", "resources": {"max_agents": 0}}, project_root=tmp_path
        )


@pytest.mark.parametrize(
    "cwd",
    ["/tmp/outside", "../outside", r"C:\outside", r"..\outside"],
)
def test_workspace_rejects_cwd_outside_project(tmp_path: Path, cwd: str):
    with pytest.raises(WorkspaceConfigError, match="ruta relativa"):
        WorkspaceConfig.from_mapping(
            {"name": "demo", "processes": [{"id": "api", "command": ["python"], "cwd": cwd}]},
            project_root=tmp_path,
        )


def test_workspace_rejects_duplicate_process_and_agent_ids(tmp_path: Path):
    with pytest.raises(WorkspaceConfigError, match="IDs duplicados"):
        WorkspaceConfig.from_mapping(
            {
                "name": "demo",
                "processes": [{"id": "api", "command": ["python"]}],
                "agents": [{"id": "api", "command": ["codex"]}],
            },
            project_root=tmp_path,
        )


def test_shell_command_requires_explicit_shell_flag(tmp_path: Path):
    with pytest.raises(WorkspaceConfigError, match="lista de argumentos"):
        WorkspaceConfig.from_mapping(
            {"name": "demo", "processes": [{"id": "api", "command": "python -m http.server"}]},
            project_root=tmp_path,
        )

    config = WorkspaceConfig.from_mapping(
        {
            "name": "demo",
            "processes": [
                {"id": "api", "command": "python -m http.server", "shell": True}
            ],
        },
        project_root=tmp_path,
    )
    assert config.processes[0].shell is True


def test_workspace_parses_agent_presets_and_rejects_unknown_agents(tmp_path: Path):
    config = WorkspaceConfig.from_mapping(
        {
            "name": "demo",
            "agents": [
                {"id": "codex", "command": ["codex"]},
                {"id": "claude", "command": ["claude"]},
            ],
            "presets": {"pair": ["codex", "claude"], "solo": ["codex"]},
        },
        project_root=tmp_path,
    )

    assert [(item.id, item.agents) for item in config.presets] == [
        ("pair", ("codex", "claude")),
        ("solo", ("codex",)),
    ]
    with pytest.raises(WorkspaceConfigError, match="agentes inexistentes"):
        WorkspaceConfig.from_mapping(
            {"name": "demo", "agents": [{"id": "codex", "command": ["codex"]}], "presets": {"bad": ["claude"]}},
            project_root=tmp_path,
        )
