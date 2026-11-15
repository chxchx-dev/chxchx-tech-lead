import json
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


def test_generate_sublime_project_is_local_idempotent_and_excludes_build_output(tmp_path: Path):
    adapter = SublimeAdapter(lookup=lambda _: "/usr/bin/subl")

    project_file, preview = adapter.generate_project(tmp_path, dry_run=True)
    assert preview.returncode == 0
    assert not project_file.exists()

    project_file, generated = adapter.generate_project(tmp_path)
    payload = json.loads(project_file.read_text(encoding="utf-8"))
    assert generated.returncode == 0
    assert project_file == tmp_path / ".ai" / "sublime" / f"{tmp_path.name}.sublime-project"
    assert "node_modules" in payload["folders"][0]["folder_exclude_patterns"]
    assert "__pycache__" in payload["folders"][0]["folder_exclude_patterns"]
    assert payload["settings"]["chxchx_tech_lead_managed"] is True

    _, unchanged = adapter.generate_project(tmp_path)
    assert unchanged.stdout == "Configuración Sublime sin cambios"
    assert not list(project_file.parent.glob("*.bak-*"))


def test_generate_sublime_project_backups_managed_file_and_refuses_unmanaged(tmp_path: Path):
    adapter = SublimeAdapter(lookup=lambda _: "/usr/bin/subl")
    project_file, _ = adapter.generate_project(tmp_path)
    project_file.write_text('{"settings":{"chxchx_tech_lead_managed":true},"custom":1}\n', encoding="utf-8")

    _, generated = adapter.generate_project(tmp_path)
    assert generated.returncode == 0
    assert list(project_file.parent.glob("*.bak-*"))

    project_file.write_text('{"folders":[]}\n', encoding="utf-8")
    _, refused = adapter.generate_project(tmp_path)
    assert refused.returncode == 3
    assert "no administrado" in refused.stderr


def test_open_sublime_project_file(tmp_path: Path):
    calls = []
    project_file = tmp_path / "sample.sublime-project"
    project_file.write_text("{}", encoding="utf-8")
    adapter = SublimeAdapter(
        runner=lambda command, **kwargs: calls.append(command) or CommandResult(list(command), 0, "", ""),
        lookup=lambda _: "/usr/bin/subl",
    )

    result = adapter.open_project(project_file)

    assert result.returncode == 0
    assert calls == [["subl", str(project_file.resolve())]]
