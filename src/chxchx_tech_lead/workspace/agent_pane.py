"""Visible launcher used inside agent panes."""

from __future__ import annotations

import os
import json
import shlex
import subprocess
import sys
from pathlib import Path

from ..integrations.agent_usage import usage_snapshot_path
from ..integrations.explicit_memory import (
    format_recovered_memories,
    persist_explicit_memories,
    recover_explicit_memories,
)


def banner(name: str, label: str = "CHXCHX TECH", logo: str = "") -> str:
    title = name.upper()
    identity = " ".join(part for part in (logo, label) if part).strip()
    caption = f"{identity} · {title}"
    return "\n".join(
        (
            "╭──────────────────────────────╮",
            f"│  {caption:<28} │",
            "╰──────────────────────────────╯",
        )
    )


def main() -> int:
    args = sys.argv[1:]
    try:
        name_index = args.index("--name")
        name = args[name_index + 1]
        label = args[args.index("--label") + 1] if "--label" in args else "CHXCHX TECH"
        logo = args[args.index("--logo") + 1] if "--logo" in args else ""
        recover_memory = "--recover-explicit-memory" in args
        capture_memory = "--capture-explicit-memory" in args
        project_root = Path(args[args.index("--project-root") + 1]).resolve() if "--project-root" in args else None
        separator = args.index("--", name_index + 2)
    except (ValueError, IndexError):
        print("Uso: agent_pane --name ID -- COMANDO [ARGUMENTOS...]", file=sys.stderr)
        return 2
    command = args[separator + 1 :]
    if not command:
        print("El comando del agente está vacío", file=sys.stderr)
        return 2
    if recover_memory and project_root is not None:
        try:
            memories = recover_explicit_memories(project_root)
            persist_explicit_memories(project_root, memories)
            if memories and command:
                command[-1] += format_recovered_memories(memories)
            if memories:
                print(f"Memoria explícita recuperada: {len(memories)} solicitud(es) del proyecto.", flush=True)
        except OSError as exc:
            print(f"No pude recuperar la memoria local del proyecto: {exc}", file=sys.stderr, flush=True)
    print(f"\033[1;36m{banner(name, label, logo)}\033[0m", flush=True)
    print(flush=True)
    command, environment = _usage_instrumentation(command, name)
    completed = subprocess.run(command, cwd=os.getcwd(), env=environment, check=False)
    if capture_memory and project_root is not None:
        try:
            memories = recover_explicit_memories(project_root)
            persist_explicit_memories(project_root, memories)
        except OSError as exc:
            print(f"No pude guardar la memoria local del proyecto: {exc}", file=sys.stderr, flush=True)
    return completed.returncode


def _usage_instrumentation(command: list[str], name: str) -> tuple[list[str], dict[str, str]]:
    executable = Path(command[0].replace("\\", "/")).name.casefold()
    for suffix in (".exe", ".cmd", ".bat"):
        executable = executable.removesuffix(suffix)
    if executable == "codex":
        status_items = 'tui.status_line=["model","context-remaining","rate-limits"]'
        return [command[0], "--config", status_items, *command[1:]], os.environ.copy()
    if executable != "claude":
        return command, os.environ.copy()
    if "--settings" in command:
        return command, os.environ.copy()

    snapshot = usage_snapshot_path(Path.cwd(), name)
    statusline_command = shlex.join(
        [sys.executable, "-m", "chxchx_tech_lead.workspace.usage_statusline"]
    )
    settings = json.dumps(
        {
            "statusLine": {
                "type": "command",
                "command": statusline_command,
                "refreshInterval": 3,
            }
        },
        separators=(",", ":"),
    )
    environment = os.environ.copy()
    environment["CHXCHX_USAGE_SNAPSHOT"] = str(snapshot)
    return [command[0], "--settings", settings, *command[1:]], environment


if __name__ == "__main__":
    raise SystemExit(main())
