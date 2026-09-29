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
    """Return a login shell that keeps the main workspace pane alive."""
    if os.name == "nt":
        return os.environ.get("COMSPEC", "cmd.exe"), ()
    # Explicitly request interactive mode and repair the PTY before starting
    # the user's shell. A pane created from a suspended TUI or a resurrected
    # Zellij session can inherit ``-echo``/raw terminal flags, which makes
    # commands execute without showing what the user types.
    user_shell = os.environ.get("SHELL", "sh")
    bootstrap = f"stty sane 2>/dev/null || true; exec {shlex.quote(user_shell)} -l -i"
    return "/bin/sh", ("-lc", bootstrap)


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


def workspace_layout(
    root: Path,
    header: HeaderConfig,
    project: str,
    profile: str,
    orientation: str = "horizontal",
    agents: Sequence[AgentConfig] = (),
) -> str | None:
    if not header.enabled and not agents:
        return None
    cwd = json.dumps(str(root.resolve()), ensure_ascii=False)
    lines = ["layout {"]
    if header.enabled:
        text = header_text(header, project, profile)
        command = json.dumps(sys.executable, ensure_ascii=False)
        args = " ".join(
            json.dumps(value, ensure_ascii=False)
            for value in (
                "-m",
                "chxchx_tech_lead.workspace.header",
                "--text",
                text,
            )
        )
        lines.extend(
            [
                f"  pane size=2 borderless=true cwd={cwd} command={command} {{",
                f"    args {args};",
                "  }",
            ]
        )

    configured_agents = [
        agent for agent in agents if isinstance(agent.command, list) and agent.command
    ]
    if configured_agents:
        usage_args = [
            "-m",
            "chxchx_tech_lead.workspace.usage_panel",
            "--root",
            str(root.resolve()),
        ]
        for agent in configured_agents:
            executable = Path(agent.command[0].replace("\\", "/")).name.casefold()
            provider = executable.removesuffix(".exe")
            if provider not in {"codex", "claude"}:
                provider = "other"
            usage_args.extend(["--agent", f"{agent.id}={provider}"])
        usage_command = json.dumps(sys.executable, ensure_ascii=False)
        usage_command_args = " ".join(json.dumps(value, ensure_ascii=False) for value in usage_args)
        lines.extend(
            [
                f"  pane size=8 borderless=true cwd={cwd} command={usage_command} {{",
                f"    args {usage_command_args};",
                "  }",
            ]
        )

    if configured_agents:
        split_direction = "vertical" if orientation == "horizontal" else "horizontal"
        lines.append(f'  pane split_direction={json.dumps(split_direction)} {{')
        for agent in configured_agents:
            agent_cwd = json.dumps(str((root / agent.cwd).resolve()), ensure_ascii=False)
            command_parts = agent_pane_command(
                agent.id,
                agent.command,
                label=header.label,
                logo=header.logo,
            )
            executable_name = json.dumps(command_parts[0], ensure_ascii=False)
            args = " ".join(
                json.dumps(value, ensure_ascii=False) for value in command_parts[1:]
            )
            lines.extend(
                [
                    f"    pane name={json.dumps(agent.id)} cwd={agent_cwd} command={executable_name} {{",
                    f"      args {args};",
                    "    }",
                ]
            )
        lines.extend(_shell_pane(cwd, indent="    "))
        lines.append("  }")
    else:
        lines.extend(_shell_pane(cwd))
    lines.extend(["}", ""])
    return "\n".join(lines)
