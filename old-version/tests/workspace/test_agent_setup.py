from __future__ import annotations

import tomllib
from pathlib import Path

import pytest

from chxchx_tech_lead.workspace.agent_setup import AgentSetupError, add_agent


CONFIG = '''version = 2
memory_project = "project-memory"

[workspace]
name = "demo"

[workspace.presets]
default = ["codex"]

[[workspace.agents]]
id = "codex"
command = ["codex"]
cwd = "."
auto_start = false
'''


def _config_path(root: Path) -> Path:
    target = root / ".ai" / "chxchx-tech.toml"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(CONFIG, encoding="utf-8")
    return target


def test_agent_add_dry_run_shows_config_without_writing_or_backup(tmp_path: Path, monkeypatch):
    path = _config_path(tmp_path)
    original = path.read_bytes()
    monkeypatch.setattr(
        "chxchx_tech_lead.workspace.agent_setup.backup_project",
        lambda _root: pytest.fail("dry-run no debe crear backups"),
    )

    result = add_agent(tmp_path, "ollama-opencode", dry_run=True)

    assert result.dry_run and result.changed
    assert 'command = ["ollama", "launch", "opencode"]' in result.preview
    assert path.read_bytes() == original


def test_agent_add_appends_once_and_backups_before_writing(tmp_path: Path, monkeypatch):
    path = _config_path(tmp_path)
    original = path.read_bytes()
    backup = tmp_path / "backup"
    backup.mkdir()
    backup_calls: list[Path] = []

    def create_backup(root: Path) -> Path:
        backup_calls.append(root)
        (backup / path.name).write_bytes(path.read_bytes())
        return backup

    monkeypatch.setattr("chxchx_tech_lead.workspace.agent_setup.backup_project", create_backup)

    result = add_agent(tmp_path, "opencode")
    parsed = tomllib.loads(path.read_text(encoding="utf-8"))
    repeated = add_agent(tmp_path, "opencode")

    assert result.changed and result.backup_path == backup
    assert backup_calls == [tmp_path.resolve()]
    assert (backup / path.name).read_bytes() == original
    assert path.read_bytes().startswith(original)
    assert parsed["workspace"]["agents"][-1]["command"] == ["opencode"]
    assert parsed["workspace"]["presets"]["default"] == ["codex"]
    assert not repeated.changed
    assert "ya configurado" in repeated.preview
    assert backup_calls == [tmp_path.resolve()]


def test_agent_add_rejects_conflicting_id_and_unknown_provider(tmp_path: Path):
    path = _config_path(tmp_path)
    contents = path.read_text(encoding="utf-8").replace('id = "codex"', 'id = "opencode"')
    contents = contents.replace('default = ["codex"]', 'default = ["opencode"]')
    path.write_text(contents, encoding="utf-8")

    with pytest.raises(AgentSetupError, match="otro comando"):
        add_agent(tmp_path, "opencode", dry_run=True)
    with pytest.raises(AgentSetupError, match="Opciones"):
        add_agent(tmp_path, "unknown", dry_run=True)


def test_agent_add_requires_existing_v2_config(tmp_path: Path):
    with pytest.raises(AgentSetupError, match="init"):
        add_agent(tmp_path, "opencode", dry_run=True)
