from __future__ import annotations

import asyncio
import time
from concurrent.futures import ThreadPoolExecutor
from contextlib import nullcontext
from pathlib import Path
from types import SimpleNamespace

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.core.trust import is_trusted
from chxchx_tech_lead.workspace.manager import WorkspaceManager
from chxchx_tech_lead.tui import actions
from chxchx_tech_lead.tui import dashboard
from chxchx_tech_lead.tui.application import WorkspaceConsole
from chxchx_tech_lead.tui.palette import CommandPalette
from textual.widgets import Button, Static, TabbedContent


async def _wait_for(pilot, predicate, *, timeout: float = 5.0) -> None:
    deadline = asyncio.get_running_loop().time() + timeout
    while not predicate():
        if asyncio.get_running_loop().time() >= deadline:
            raise AssertionError("Timed out waiting for the expected TUI state")
        await pilot.pause(0.05)


def test_quit_key_exits_tui_without_stopping_workspace(tmp_path: Path, monkeypatch):
    project = ProjectInfo(tmp_path, "project")
    app = WorkspaceConsole(project)

    class ServiceSpy:
        stop_called = False

        def stop(self):
            self.stop_called = True

    service = ServiceSpy()
    app.service = service
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)

    async def press_quit():
        async with app.run_test() as pilot:
            await pilot.press("q")

    asyncio.run(press_quit())

    assert not service.stop_called


def test_primary_navigation_groups_work_and_advanced_views(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    monkeypatch.setattr(WorkspaceConsole, "refresh_dashboard", lambda _self: None)
    monkeypatch.setattr(WorkspaceConsole, "_refresh_project_console", lambda _self: None)

    async def navigate():
        async with app.run_test() as pilot:
            app._show("agents")
            await pilot.pause()
            assert app.query_one("#tabs", TabbedContent).active == "work"
            assert app.query_one("#work-tabs", TabbedContent).active == "agents"
            assert all(
                hasattr(WorkspaceConsole, f"action_{command_id}")
                for command_id, _label, _shortcut in CommandPalette.COMMANDS
            )

            app._show("setup")
            await pilot.pause()
            assert app.query_one("#tabs", TabbedContent).active == "more"
            assert app.query_one("#more-tabs", TabbedContent).active == "setup"

    asyncio.run(navigate())



def test_home_has_direct_terminal_and_agent_actions(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    calls = []
    monkeypatch.setattr(app, "action_attach_agents_workspace", lambda: calls.append("agents"))
    monkeypatch.setattr(app, "action_start_workspace_all", lambda: calls.append("terminal"))

    async def use_home_shortcuts():
        async with app.run_test() as pilot:
            await pilot.click("#btn-start-all")
            await pilot.click("#btn-attach-agents")
            await pilot.pause()

    asyncio.run(use_home_shortcuts())

    assert calls == ["terminal", "agents"]


def test_t_and_a_shortcuts_open_terminal_and_agents(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    calls = []
    monkeypatch.setattr(app, "action_start_workspace_all", lambda: calls.append("terminal"))
    monkeypatch.setattr(app, "action_attach_agents_workspace", lambda: calls.append("agents"))

    async def press_shortcuts():
        async with app.run_test() as pilot:
            await pilot.press("t")
            await pilot.press("a")
            await pilot.pause()

    asyncio.run(press_shortcuts())

    assert calls == ["terminal", "agents"]


def test_long_workspace_action_runs_without_blocking_navigation(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    monkeypatch.setattr(WorkspaceConsole, "_refresh_project_console", lambda _self: None)

    def slow_action():
        time.sleep(0.2)
        return "complete"

    async def interact():
        pool = ThreadPoolExecutor(max_workers=1)
        asyncio.get_running_loop().set_default_executor(pool)
        try:
            async with app.run_test() as pilot:
                app._perform("Tarea lenta", slow_action, refresh=False)
                assert app._operation_pending
                await pilot.press("2")
                await pilot.pause()
                assert app.query_one("#tabs", TabbedContent).active == "work"
                await _wait_for(pilot, lambda: not app._operation_pending)
                assert not app._operation_pending
                assert "Tarea lenta" in str(app.query_one("#log", Static).render())
        finally:
            pool.shutdown(wait=True)

    asyncio.run(interact())


def test_setup_action_refreshes_dashboard_after_project_init(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    refreshes = []
    monkeypatch.setattr(app, "refresh_dashboard", lambda: refreshes.append("dashboard"))
    monkeypatch.setattr(app, "_refresh_project_console", lambda: refreshes.append("console"))

    async def finish_setup():
        async with app.run_test():
            app._finish_setup_action(
                "Proyecto inicializado",
                SimpleNamespace(actions=["ensure config"], warnings=[], backup=None),
                None,
            )

    asyncio.run(finish_setup())
    assert refreshes == ["dashboard", "console"]


def test_dashboard_collection_runs_without_blocking_navigation(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    monkeypatch.setattr(WorkspaceConsole, "_refresh_project_console", lambda _self: None)
    messages = []

    def slow_collection(*_args, **_kwargs):
        time.sleep(0.2)
        raise RuntimeError("simulated slow dashboard probe")

    monkeypatch.setattr(dashboard, "collect_dashboard", slow_collection)
    monkeypatch.setattr(app, "_set_log", messages.append)

    async def interact():
        pool = ThreadPoolExecutor(max_workers=1)
        asyncio.get_running_loop().set_default_executor(pool)
        try:
            async with app.run_test() as pilot:
                app.refresh_dashboard()
                assert app._dashboard_pending
                await pilot.press("2")
                await pilot.pause()
                assert app.query_one("#tabs", TabbedContent).active == "work"
                await _wait_for(pilot, lambda: not app._dashboard_pending)
                assert not app._dashboard_pending
                assert any("simulated slow dashboard probe" in item for item in messages)
        finally:
            pool.shutdown(wait=True)

    asyncio.run(interact())


def test_project_console_probe_runs_without_blocking_navigation(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)

    class SlowService:
        def inspect(self):
            time.sleep(0.2)
            raise RuntimeError("simulated slow console probe")

    app.service = SlowService()

    async def interact():
        pool = ThreadPoolExecutor(max_workers=1)
        asyncio.get_running_loop().set_default_executor(pool)
        try:
            async with app.run_test() as pilot:
                app.query_one("#tabs", TabbedContent).active = "work"
                await _wait_for(pilot, lambda: app._console_pending)
                assert app._console_pending
                await pilot.press("2")
                await pilot.pause()
                assert app.query_one("#tabs", TabbedContent).active == "work"
                await _wait_for(pilot, lambda: not app._console_pending)
                assert not app._console_pending
                assert "simulated slow console probe" in str(
                    app.query_one("#console-technology", Static).render()
                )
        finally:
            pool.shutdown(wait=True)

    asyncio.run(interact())


def test_command_palette_navigation_uses_valid_actions(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    monkeypatch.setattr(WorkspaceConsole, "refresh_dashboard", lambda _self: None)

    async def choose():
        async with app.run_test() as pilot:
            app._run_palette_command("show_agents")
            await pilot.pause()
            assert app.query_one("#tabs", TabbedContent).active == "work"
            assert app.query_one("#work-tabs", TabbedContent).active == "agents"

    asyncio.run(choose())


def test_views_load_on_tab_activation_without_duplicate_refreshes(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    calls = []
    monkeypatch.setattr(app, "_refresh_conversations", lambda: calls.append("conversations"))
    monkeypatch.setattr(app, "refresh_dashboard", lambda: calls.append("dashboard"))

    async def select_views():
        async with app.run_test() as pilot:
            app._show("conversations")
            for _ in range(20):
                await pilot.pause(0.02)
                if calls:
                    break
            assert calls == ["conversations"]
            app._show("resources")
            for _ in range(20):
                await pilot.pause(0.02)
                if len(calls) == 2:
                    break
            assert calls == ["conversations", "dashboard"]

    asyncio.run(select_views())


def test_workspace_start_without_agents_attaches_to_terminal_session(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    calls = []

    class ServiceSpy:
        def attach(self):
            calls.append("terminal")

        def attach_agents(self, **_kwargs):
            calls.append("agents")

    app.service = ServiceSpy()
    monkeypatch.setattr(app, "suspend", lambda: nullcontext())
    monkeypatch.setattr(app, "_restore_after_external_terminal", lambda: None)

    async def attach():
        async with app.run_test():
            app._attach_prepared_agents(include_agents=False)

    asyncio.run(attach())

    assert calls == ["terminal"]


def test_start_workspace_prepares_only_workspace_before_opening_terminal(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    calls = []

    class ServiceSpy:
        def start(self):
            calls.append("workspace")

        def prepare_agents(self):
            calls.append("agents")
            raise AssertionError("el inicio habitual no debe lanzar agentes")

    app.service = ServiceSpy()
    monkeypatch.setattr(
        app,
        "_attach_prepared_agents",
        lambda include_agents: calls.append("agents" if include_agents else "terminal"),
    )

    async def prepare():
        pool = ThreadPoolExecutor(max_workers=1)
        asyncio.get_running_loop().set_default_executor(pool)
        try:
            await WorkspaceConsole._prepare_workspace_for_attach.__wrapped__(app)
        finally:
            pool.shutdown(wait=True)

    asyncio.run(prepare())

    assert calls == ["workspace", "terminal"]


def test_explicit_agents_attach_still_prepares_and_opens_agents(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    calls = []
    config = SimpleNamespace(agents=[object()])

    class ServiceSpy:
        def prepare_agents(self):
            calls.append("agents")
            return SimpleNamespace(inspection=SimpleNamespace(config=config)), []

    app.service = ServiceSpy()
    monkeypatch.setattr(
        app,
        "_attach_prepared_agents",
        lambda include_agents: calls.append("attach-agents" if include_agents else "terminal"),
    )

    async def prepare():
        pool = ThreadPoolExecutor(max_workers=1)
        asyncio.get_running_loop().set_default_executor(pool)
        try:
            await WorkspaceConsole._prepare_agents_workspace.__wrapped__(app)
        finally:
            pool.shutdown(wait=True)

    asyncio.run(prepare())

    assert calls == ["agents", "attach-agents"]


def test_start_workspace_without_config_reports_setup_instead_of_opening_terminal(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    calls = []
    messages = []

    class ServiceSpy:
        def inspect(self):
            return SimpleNamespace(
                config_path=tmp_path / ".ai" / "chxchx-tech.toml",
                config=object(),
                trusted=False,
            )

        def start(self):
            calls.append("start")

    app.service = ServiceSpy()
    monkeypatch.setattr(app, "_set_log", messages.append)

    async def refuse_start():
        async with app.run_test():
            app.action_start_workspace_all()

    asyncio.run(refuse_start())

    assert not calls
    assert any("Falta .ai/chxchx-tech.toml" in message for message in messages)


def test_trust_without_config_marks_project_trusted_from_button(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))
    app = WorkspaceConsole(ProjectInfo(project, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    monkeypatch.setattr(app, "refresh_dashboard", lambda: None)

    async def trust_from_button():
        async with app.run_test() as pilot:
            inspection = WorkspaceManager(app.project).inspect()
            snapshot = dashboard.DashboardSnapshot(inspection, None, None, None, [], [], None)
            app._render_dashboard(snapshot)
            assert not app.query_one("#btn-trust", Button).disabled
            assert not app.query_one("#btn-console-trust", Button).disabled
            assert app.query_one("#btn-start-all", Button).disabled
            await pilot.click("#btn-trust")
            await pilot.pause()

    asyncio.run(trust_from_button())

    assert is_trusted(project)


def test_dashboard_disables_start_but_allows_trust_without_saved_config(tmp_path: Path, monkeypatch):
    project = ProjectInfo(tmp_path, "project")
    app = WorkspaceConsole(project)
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    inspection = WorkspaceManager(project).inspect()
    snapshot = dashboard.DashboardSnapshot(inspection, None, None, None, [], [], None)

    async def render():
        async with app.run_test():
            app._render_dashboard(snapshot)
            assert not app.query_one("#btn-trust", Button).disabled
            assert not app.query_one("#btn-console-trust", Button).disabled
            assert app.query_one("#btn-start-all", Button).disabled
            assert app.query_one("#btn-attach-agents", Button).disabled

    asyncio.run(render())
