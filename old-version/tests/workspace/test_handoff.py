from pathlib import Path

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.workspace.agent_status import AgentRuntimeStatus
from chxchx_tech_lead.workspace.handoff import update_handoff
from chxchx_tech_lead.workspace.models import WorkspaceStatus


def test_handoff_updates_managed_block_and_preserves_manual_notes(tmp_path: Path):
    handoff = tmp_path / ".ai" / "HANDOFF.md"
    handoff.parent.mkdir()
    handoff.write_text("# Handoff\n\nNota manual.\n", encoding="utf-8")
    agent = AgentRuntimeStatus("codex", "codex", str(tmp_path), True, "codex 1", "ACTIVE", "RUNNING", "default")

    changed = update_handoff(
        ProjectInfo(tmp_path, "demo"),
        WorkspaceStatus.ACTIVE,
        [agent],
        summary="Se validó el flujo.",
        pending="Probar el siguiente cambio.",
        validation="pytest",
    )

    content = handoff.read_text(encoding="utf-8")
    assert changed is True
    assert "Nota manual." in content
    assert "Se validó el flujo." in content
    assert "chxchx-tech:start workspace-handoff" in content
