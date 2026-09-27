from pathlib import Path

from chxchx_tech_lead.adapters.editor.sublime import SublimeAdapter
from chxchx_tech_lead.core.runner import CommandResult


def test_sublime_adapter_builds_safe_project_and_location_commands(tmp_path: Path):
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        return CommandResult(list(command), 0, "ok", "")

    adapter = SublimeAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/subl")

    project = adapter.open_project(tmp_path)
    file_result = adapter.open_file(tmp_path / "main.py", line=10, column=4)

    assert adapter.available() is True
    assert project.returncode == 0
    assert file_result.command[-1].endswith("main.py:10:4")
    assert all("shell" not in kwargs for _, kwargs in calls)


def test_sublime_adapter_rejects_invalid_location(tmp_path: Path):
    adapter = SublimeAdapter(runner=lambda *args, **kwargs: None, lookup=lambda _: None)

    result = adapter.open_file(tmp_path / "main.py", line=0)

    assert result.returncode == 2
    assert "mayor que cero" in result.stderr
