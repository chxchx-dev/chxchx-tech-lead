"""Visible launcher used inside agent panes."""

from __future__ import annotations

import os
import json
import shlex
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

from ..integrations.agent_usage import usage_snapshot_path
from ..integrations.executable_search import find_executable, with_search_path
from .chat_context import capture_after_session, prepare_new_chat

_JOURNAL_PROVIDERS = {"codex": "Codex", "claude": "Claude", "opencode": "OpenCode"}


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

    opencode_executable = _opencode_executable(command)
    if recover_memory and project_root is not None:
        command[-1] += prepare_new_chat(project_root, _print_notice, opencode_executable=opencode_executable)
    provider = _JOURNAL_PROVIDERS.get(_executable_name(command[0]))
    started_at = datetime.now(timezone.utc)
    print(f"\033[1;36m{banner(name, label, logo)}\033[0m", flush=True)
    print(flush=True)
    command, environment = _usage_instrumentation(command, name)
    environment = with_search_path(environment)  # a Studio started from a launcher has a minimal PATH
    command[0] = find_executable(command[0]) or command[0]
    try:
        completed = subprocess.run(command, cwd=os.getcwd(), env=environment, check=False)
    except OSError as exc:
        print(f"No pude iniciar `{command[0]}`: {exc}. ¿Está instalado y en el PATH?", file=sys.stderr, flush=True)
        return 127
    if capture_memory and project_root is not None:
        capture_after_session(
            project_root,
            _print_notice,
            provider=provider,
            started_at=started_at,
            opencode_executable=opencode_executable,
        )
    return completed.returncode


def _print_notice(message: str, is_error: bool) -> None:
    print(message, file=sys.stderr if is_error else sys.stdout, flush=True)


def _executable_name(program: str) -> str:
    executable = Path(program.replace("\\", "/")).name.casefold()
    for suffix in (".exe", ".cmd", ".bat"):
        executable = executable.removesuffix(suffix)
    return executable


def _opencode_executable(command: list[str]) -> str | None:
    if not command:
        return None
    return command[0] if _executable_name(command[0]) == "opencode" else None


def _usage_instrumentation(command: list[str], name: str) -> tuple[list[str], dict[str, str]]:
    executable = _executable_name(command[0])
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
