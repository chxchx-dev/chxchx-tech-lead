from __future__ import annotations

import json
from pathlib import Path

from chxchx_tech_lead.skills import SkillRegistry, TechPackRegistry, set_skill_enabled
from chxchx_tech_lead.workspace.error_cache import record_error
from chxchx_tech_lead.workspace.bridge import (
    project_status_payload,
)
from chxchx_tech_lead.workspace.agent_commands import NEW_CHAT_PROMPT
from chxchx_tech_lead.workspace.bridge_resources import resources_overview_payload
from chxchx_tech_lead.workspace.bridge_history import (
    conversation_payload,
    conversations_payload,
    errors_payload,
    handoff_payload,
    memory_payload,
)


def test_project_status_includes_skill_and_pack_catalog(tmp_path: Path) -> None:
    (tmp_path / "pyproject.toml").write_text("[project]\nname = 'bridge-fixture'\n", encoding="utf-8")

    payload = project_status_payload(tmp_path)

    assert payload["schema"] == "chxchx.project-status"
    assert payload["schema_version"] == 1
    assert len(payload["skills"]) == len(SkillRegistry().list())
    assert len(payload["packs"]) == len(TechPackRegistry().list())
    assert payload["enabled_skill_count"] == 0
    assert all({"name", "description", "enabled", "recommended", "reasons"} <= skill.keys()
               for skill in payload["skills"])
    assert all({"name", "description", "skills", "recommended", "reasons"} <= pack.keys()
               for pack in payload["packs"])

    selected_name = payload["skills"][0]["name"]
    set_skill_enabled(tmp_path, selected_name, True)
    selected_payload = project_status_payload(tmp_path)
    selected = next(skill for skill in selected_payload["skills"] if skill["name"] == selected_name)
    assert selected_payload["enabled_skill_count"] == 1
    assert selected["enabled"] is True


def test_project_status_includes_shared_studio_agent_launchers(tmp_path: Path) -> None:
    project_config = tmp_path / ".ai" / "chxchx-tech.toml"
    project_config.parent.mkdir()
    project_config.write_text(
        'version = 2\nprofile = "python"\n'
        '[workspace]\nname = "bridge fixture"\nauto_start = false\nauto_attach = false\n'
        '[[workspace.agents]]\nid = "codex"\ncommand = ["codex", "--model", "gpt"]\n',
        encoding="utf-8",
    )

    agent = project_status_payload(tmp_path)["agents"][0]

    assert agent["studio_command"][1:4] == ["-m", "chxchx_tech_lead.workspace.agent_pane", "--name"]
    assert "codex" in agent["studio_command"]
    assert agent["studio_command"][-3:] == ["codex", "--model", "gpt"]
    assert agent["studio_new_chat_command"][-1] == NEW_CHAT_PROMPT
    assert ".ai/memory/PROJECT_MEMORY.md" in NEW_CHAT_PROMPT
    assert "añádelo de inmediato" in NEW_CHAT_PROMPT


def test_readonly_handoff_and_memory_payloads(tmp_path: Path) -> None:
    handoff_path = tmp_path / ".ai" / "HANDOFF.md"
    handoff_path.parent.mkdir()
    handoff_path.write_text(
        "# Handoff\n\n### Último cambio\n\n- Actualicé el bridge.\n\n"
        "### Pendiente\n\n- Conectar la memoria.\n\n### Validación\n\n- Ejecutar pytest.\n",
        encoding="utf-8",
    )
    memory_root = tmp_path / ".ai" / "memory"
    memory_root.mkdir()
    (memory_root / "decision.md").write_text("# Architecture decision\nUse Qt Widgets.\n", encoding="utf-8")
    (memory_root / "other.md").write_text("# Deployment\nShip packages.\n", encoding="utf-8")

    handoff = handoff_payload(tmp_path)
    all_notes = memory_payload(tmp_path)
    filtered = memory_payload(tmp_path, query="widgets")

    assert handoff["exists"] is True
    assert "Actualicé el bridge" in handoff["content"]
    assert handoff["summary"] == "Actualicé el bridge."
    assert handoff["pending"] == "Conectar la memoria."
    assert handoff["validation"] == "Ejecutar pytest."
    assert len(all_notes["notes"]) == 2
    assert len(filtered["notes"]) == 1
    assert filtered["notes"][0]["title"] == "Architecture decision"


def test_chat_and_error_payloads_are_project_scoped(tmp_path: Path, monkeypatch) -> None:
    project = tmp_path / "project"
    project.mkdir()
    other_project = tmp_path / "other"
    other_project.mkdir()
    codex_home = tmp_path / "codex"
    transcript = codex_home / "sessions" / "rollout-123.jsonl"
    transcript.parent.mkdir(parents=True)
    transcript.write_text(
        "\n".join([
            json.dumps({"type": "session_meta", "payload": {"id": "session-123", "cwd": str(project)}}),
            json.dumps({"type": "event_msg", "payload": {"type": "user_message", "message": "Fix startup"}}),
            json.dumps({"type": "event_msg", "payload": {"type": "agent_message", "message": "Updated startup."}}),
        ]),
        encoding="utf-8",
    )
    monkeypatch.setenv("CODEX_HOME", str(codex_home))
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "chxchx"))
    record_error(project="project", project_path=project, operation="build", message="Build warning")
    record_error(project="other", project_path=other_project, operation="test", message="Other project")

    conversations = conversations_payload(project, query="startup")
    conversation = conversation_payload(project, "Codex", "session-123")
    errors = errors_payload(project)

    assert conversations["schema"] == "chxchx.conversations"
    assert [item["session_id"] for item in conversations["conversations"]] == ["session-123"]
    assert conversation["messages"][-1]["text"] == "Updated startup."
    assert errors["schema"] == "chxchx.errors"
    assert [item["operation"] for item in errors["errors"]] == ["build"]


def test_resources_overview_aggregates_registered_projects_read_only(tmp_path: Path, monkeypatch) -> None:
    current = tmp_path / "current"
    other = tmp_path / "other"
    current.mkdir()
    other.mkdir()
    config_path = other / ".ai" / "chxchx-tech.toml"
    config_path.parent.mkdir()
    config_path.write_text("[workspace]\nversion = 2\n", encoding="utf-8")
    registry_home = tmp_path / "registry"
    registry_home.mkdir()
    (registry_home / "projects.json").write_text(json.dumps({"projects": [
        {"alias": "other-app", "name": "Other", "path": str(other)},
    ]}), encoding="utf-8")
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(registry_home))

    payload = resources_overview_payload(current)

    assert payload["schema"] == "chxchx.resources-overview"
    assert payload["schema_version"] == 1
    assert payload["system"]["memory_total_bytes"] > 0
    assert {item["path"] for item in payload["projects"]} == {str(current), str(other)}
    other_project = next(item for item in payload["projects"] if item["path"] == str(other))
    assert other_project["alias"] == "other-app"
    assert {"running_count", "rss_bytes", "cpu_percent", "max_agents"} <= other_project.keys()
    assert config_path.read_text(encoding="utf-8") == "[workspace]\nversion = 2\n"
