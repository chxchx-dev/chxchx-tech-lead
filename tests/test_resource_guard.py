from __future__ import annotations

from pathlib import Path
from types import SimpleNamespace

from typer.testing import CliRunner

from chxchx_tech_lead.cli import app
from chxchx_tech_lead.workspace.models import ResourceConfig
from chxchx_tech_lead.workspace.resources import ResourceManager, SystemResources


def test_agent_cli_requires_force_for_resource_budget_override(tmp_path: Path, monkeypatch):
    class ServiceSpy:
        start_calls = 0

        def inspect(self):
            return SimpleNamespace(
                config=SimpleNamespace(
                    resources=ResourceConfig(max_agents=2),
                    agents=[SimpleNamespace(id=name) for name in ("codex", "claude")],
                    presets={},
                )
            )

        def agent_statuses(self, *, probe_versions=False):
            return []

        def start_agents(self, **_kwargs):
            self.start_calls += 1
            return [("codex", SimpleNamespace(command=["codex"])),
                    ("claude", SimpleNamespace(command=["claude"]))]

    service = ServiceSpy()
    monkeypatch.setattr(
        "chxchx_tech_lead.commands.agents._workspace_service",
        lambda _path: service,
    )
    monkeypatch.setattr(
        ResourceManager,
        "system",
        lambda _self: SystemResources(100, 95, 5),
    )
    runner = CliRunner()

    blocked = runner.invoke(app, ["agent", "start", "--all", "--path", str(tmp_path)])
    assert service.start_calls == 0
    forced = runner.invoke(
        app, ["agent", "start", "--all", "--path", str(tmp_path), "--force"]
    )

    assert blocked.exit_code == 2
    assert "RAM Governor" in blocked.stdout
    assert forced.exit_code == 0, forced.stdout
    assert "--force confirma" in forced.stdout
    assert service.start_calls == 1


def test_studio_agent_preflight_requires_current_project_trust(tmp_path: Path, monkeypatch):
    service = SimpleNamespace(
        inspect=lambda: SimpleNamespace(trusted=False, config=None),
    )
    monkeypatch.setattr(
        "chxchx_tech_lead.commands.agents._workspace_service",
        lambda _path: service,
    )

    result = CliRunner().invoke(app, [
        "agent", "preflight", "--agent", "codex", "--path", str(tmp_path)
    ])

    assert result.exit_code == 1
    assert "no tiene trust" in result.stdout


def test_studio_agent_preflight_includes_existing_embedded_sessions(tmp_path: Path, monkeypatch):
    service = SimpleNamespace(
        inspect=lambda: SimpleNamespace(
            trusted=True,
            config=SimpleNamespace(
                resources=ResourceConfig(max_agents=2),
                agents=[SimpleNamespace(id="codex", shell=False)],
                presets={},
            ),
        ),
        agent_statuses=lambda *, probe_versions=False: [],
    )
    monkeypatch.setattr(
        "chxchx_tech_lead.commands.agents._workspace_service",
        lambda _path: service,
    )
    monkeypatch.setattr(
        ResourceManager,
        "system",
        lambda _self: SystemResources(100, 40, 60),
    )

    result = CliRunner().invoke(app, [
        "agent", "preflight", "--agent", "codex", "--path", str(tmp_path),
        "--additional-active", "2",
    ])

    assert result.exit_code == 0, result.stdout
    assert "RAM Governor" in result.stdout
    assert "No se inició ningún proceso" in result.stdout
