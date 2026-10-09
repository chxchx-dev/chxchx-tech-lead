from __future__ import annotations

import asyncio
from pathlib import Path
from types import SimpleNamespace

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.tui.application import WorkspaceConsole
from chxchx_tech_lead.workspace.models import ResourceConfig
from chxchx_tech_lead.workspace.resources import ResourceManager, SystemResources


def test_ram_governor_prompts_and_respects_cancel_or_override(tmp_path: Path, monkeypatch):
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "on_mount", lambda _self: None)
    monkeypatch.setattr(
        ResourceManager,
        "system",
        lambda _self: SystemResources(100, 92, 8, 100, 45),
    )
    app.service = SimpleNamespace(
        inspect=lambda: SimpleNamespace(
            config=SimpleNamespace(
                resources=ResourceConfig(
                    warn_memory_percent=70,
                    critical_memory_percent=85,
                    warn_swap_percent=40,
                    max_agents=2,
                ),
                agents=[SimpleNamespace(id=name) for name in ("codex", "claude", "opencode")],
            )
        )
    )
    app._last_agents = [SimpleNamespace(id="codex", pane="RUNNING")]
    calls: list[str] = []

    async def confirm_budget():
        async with app.run_test() as pilot:
            app._guard_agent_launch(None, lambda: calls.append("started"))
            await pilot.pause(0.1)
            assert app._governor_pending
            await pilot.click("#governor-cancel")
            await pilot.pause(0.1)
            assert calls == []
            assert not app._governor_pending

            app._guard_agent_launch(None, lambda: calls.append("started"))
            await pilot.pause(0.1)
            await pilot.click("#governor-continue")
            await pilot.pause(0.1)
            assert calls == ["started"]
            assert not app._governor_pending

    asyncio.run(confirm_budget())
