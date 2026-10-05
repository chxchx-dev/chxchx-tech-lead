from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import tomllib

from ..core.detector import detect_project


@dataclass(frozen=True)
class Skill:
    name: str
    description: str
    stacks: tuple[str, ...]
    languages: tuple[str, ...]
    tags: tuple[str, ...]
    always_recommend: bool
    origin: str
    instructions: str
    source: Path


@dataclass(frozen=True)
class SkillRecommendation:
    skill: Skill
    reasons: tuple[str, ...]


class SkillRegistry:
    """Loads small, local Markdown skills with TOML metadata."""

    def __init__(self, roots: tuple[Path, ...] | None = None) -> None:
        default = Path(__file__).parent / "library" / "skills"
        self.roots = roots or (default,)

    def list(self) -> list[Skill]:
        skills: dict[str, Skill] = {}
        for root in self.roots:
            if not root.is_dir():
                continue
            for manifest in sorted(root.glob("*/skill.toml")):
                skill = self._load(manifest)
                skills.setdefault(skill.name, skill)
        return sorted(skills.values(), key=lambda skill: skill.name)

    def get(self, name: str) -> Skill | None:
        key = name.casefold()
        return next((skill for skill in self.list() if skill.name.casefold() == key), None)

    def search(self, query: str) -> list[Skill]:
        terms = query.casefold().split()
        if not terms:
            return self.list()
        found = []
        for skill in self.list():
            haystack = " ".join((skill.name, skill.description, *skill.tags)).casefold()
            if all(term in haystack for term in terms):
                found.append(skill)
        return found

    def recommend(self, project: Path) -> list[Skill]:
        return [item.skill for item in self.recommendations(project)]

    def recommendations(self, project: Path) -> list[SkillRecommendation]:
        info = detect_project(project)
        stacks, languages = set(info.stacks), set(info.languages)
        found = []
        for skill in self.list():
            reasons = [f"stack detectado: {item}" for item in skill.stacks if item in stacks]
            reasons.extend(
                f"lenguaje detectado: {item}"
                for item in skill.languages
                if item in languages
            )
            if skill.always_recommend:
                reasons.append("guía general aplicable a cualquier proyecto")
            if reasons:
                found.append(SkillRecommendation(skill, tuple(reasons)))
        return found

    @staticmethod
    def _load(manifest: Path) -> Skill:
        metadata = tomllib.loads(manifest.read_text(encoding="utf-8"))
        name = metadata.get("name")
        description = metadata.get("description")
        if not isinstance(name, str) or not isinstance(description, str):
            raise ValueError(f"Skill inválida, requiere name y description: {manifest}")
        instructions_path = manifest.with_name("SKILL.md")
        instructions = instructions_path.read_text(encoding="utf-8").strip()
        return Skill(
            name=name,
            description=description,
            stacks=tuple(metadata.get("stacks", [])),
            languages=tuple(metadata.get("languages", [])),
            tags=tuple(metadata.get("tags", [])),
            always_recommend=bool(metadata.get("always_recommend", False)),
            origin=str(metadata.get("origin", "local")),
            instructions=instructions,
            source=manifest.parent,
        )
