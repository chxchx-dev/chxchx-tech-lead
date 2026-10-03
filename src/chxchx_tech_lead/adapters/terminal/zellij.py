from __future__ import annotations

import json
import os
import re
from pathlib import Path
from typing import Callable, Sequence

from ...core.runner import CommandResult, executable, run
from .zellij_layouts import ZellijLayoutOperations


class ZellijAdapter(ZellijLayoutOperations):
    """Controla sesiones Zellij sin construir comandos fuera del adapter."""

    _ANSI_ESCAPE = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")

    def __init__(
        self,
        command: str = "zellij",
        runner: Callable[..., CommandResult] = run,
        lookup: Callable[[str], str | None] = executable,
    ):
        self.command = command
        self._runner = runner
        self._lookup = lookup

    def available(self) -> bool:
        return self._lookup(self.command) is not None

    def list_sessions(self, dry_run: bool = False) -> CommandResult:
        return self._runner(
            [self.command, "list-sessions"],
            dry_run=dry_run,
            timeout=3,
        )

    def session_exists(self, name: str) -> bool:
        return self.session_status(name) == "active"

    def session_status(self, name: str) -> str:
        """Return ``active``, ``exited`` or ``missing`` for a Zellij session."""
        if not self.available():
            return "missing"
        result = self.list_sessions()
        if result.returncode != 0:
            return "missing"
        for line in result.stdout.splitlines():
            if self._session_name(line) != name:
                continue
            return "exited" if "EXITED" in line.upper() else "active"
        return "missing"

    def create_session(
        self,
        name: str,
        cwd: Path,
        dry_run: bool = False,
        layout: str | None = None,
    ) -> CommandResult:
        target = cwd.expanduser().resolve()
        if not target.is_dir():
            return CommandResult(
                [self.command, "attach", "--create-background", name],
                2,
                "",
                f"No existe el cwd de la sesión: {target}",
            )
        if self.session_status(name) == "exited":
            delete_command = [self.command, "delete-session", "--force", name]
            removed = self._runner(delete_command, dry_run=dry_run)
            if removed.returncode != 0 and not dry_run:
                return removed
            if not dry_run and self.session_status(name) != "missing":
                return CommandResult(
                    delete_command,
                    1,
                    "",
                    f"Zellij no eliminó la sesión EXITED `{name}`; ejecuta `zellij delete-session --force {name}`.",
                )
        if layout:
            command = [self.command, "--layout-string", layout, "attach", "--create-background", name]
        else:
            command = [
                self.command,
                "attach",
                "--create-background",
                name,
                "options",
                "--default-cwd",
                str(target),
            ]
        return self._runner(command, dry_run=dry_run)

    def attach_session(self, name: str, dry_run: bool = False) -> CommandResult:
        return self._runner(
            [self.command, "attach", "--force-run-commands", name],
            dry_run=dry_run,
            interactive=True,
        )

    def list_tabs(self, name: str, dry_run: bool = False) -> CommandResult:
        return self._runner(
            [self.command, "--session", name, "action", "list-tabs", "--json"],
            dry_run=dry_run,
            timeout=3,
        )

    def focus_tab(self, name: str, tab_name: str, dry_run: bool = False) -> CommandResult:
        return self._runner(
            [self.command, "--session", name, "action", "go-to-tab-name", "--create", tab_name],
            dry_run=dry_run,
        )

    def focus_named_pane(self, session: str, pane_name: str, dry_run: bool = False) -> CommandResult:
        """Focus a named agent pane before handing the user's TTY to Zellij."""
        panes = self._runner(
            [self.command, "--session", session, "action", "list-panes", "--all", "--json"],
            dry_run=dry_run,
        )
        pane_id = self._find_named_pane_id(panes.stdout, pane_name.casefold())
        if pane_id is None:
            return CommandResult(
                [self.command, "--session", session, "action", "focus-pane-id"],
                1,
                "",
                f"No encontré la terminal del agente `{pane_name}` en la sesión `{session}`.",
            )
        return self._runner(
            [self.command, "--session", session, "action", "focus-pane-id", pane_id],
            dry_run=dry_run,
        )

    def focus_terminal_pane(self, name: str, dry_run: bool = False) -> CommandResult:
        """Focus the pane named ``terminal`` before handing over the TTY."""
        panes = self._runner(
            [self.command, "--session", name, "action", "list-panes", "--all", "--json"],
            dry_run=dry_run,
        )
        pane_id = self._find_terminal_pane_id(panes.stdout)
        if pane_id is not None:
            return self._runner(
                [self.command, "--session", name, "action", "focus-pane-id", pane_id],
                dry_run=dry_run,
            )
        # Older sessions may not have a named terminal pane. Preserve the
        # previous best effort for those sessions; recreated sessions use the
        # deterministic name above.
        return self.focus_last_pane(name, dry_run=dry_run)

    def close_session(self, name: str, dry_run: bool = False) -> CommandResult:
        return self._runner([self.command, "delete-session", name], dry_run=dry_run)

    def list_panes(self, name: str, dry_run: bool = False) -> CommandResult:
        return self._runner(
            [self.command, "--session", name, "action", "list-panes", "--all"],
            dry_run=dry_run,
            timeout=3,
        )

    def open_terminal_pane(self, name: str, cwd: Path, dry_run: bool = False) -> CommandResult:
        """Open a shell in the dedicated Terminales tab."""
        target = cwd.expanduser().resolve()
        if not target.is_dir():
            return CommandResult([self.command, "--session", name, "action", "new-pane"], 2, "", f"No existe el cwd: {target}")
        focused = self.focus_tab(name, "Terminales", dry_run=dry_run)
        if focused.returncode != 0:
            return focused
        panes = self._runner(
            [self.command, "--session", name, "action", "list-panes", "--all", "--json"]
        )
        pane_name = self._next_terminal_name(panes.stdout)
        return self._runner([
            self.command, "--session", name, "action", "new-pane",
            "--cwd", str(target), "--name", pane_name,
        ], dry_run=dry_run)

    @staticmethod
    def _next_terminal_name(raw: str) -> str:
        try:
            payload = json.loads(raw)
        except (TypeError, json.JSONDecodeError):
            payload = []
        names: set[str] = set()

        def walk(value):
            if isinstance(value, dict):
                candidate = value.get("pane_name") or value.get("name")
                if isinstance(candidate, str):
                    names.add(candidate.casefold())
                for child in value.values():
                    walk(child)
            elif isinstance(value, list):
                for child in value:
                    walk(child)

        walk(payload)
        index = 1
        while f"terminal-{index}" in names:
            index += 1
        return f"terminal-{index}"

    def focus_last_pane(self, name: str, dry_run: bool = False) -> CommandResult:
        """Restore the pane that was focused before dynamic panes were added."""
        return self._runner(
            [self.command, "--session", name, "action", "focus-last-pane"],
            dry_run=dry_run,
        )

    def run_in_session(
        self,
        name: str,
        command: Sequence[str],
        cwd: Path,
        pane_name: str | None = None,
        direction: str | None = None,
        dry_run: bool = False,
    ) -> CommandResult:
        target = cwd.expanduser().resolve()
        if not target.is_dir():
            return CommandResult([self.command, "--session", name, "action", "new-pane"], 2, "", f"No existe el cwd: {target}")
        if not command or not all(str(part) for part in command):
            return CommandResult([], 2, "", "El comando del pane no puede estar vacío")
        args = [self.command, "--session", name, "action", "new-pane", "--cwd", str(target)]
        if direction:
            args.extend(["--direction", direction])
        if pane_name:
            args.extend(["--name", pane_name])
        args.extend(["--", *[str(part) for part in command]])
        return self._runner(args, dry_run=dry_run)

    @staticmethod
    def _session_name(line: str) -> str:
        clean = ZellijAdapter._ANSI_ESCAPE.sub("", line).strip()
        return clean.split(maxsplit=1)[0] if clean else ""

    @staticmethod
    def _find_named_pane_id(raw: str, wanted: str) -> str | None:
        try:
            payload = json.loads(raw)
        except (TypeError, json.JSONDecodeError):
            return None

        def walk(value):
            if isinstance(value, dict):
                name = value.get("pane_name") or value.get("name") or value.get("title")
                pane_id = value.get("pane_id") or value.get("id")
                if str(name).strip().lower() == wanted and pane_id is not None:
                    return str(pane_id)
                for child in value.values():
                    found = walk(child)
                    if found is not None:
                        return found
            elif isinstance(value, list):
                for child in value:
                    found = walk(child)
                    if found is not None:
                        return found
            return None

        return walk(payload)

    @staticmethod
    def _find_terminal_pane_id(raw: str) -> str | None:
        """Find the interactive shell using Zellij's documented JSON fields."""
        named = ZellijAdapter._find_named_pane_id(raw, "terminal")
        if named is not None:
            return named
        try:
            payload = json.loads(raw)
        except (TypeError, json.JSONDecodeError):
            return None

        shell = Path(os.environ.get("SHELL", "sh")).name.lower()
        shell_names = {shell, "sh", "bash", "zsh", "fish", "dash", "ksh", "cmd.exe", "powershell"}

        def walk(value):
            if isinstance(value, dict):
                if not value.get("is_plugin", False):
                    command = str(value.get("pane_command") or value.get("command") or "").strip()
                    command_name = Path(command.split()[0]).name.lower() if command else ""
                    pane_id = value.get("pane_id") or value.get("id")
                    if pane_id is not None and command_name in shell_names:
                        return str(pane_id)
                for child in value.values():
                    found = walk(child)
                    if found is not None:
                        return found
            elif isinstance(value, list):
                for child in value:
                    found = walk(child)
                    if found is not None:
                        return found
            return None

        return walk(payload)
