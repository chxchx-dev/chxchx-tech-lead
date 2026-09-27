from pathlib import Path

from chxchx_tech_lead.adapters.agents.claude import ClaudeAdapter
from chxchx_tech_lead.adapters.agents.codex import CodexAdapter
from chxchx_tech_lead.core.runner import CommandResult


def test_agent_adapter_reports_version_and_starts_in_project(tmp_path: Path):
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        if command[-1] == "--version":
            return CommandResult(list(command), 0, "codex 1.2.3", "")
        return CommandResult(list(command), 0, "started", "")

    adapter = CodexAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/codex")

    assert adapter.status().version == "codex 1.2.3"
    result = adapter.start(tmp_path, dry_run=True)

    assert result.returncode == 0
    assert calls[-1][0] == ["codex"]
    assert calls[-1][1]["cwd"] == tmp_path
    assert calls[-1][1]["interactive"] is True


def test_unavailable_agent_is_reported_without_execution(tmp_path: Path):
    calls = []
    adapter = ClaudeAdapter(runner=lambda *args, **kwargs: calls.append(args), lookup=lambda _: None)

    info = adapter.status()
    result = adapter.start(tmp_path)

    assert info.available is False
    assert result.returncode == 127
    assert calls == []
