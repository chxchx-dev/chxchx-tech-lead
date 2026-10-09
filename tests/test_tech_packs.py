from pathlib import Path

from chxchx_tech_lead.skills import SkillRegistry, TechPackRegistry, enabled_skills, enable_skills


def test_detect_packs_explains_matching_stack_and_language(tmp_path: Path) -> None:
    project = tmp_path / "next-app"
    project.mkdir()
    (project / "package.json").write_text(
        '{"dependencies":{"next":"^15","react":"^19"}}',
        encoding="utf-8",
    )
    (project / "page.tsx").write_text("export default function Page() { return null }\n", encoding="utf-8")

    matches = TechPackRegistry().detect(project)
    reasons = {match.pack.name: match.reasons for match in matches}

    assert "nextjs-app" in reasons
    assert "stack detectado: nextjs" in reasons["nextjs-app"]
    assert "typescript-foundation" in reasons
    assert "lenguaje detectado: typescript" in reasons["typescript-foundation"]
    assert "ui-ux-design" in {skill.name for skill in SkillRegistry().recommend(project)}
    nextjs_pack = TechPackRegistry().get("nextjs-app")
    assert nextjs_pack is not None and "ui-ux-design" in nextjs_pack.skills


def test_every_pack_references_known_skills() -> None:
    packs = TechPackRegistry().list()
    skill_names = {skill.name for skill in SkillRegistry().list()}

    assert len(packs) >= 10
    assert all(set(pack.skills).issubset(skill_names) for pack in packs)


def test_apply_pack_adds_to_existing_selection_without_duplicates(tmp_path: Path, monkeypatch) -> None:
    project = tmp_path / "project"
    project.mkdir()
    monkeypatch.setenv("CHXCHX_TECH_HOME", str(tmp_path / "chxchx-home"))
    enable_skills(project, ("architecture",))
    pack = TechPackRegistry().get("python-backend")

    assert pack is not None
    first = enable_skills(project, pack.skills)
    second = enable_skills(project, pack.skills)

    assert first.changed is True
    assert second.changed is False
    assert set(pack.skills).issubset(enabled_skills(project))
    assert "architecture" in enabled_skills(project)
