from pathlib import Path

from chxchx_tech_lead.skills import SkillRegistry


def _write_skill(root: Path, name: str, *, stacks: str = "[]", always: bool = False) -> None:
    folder = root / name
    folder.mkdir(parents=True)
    (folder / "skill.toml").write_text(
        f'name = "{name}"\n'
        f'description = "Guidance for {name}."\n'
        f"stacks = {stacks}\n"
        "languages = []\n"
        f"always_recommend = {str(always).lower()}\n"
        'tags = ["engineering"]\n',
        encoding="utf-8",
    )
    (folder / "SKILL.md").write_text(f"Instructions for {name}.", encoding="utf-8")


def test_registry_lists_and_loads_instructions(tmp_path: Path) -> None:
    _write_skill(tmp_path, "python-engineering", stacks='["python"]')
    registry = SkillRegistry((tmp_path,))

    skill = registry.get("PYTHON-ENGINEERING")

    assert skill is not None
    assert [item.name for item in registry.list()] == ["python-engineering"]
    assert skill.instructions == "Instructions for python-engineering."
    assert skill.origin == "local"


def test_search_matches_all_terms_in_metadata(tmp_path: Path) -> None:
    _write_skill(tmp_path, "python-engineering", stacks='["python"]')
    _write_skill(tmp_path, "api-design")
    registry = SkillRegistry((tmp_path,))

    assert [skill.name for skill in registry.search("PYTHON engineering")] == ["python-engineering"]
    assert registry.search("missing") == []


def test_recommendation_matches_project_stack_and_general_skills(tmp_path: Path) -> None:
    library = tmp_path / "library"
    library.mkdir()
    _write_skill(library, "python-engineering", stacks='["python"]')
    _write_skill(library, "testing", always=True)
    _write_skill(library, "nestjs", stacks='["nestjs"]')
    project = tmp_path / "project"
    project.mkdir()
    (project / "pyproject.toml").write_text("[project]\nname = 'sample'\n", encoding="utf-8")
    (project / "main.py").write_text("print('hello')\n", encoding="utf-8")

    recommended = SkillRegistry((library,)).recommend(project)

    assert [skill.name for skill in recommended] == ["python-engineering", "testing"]
