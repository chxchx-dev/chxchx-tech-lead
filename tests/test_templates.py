from pathlib import Path

from chichan_tech_lead.core.detector import detect_project
from chichan_tech_lead.core.templates import create_project_structure, project_rule_body


def test_structure_idempotent(tmp_path: Path):
    info = detect_project(tmp_path)
    first = create_project_structure(info)
    second = create_project_structure(info)
    assert first
    assert second == []
    assert (tmp_path / ".ai" / "PROJECT.md").exists()
    assert (tmp_path / "docs" / "adr").exists()


def test_generated_rules_include_agent_protocol(tmp_path: Path):
    info = detect_project(tmp_path)

    body = project_rule_body(info)

    assert "## Protocolo de trabajo" in body
    assert "`.ai/PROJECT.md`" in body
    assert "`.ai/HANDOFF.md`" in body
