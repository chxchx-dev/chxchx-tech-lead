from __future__ import annotations

import asyncio
import json
from pathlib import Path

from textual.widgets import DataTable, Input, TabbedContent

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.skills import enabled_skills
from chxchx_tech_lead.tui.application import WorkspaceConsole


def test_skills_panel_lists_catalog_and_manages_project_context(
    tmp_path: Path, monkeypatch
):
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "chxchx-home"))
    (tmp_path / "package.json").write_text(
        json.dumps({"dependencies": {"next": "1", "react": "1"}}), encoding="utf-8"
    )
    app = WorkspaceConsole(ProjectInfo(tmp_path, "project"))
    monkeypatch.setattr(WorkspaceConsole, "refresh_dashboard", lambda _self: None)
    monkeypatch.setattr(WorkspaceConsole, "_refresh_project_console", lambda _self: None)

    async def use_skills_panel():
        async with app.run_test() as pilot:
            app.action_show_skills()
            await pilot.pause(0.2)
            assert app.query_one("#skills-table", DataTable).row_count == 17
            assert app.query_one("#packs-table", DataTable).row_count == 11
            assert app._pack_matches

            search = app.query_one("#skills-search", Input)
            search.value = "python"
            await pilot.pause(0.1)
            assert app.query_one("#skills-table", DataTable).row_count == 1
            search.value = ""
            await pilot.pause(0.1)

            await pilot.click("#btn-skill-enable")
            await pilot.pause(0.1)
            assert len(enabled_skills(tmp_path)) == 1

            await pilot.click("#btn-skills-sync")
            await pilot.pause(0.1)
            assert (tmp_path / ".ai" / "SKILLS.md").is_file()
            assert (tmp_path / "AGENTS.md").is_file()

            app.query_one("#skills-tabs", TabbedContent).active = "skill-packs"
            await pilot.pause(0.1)
            await pilot.click("#btn-packs-apply-detected")
            await pilot.pause(0.1)
            assert len(enabled_skills(tmp_path)) > 1

    asyncio.run(use_skills_panel())
