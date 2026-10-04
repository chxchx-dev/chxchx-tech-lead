import json
import os
from pathlib import Path

from chxchx_tech_lead.workspace.layouts import agents_tab_layout, header_text, workspace_layout
from chxchx_tech_lead.workspace.models import AgentConfig, HeaderConfig


def test_header_uses_project_profile_and_custom_label(tmp_path: Path):
    config = HeaderConfig(label="backend", logo="◆", template="{logo} [{label}] {project} / {profile}")

    assert header_text(config, "bokana", "dotnet") == "◆ [backend] bokana / dotnet"
    layout = workspace_layout(
        tmp_path,
        config,
        "bokana",
        "dotnet",
        agents=(AgentConfig("codex", ["codex"]), AgentConfig("claude", ["claude"])),
    )

    assert layout is not None
    assert "chxchx_tech_lead.workspace.header" in layout
    assert "[backend] bokana / dotnet" in layout
    assert "size=2" in layout
    assert f"cwd={json.dumps(str(tmp_path.resolve()), ensure_ascii=False)}" in layout
    assert 'split_direction="vertical"' in layout
    assert 'name="codex"' in layout
    assert 'name="claude"' in layout
    assert "command=" in layout
    assert 'tab name="Agentes" focus=true {' in layout
    assert 'tab name="Agentes" focus=true split_direction=' not in layout
    assert f'"--root" {json.dumps(str(tmp_path.resolve()), ensure_ascii=False)}' in layout
    assert "\\\"" not in layout
    if os.name == "nt":
        shell = os.environ.get("COMSPEC", "cmd.exe")
        assert f"command={json.dumps(shell, ensure_ascii=False)} {{" in layout
        assert 'args "-c"' not in layout
        assert "stty sane" not in layout
    else:
        assert 'args "-c"' in layout
        assert "-i" in layout
        assert "-l -i" not in layout
        assert "stty sane" in layout
        assert "exec" in layout


def test_header_can_be_disabled(tmp_path: Path):
    assert workspace_layout(tmp_path, HeaderConfig(enabled=False), "demo", "python") is None


def test_vertical_orientation_stacks_agent_panes(tmp_path: Path):
    layout = workspace_layout(
        tmp_path,
        HeaderConfig(),
        "demo",
        "python",
        orientation="vertical",
        agents=(AgentConfig("codex", ["codex"]), AgentConfig("claude", ["claude"])),
    )

    assert layout is not None
    assert 'tab name="Agentes" focus=true {' in layout
    assert 'pane split_direction="horizontal" {' in layout
    assert "command=" in layout


def test_added_agents_tab_stacks_panels_and_uses_plain_project_path(tmp_path: Path):
    layout = agents_tab_layout(
        tmp_path,
        HeaderConfig(),
        "demo",
        "python",
        "horizontal",
        (AgentConfig("codex", ["codex"]),),
    )

    assert layout.startswith("layout {")
    assert 'tab name="Agentes"' not in layout
    assert f'"--root" {json.dumps(str(tmp_path.resolve()), ensure_ascii=False)}' in layout
    assert "\\\"" not in layout


def test_agent_pane_command_omits_empty_brand_arguments():
    from chxchx_tech_lead.workspace.agent_commands import agent_pane_command

    command = agent_pane_command("codex", ["codex"], label="", logo="", new_chat=True)

    assert all(part for part in command)
    assert "--label" not in command
    assert "--logo" not in command
    assert command[-2] == "codex"
    assert "Basic Memory" in command[-1]
