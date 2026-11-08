from pathlib import Path

from chxchx_tech_lead.workspace.error_cache import list_errors, record_error


def test_record_error_redacts_common_credentials(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "home"))

    record_error(
        project="project",
        project_path=project,
        operation="chat",
        message="api_key=sk-1234567890abcdef123456 password=hunter2 Bearer abc.def.ghi",
    )

    errors = list_errors(project_path=project)

    assert len(errors) == 1
    assert "sk-1234567890abcdef123456" not in errors[0].message
    assert "hunter2" not in errors[0].message
    assert "abc.def.ghi" not in errors[0].message
    assert errors[0].message.count("[REDACTED]") == 3
