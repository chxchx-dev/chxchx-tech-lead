from __future__ import annotations

import asyncio
from pathlib import Path

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.tui.application import WorkspaceConsole


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
