from pathlib import Path

from chxchx_tech_lead.core.trust import is_trusted, trust_project


def test_trust_is_local_and_dry_run_is_read_only(tmp_path: Path, monkeypatch):
    project = tmp_path / "project"
    project.mkdir()
    config = project / ".ai" / "chxchx-tech.toml"
    config.parent.mkdir()
    config.write_text("version = 2\n", encoding="utf-8")
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "global"))

    assert trust_project(project, dry_run=True) is True
    assert not (tmp_path / "global").exists()
    assert is_trusted(project) is False

    assert trust_project(project) is True
    assert is_trusted(project) is True
    config.write_text("version = 2\nchanged = true\n", encoding="utf-8")
    assert is_trusted(project) is False
