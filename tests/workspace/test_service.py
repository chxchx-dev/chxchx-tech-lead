import json
from pathlib import Path
import sys

import pytest

from chxchx_tech_lead.adapters.terminal.zellij import ZellijAdapter
from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.core.registry import register_project
from chxchx_tech_lead.core.runner import CommandResult
from chxchx_tech_lead.core.trust import trust_project
from chxchx_tech_lead.workspace.service import WorkspaceOperationError, WorkspaceService
from chxchx_tech_lead.workspace.models import WorkspaceStatus
from chxchx_tech_lead.workspace.state import load_state, save_state


class FakeTerminal:
    def __init__(self):
        self.calls = []

    def available(self):
        return True

    def session_exists(self, name):
        self.calls.append(("exists", name))
        return False

    def create_session(self, name, cwd, dry_run=False, layout=None):
        self.calls.append(("create", name, cwd, dry_run, layout))
        return CommandResult(["fake-terminal", "create", name], 0, "created", "")

    def attach_session(self, name, dry_run=False):
        self.calls.append(("attach", name, dry_run))
        return CommandResult(["fake-terminal", "attach", name], 0, "attached", "")

    def close_session(self, name, dry_run=False):
        return CommandResult(["fake-terminal", "close", name], 0, "", "")

    def run_in_session(self, name, command, cwd, pane_name=None, direction=None, dry_run=False):
        self.calls.append(("run", name, list(command), cwd, pane_name, direction, dry_run))
        return CommandResult(["fake-terminal", "run", *command], 0, "started", "")


class FakeEditor:
    def open_project(self, path, dry_run=False):
        return CommandResult(["subl", str(path)], 0, "opened", "")


def _write_config(project: Path, auto_start: bool = True):
    config = project / ".ai" / "chxchx-tech.toml"
    config.parent.mkdir()
    config.write_text(
        f'''version = 2
profile = "python"
memory_project = "demo-123456"
memory_path = ".ai/memory"

[workspace]
name = "demo workspace"
adapter = "zellij"
editor = "sublime"
auto_open_editor = false
auto_attach = false
auto_start = {str(auto_start).lower()}

[workspace.resources]
warn_memory_percent = 75
critical_memory_percent = 90
warn_swap_percent = 40

[workspace.docker]
enabled = true
compose_file = "missing-compose.yaml"
auto_start = true

[[workspace.processes]]
id = "api"
label = "API"
command = [{json.dumps(sys.executable)}, "-c", "pass"]
cwd = "."
auto_start = true
restart = "never"

[[workspace.agents]]
id = "codex"
command = ["codex"]
cwd = "."
auto_start = false
''',
        encoding="utf-8",
    )


def test_untrusted_workspace_never_starts_auto_processes(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project)
    terminal = FakeTerminal()

    action = WorkspaceService(ProjectInfo(project, "project"), terminal=terminal, editor=FakeEditor()).open()

    assert action.process_results == []
    assert any(call[0] == "create" for call in terminal.calls)


def test_stop_ignores_legacy_docker_settings(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)

    action = WorkspaceService(ProjectInfo(project, "project"), terminal=FakeTerminal()).stop()

    assert action.messages == []


def test_start_ignores_legacy_docker_settings(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    trust_project(project)

    action = WorkspaceService(ProjectInfo(project, "project"), terminal=FakeTerminal()).start()

    assert "Docker Compose iniciado" not in action.messages


def test_trusted_workspace_starts_auto_process_in_dry_run(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project)
    trust_project(project)
    terminal = FakeTerminal()

    action = WorkspaceService(ProjectInfo(project, "project"), terminal=terminal, editor=FakeEditor()).start(dry_run=True)

    assert len(action.process_results) == 1
    assert action.process_results[0].process.id == "api"
    assert action.process_results[0].message.startswith("DRY RUN")


def test_service_fails_if_zellij_does_not_recognize_created_session(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        if command[-1] == "list-sessions":
            return CommandResult(list(command), 0, "", "")
        return CommandResult(list(command), 0, "created", "")

    terminal = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")

    with pytest.raises(WorkspaceOperationError, match="no la reconoce"):
        WorkspaceService(ProjectInfo(project, "project"), terminal=terminal, editor=FakeEditor()).open()

    assert any(command[-2:] == ["--create-background", "demo-workspace"] for command, _kwargs in calls)


def test_attach_fails_fast_when_zellij_session_is_missing(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)

    def fake_runner(command, **kwargs):
        return CommandResult(list(command), 0, "", "")

    terminal = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")

    with pytest.raises(WorkspaceOperationError, match="no la reconoce"):
        WorkspaceService(ProjectInfo(project, "project"), terminal=terminal, editor=FakeEditor()).attach()


def test_service_can_start_one_configured_process(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    trust_project(project)

    result = WorkspaceService(
        ProjectInfo(project, "project"), terminal=FakeTerminal(), editor=FakeEditor()
    ).start_process("api", dry_run=True)

    assert result.process.id == "api"
    assert result.message.startswith("DRY RUN")


def test_agent_start_uses_configured_session_and_cwd(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    trust_project(project)
    terminal = FakeTerminal()

    result = WorkspaceService(ProjectInfo(project, "project"), terminal=terminal, editor=FakeEditor()).start_agent(
        "codex", dry_run=True
    )

    assert result.returncode == 0
    run_calls = [call for call in terminal.calls if call[0] == "run"]
    assert run_calls[0][1] == "demo-workspace"
    assert run_calls[0][2][:4] == [
        sys.executable,
        "-m",
        "chxchx_tech_lead.workspace.agent_pane",
        "--name",
    ]
    assert run_calls[0][2][-2:] == ["--", "codex"]
    assert "--label" in run_calls[0][2]
    assert "--logo" not in run_calls[0][2]
    assert all(run_calls[0][2])
    assert run_calls[0][3] == project.resolve()

def test_attach_agent_focuses_and_opens_named_zellij_pane(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        if command == ["zellij", "list-sessions"]:
            return CommandResult(list(command), 0, "demo-workspace\n", "")
        if command[-1] == "--json":
            return CommandResult(
                list(command),
                0,
                '[{"pane_id":"codex_3","pane_name":"codex"}]',
                "",
            )
        return CommandResult(list(command), 0, "attached", "")

    terminal = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")
    result = WorkspaceService(
        ProjectInfo(project, "project"), terminal=terminal, editor=FakeEditor()
    ).attach_agent("codex")

    assert result.returncode == 0
    assert any(call[0][-2:] == ["focus-pane-id", "codex_3"] for call in calls)
    assert calls[-1][0] == ["zellij", "attach", "--force-run-commands", "demo-workspace"]
    assert calls[-1][1]["interactive"] is True


def test_agent_new_chat_passes_context_bootstrap_prompt(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    trust_project(project)
    terminal = FakeTerminal()

    WorkspaceService(ProjectInfo(project, "project"), terminal=terminal, editor=FakeEditor()).start_agent(
        "codex", dry_run=True, new_chat=True
    )

    command = next(call[2] for call in terminal.calls if call[0] == "run")
    assert command[-2] == "codex"
    assert "conversación nueva" in command[-1]
    assert ".ai/CURRENT_STATE.md" in command[-1]
    assert "Basic Memory" in command[-1]


def test_new_chat_rejects_unrecognized_agent_cli(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    config = project / ".ai" / "chxchx-tech.toml"
    content = config.read_text(encoding="utf-8").replace('id = "codex"\ncommand = ["codex"]', 'id = "other"\ncommand = ["opencode"]')
    config.write_text(content, encoding="utf-8")
    trust_project(project)

    with pytest.raises(WorkspaceOperationError, match="solo está configurado para Codex y Claude"):
        WorkspaceService(ProjectInfo(project, "project"), terminal=FakeTerminal(), editor=FakeEditor()).start_agent(
            "other", dry_run=True, new_chat=True
        )


def test_start_agents_starts_all_configured_agents(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    config = project / ".ai" / "chxchx-tech.toml"
    config.write_text(
        config.read_text(encoding="utf-8")
        + '\n[[workspace.agents]]\nid = "claude"\ncommand = ["claude"]\ncwd = "."\nauto_start = false\n',
        encoding="utf-8",
    )
    trust_project(project)
    terminal = FakeTerminal()

    results = WorkspaceService(ProjectInfo(project, "project"), terminal=terminal, editor=FakeEditor()).start_agents(
        dry_run=True
    )

    assert [agent_id for agent_id, _result in results] == ["codex", "claude"]


def test_agent_preset_starts_only_selected_agents(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    config = project / ".ai" / "chxchx-tech.toml"
    config.write_text(
        config.read_text(encoding="utf-8")
        + '\n[[workspace.agents]]\nid = "claude"\ncommand = ["claude"]\ncwd = "."\nauto_start = false\n'
        + '\n[workspace.presets]\nsolo = ["codex"]\n',
        encoding="utf-8",
    )
    trust_project(project)
    terminal = FakeTerminal()

    results = WorkspaceService(ProjectInfo(project, "project"), terminal=terminal, editor=FakeEditor()).start_preset(
        "solo", dry_run=True
    )

    assert [agent_id for agent_id, _result in results] == ["codex"]


def test_start_suspends_other_active_registered_workspaces(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    other = tmp_path / "other"
    other.mkdir()
    register_project(ProjectInfo(other, "other"))
    other_state = load_state(other)
    other_state.status = WorkspaceStatus.ACTIVE
    save_state(other_state)
    trust_project(project)

    action = WorkspaceService(
        ProjectInfo(project, "project"), terminal=FakeTerminal(), editor=FakeEditor()
    ).start(dry_run=True)

    assert any("Workspace suspendido" in message for message in action.messages)
    assert load_state(other).status is WorkspaceStatus.ACTIVE


def test_run_all_composes_start_agents_and_attach(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    project = tmp_path / "project"
    project.mkdir()
    _write_config(project, auto_start=False)
    config = project / ".ai" / "chxchx-tech.toml"
    config.write_text(
        config.read_text(encoding="utf-8")
        + '\n[[workspace.agents]]\nid = "claude"\ncommand = ["claude"]\ncwd = "."\nauto_start = false\n',
        encoding="utf-8",
    )
    trust_project(project)
    terminal = FakeTerminal()

    action, agents, attached = WorkspaceService(
        ProjectInfo(project, "project"), terminal=terminal, editor=FakeEditor()
    ).run_all(dry_run=True, attach=True)

    assert action.messages
    assert agents == []
    assert attached is not None
    create_calls = [call for call in terminal.calls if call[0] == "create"]
    assert len(create_calls) == 1
    assert 'name="codex"' in create_calls[0][4]
    assert 'name="claude"' in create_calls[0][4]
    assert not [call for call in terminal.calls if call[0] == "run"]
    assert any(call[0] == "attach" for call in terminal.calls)
