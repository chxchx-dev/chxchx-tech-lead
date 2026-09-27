import json
import os
from pathlib import Path

from chxchx_tech_lead.workspace.layouts import header_text, workspace_layout
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
    if os.name == "nt":
        shell = os.environ.get("COMSPEC", "cmd.exe")
        assert f"command={json.dumps(shell, ensure_ascii=False)} {{" in layout
        assert 'args "-lc"' not in layout
        assert "stty sane" not in layout
    else:
        assert 'args "-lc"' in layout
        assert "stty sane" in layout
        assert "exec" in layout


def test_header_can_be_disabled(tmp_path: Path):
    assert workspace_layout(tmp_path, HeaderConfig(enabled=False), "demo", "python") is None


def test_vertical_orientation_stacks_the_main_panes(tmp_path: Path):
    layout = workspace_layout(
        tmp_path,
        HeaderConfig(),
        "demo",
        "python",
        orientation="vertical",
        agents=(AgentConfig("codex", ["codex"]),),
    )

    assert layout is not None
    assert 'split_direction="horizontal"' in layout
    assert "command=" in layout
