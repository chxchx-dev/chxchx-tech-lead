from pathlib import Path

from chichan_tech_lead.core.models import ProjectInfo


def test_custom_profile_overrides_builtin_resolution(tmp_path: Path, monkeypatch):
    global_home = tmp_path / "global"
    profiles_dir = global_home / "profiles"
    profiles_dir.mkdir(parents=True)
    (profiles_dir / "custom.toml").write_text(
        "[profiles.my-stack]\n"
        "stacks = [\"nextjs\", \"nestjs\"]\n"
        "priority = 100\n"
        "rules = [\"Use the project API conventions.\"]\n",
        encoding="utf-8",
    )
    monkeypatch.setenv("CHICHAN_HOME", str(global_home))

    info = ProjectInfo(tmp_path / "project", "project", stacks=["nextjs", "nestjs"])

    assert info.profile_name == "my-stack"
