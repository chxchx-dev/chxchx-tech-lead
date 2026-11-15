from __future__ import annotations

import json
import os
import shlex
import sys
from collections.abc import Sequence
from pathlib import Path

from .agent_commands import agent_pane_command
from .models import AgentConfig, HeaderConfig


def header_text(header: HeaderConfig, project: str, profile: str) -> str:
    return header.template.format(
        logo=header.logo,
        label=header.label,
        project=project,
        profile=profile,
    ).strip()


def _shell_command() -> tuple[str, tuple[str, ...]]:
    """Return an interactive shell that keeps the main workspace pane alive."""
    if os.name == "nt":
        return os.environ.get("COMSPEC", "cmd.exe"), ()
    # Explicitly request interactive mode and repair the PTY before starting
    # the user's shell. A pane created from a suspended TUI or a resurrected
    # Zellij session can inherit ``-echo``/raw terminal flags, which makes
    # commands execute without showing what the user types.
    user_shell = os.environ.get("SHELL", "sh")
    # The TUI inherits the user's login environment already. Starting another
    # login shell here reloads profiles such as macOS ~/.zprofile in the pane,
    # which can print errors over the fresh workspace terminal.
    bootstrap = f"stty sane 2>/dev/null || true; exec {shlex.quote(user_shell)} -i"
    return "/bin/sh", ("-c", bootstrap)


def _shell_pane(cwd: str, *, indent: str = "  ", focus: bool = True) -> list[str]:
    shell, args = _shell_command()
    focus_value = " focus=true" if focus else ""
    lines = [
        f'{indent}pane name="terminal"{focus_value} cwd={cwd} '
        f'command={json.dumps(shell, ensure_ascii=False)} {{'
    ]
    if args:
        lines.append(f'{indent}  args {" ".join(json.dumps(value) for value in args)};')
    lines.append(f"{indent}}}")
    return lines


def _header_pane(root: str, header: HeaderConfig, project: str, profile: str) -> list[str]:
    if not header.enabled:
        return []
    text = header_text(header, project, profile)
    command = json.dumps(sys.executable, ensure_ascii=False)
    args = " ".join(
        json.dumps(value, ensure_ascii=False)
        for value in ("-m", "chxchx_tech_lead.workspace.header", "--text", text)
    )
    return [
        f"    pane size=2 borderless=true cwd={root} command={command} {{",
        f"      args {args};",
        "    }",
    ]


def _usage_pane(root: str, agents: Sequence[AgentConfig]) -> list[str]:
    cwd = json.dumps(root, ensure_ascii=False)
    usage_args = ["-m", "chxchx_tech_lead.workspace.usage_panel", "--root", root]
    for agent in agents:
        executable = Path(agent.command[0].replace("\\", "/")).name.casefold()
        provider = executable.removesuffix(".exe")
        usage_args.extend(["--agent", f"{agent.id}={provider if provider in {'codex', 'claude'} else 'other'}"])
    command = json.dumps(sys.executable, ensure_ascii=False)
    args = " ".join(json.dumps(value, ensure_ascii=False) for value in usage_args)
    return [
        f"    pane size=8 borderless=true cwd={cwd} command={command} {{",
        f"      args {args};",
        "    }",
    ]


def _agent_panes(
    root: Path, cwd: str, header: HeaderConfig, agents: Sequence[AgentConfig], orientation: str
) -> list[str]:
    direction = "vertical" if orientation == "horizontal" else "horizontal"
    indent = "      " if len(agents) > 1 else "    "
    lines = [f'    pane split_direction={json.dumps(direction)} {{'] if len(agents) > 1 else []
    for agent in agents:
        agent_cwd = json.dumps(str((root / agent.cwd).resolve()), ensure_ascii=False)
        command = agent_pane_command(
            agent.id,
            agent.command,
            label=header.label,
            logo=header.logo,
            project_root=str(root.resolve()),
        )
        executable = json.dumps(command[0], ensure_ascii=False)
        args = " ".join(json.dumps(value, ensure_ascii=False) for value in command[1:])
        lines.extend(
            [
                f"{indent}pane name={json.dumps(agent.id)} cwd={agent_cwd} command={executable} {{",
                f"{indent}  args {args};",
                f"{indent}}}",
            ]
        )
    if len(agents) > 1:
        lines.append("    }")
    return lines


def workspace_layout(
    root: Path,
    header: HeaderConfig,
    project: str,
    profile: str,
    orientation: str = "horizontal",
    agents: Sequence[AgentConfig] = (),
) -> str | None:
    configured_agents = [agent for agent in agents if isinstance(agent.command, list) and agent.command]
    if not header.enabled and not configured_agents:
        return None
    root_path = str(root.resolve())
    cwd = json.dumps(root_path, ensure_ascii=False)
    lines = ["layout {", '  tab name="Terminales" {']
    lines.extend(_header_pane(cwd, header, project, profile))
    lines.extend(_shell_pane(cwd))
    lines.append("  }")
    if configured_agents:
        lines.append('  tab name="Agentes" focus=true {')
        lines.extend(_header_pane(cwd, header, project, profile))
        lines.extend(_usage_pane(root_path, configured_agents))
        lines.extend(_agent_panes(root, cwd, header, configured_agents, orientation))
        lines.append("  }")
    lines.extend(["}", ""])
    return "\n".join(lines)


def agents_tab_layout(
    root: Path,
    header: HeaderConfig,
    project: str,
    profile: str,
    orientation: str,
    agents: Sequence[AgentConfig],
) -> str:
    """Return a layout that can add the dedicated agents tab to an active session."""
    root_path = str(root.resolve())
    cwd = json.dumps(root_path, ensure_ascii=False)
    configured = [agent for agent in agents if isinstance(agent.command, list) and agent.command]
    # `action new-tab --layout` accepts a tab layout (pane nodes only). A
    # top-level `tab` node is ignored there and Zellij creates an empty pane.
    lines = ["layout {"]
    lines.extend(_header_pane(cwd, header, project, profile))
    if configured:
        lines.extend(_usage_pane(root_path, configured))
        lines.extend(_agent_panes(root, cwd, header, configured, orientation))
    else:
        lines.extend(_shell_pane(cwd, indent="    "))
    lines.extend(["}", ""])
    return "\n".join(lines)


def terminal_tab_layout(root: Path, header: HeaderConfig, project: str, profile: str) -> str:
    cwd = json.dumps(str(root.resolve()), ensure_ascii=False)
    lines = ['layout {', '  tab name="Terminales" focus=true {']
    lines.extend(_header_pane(cwd, header, project, profile))
    lines.extend(_shell_pane(cwd, indent="    "))
    lines.extend(["  }", "}", ""])
    return "\n".join(lines)
