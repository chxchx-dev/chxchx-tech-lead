from __future__ import annotations

import json
import shlex
import threading
import time
from collections.abc import Callable

from ...core.runner import CommandResult


class ZellijAttachOperations:
    """Attach clients and focus their requested tab after they connect."""

    command: str
    _runner: Callable[..., CommandResult]

    def attach_session(
        self,
        name: str,
        dry_run: bool = False,
        *,
        focus_tab: str | None = None,
        focus_pane: str | None = None,
        tab_layout: str | None = None,
    ) -> CommandResult:
        command = [self.command, "attach", "--force-run-commands", name]
        if dry_run or not focus_tab:
            return self._runner(command, dry_run=dry_run, interactive=True)

        finished = threading.Event()
        worker = threading.Thread(
            target=self._focus_after_client_connect,
            args=(name, focus_tab, focus_pane, tab_layout, finished),
            daemon=True,
            name="zellij-tab-focus",
        )
        worker.start()
        try:
            return self._runner(command, interactive=True)
        finally:
            finished.set()
            worker.join(timeout=0.15)

    def _focus_after_client_connect(
        self,
        session: str,
        tab_name: str,
        pane_name: str | None,
        tab_layout: str | None,
        finished: threading.Event,
    ) -> None:
        deadline = time.monotonic() + 2.5
        next_create_attempt = 0.0
        while not finished.is_set() and time.monotonic() < deadline:
            tabs = self.list_tabs(session)
            target = self._find_tab(tabs.stdout, tab_name) if tabs.returncode == 0 else None
            if target is None and tab_layout and time.monotonic() >= next_create_attempt:
                self.add_layout_tab(session, tab_name, tab_layout)
                next_create_attempt = time.monotonic() + 0.25
            elif target is not None:
                self.focus_tab(session, tab_name)
                if pane_name:
                    self._focus_named_agent(session, tab_name, pane_name, finished)
            finished.wait(0.15)

    def _focus_named_agent(
        self,
        session: str,
        tab_name: str,
        pane_name: str,
        finished: threading.Event,
    ) -> None:
        deadline = time.monotonic() + 4
        command = [self.command, "--session", session, "action", "list-panes", "--all", "--json"]
        while not finished.is_set() and time.monotonic() < deadline:
            panes = self._runner(command, timeout=1)
            pane_id = self._find_agent_pane_id(panes.stdout, tab_name, pane_name)
            if panes.returncode == 0 and pane_id is not None:
                self.focus_pane_id(session, pane_id)
                return
            finished.wait(0.1)

    @staticmethod
    def _find_tab(raw: str, wanted: str) -> dict | None:
        try:
            tabs = json.loads(raw)
        except (TypeError, json.JSONDecodeError):
            return None
        if not isinstance(tabs, list):
            return None
        return next(
            (
                tab
                for tab in tabs
                if isinstance(tab, dict)
                and str(tab.get("name") or tab.get("tab_name") or "").casefold()
                == wanted.casefold()
            ),
            None,
        )

    @staticmethod
    def _find_agent_pane_id(raw: str, tab_name: str, pane_name: str) -> str | None:
        try:
            panes = json.loads(raw)
        except (TypeError, json.JSONDecodeError):
            return None
        if not isinstance(panes, list):
            return None
        for pane in panes:
            if not isinstance(pane, dict) or pane.get("tab_name") != tab_name:
                continue
            command = pane.get("pane_command") or pane.get("terminal_command") or ""
            try:
                arguments = shlex.split(command)
            except ValueError:
                continue
            for index, argument in enumerate(arguments[:-1]):
                if argument == "--name" and arguments[index + 1] == pane_name:
                    pane_id = pane.get("pane_id", pane.get("id"))
                    return str(pane_id) if pane_id is not None else None
        return None
