from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import tomllib

from ..core.detector import detect_project


@dataclass(frozen=True)
class TechPack:
    name: str
    description: str
    stacks: tuple[str, ...]
    languages: tuple[str, ...]
    skills: tuple[str, ...]


@dataclass(frozen=True)
class PackRecommendation:
    pack: TechPack
    reasons: tuple[str, ...]


class TechPackRegistry:
    """Loads curated skill combinations and matches them to detected projects."""

    def __init__(self, root: Path | None = None) -> None:
        self.root = root or Path(__file__).parent / "library" / "packs"

    def list(self) -> list[TechPack]:
        return [self._load(path) for path in sorted(self.root.glob("*.toml"))]

    def get(self, name: str) -> TechPack | None:
        key = name.casefold()
        return next((pack for pack in self.list() if pack.name.casefold() == key), None)

    def detect(self, project: Path) -> list[PackRecommendation]:
        info = detect_project(project)
        stacks, languages = set(info.stacks), set(info.languages)
        found = []
        for pack in self.list():
            reasons = [f"stack detectado: {item}" for item in pack.stacks if item in stacks]
            reasons.extend(f"lenguaje detectado: {item}" for item in pack.languages if item in languages)
            if reasons:
                found.append(PackRecommendation(pack, tuple(reasons)))
        return found

    @staticmethod
    def _load(path: Path) -> TechPack:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
        name = data.get("name")
        description = data.get("description")
        skills = data.get("skills")
        if not isinstance(name, str) or not isinstance(description, str) or not isinstance(skills, list):
            raise ValueError(f"Tech Pack inválido, requiere name, description y skills: {path}")
        return TechPack(
            name=name,
            description=description,
            stacks=tuple(data.get("stacks", [])),
            languages=tuple(data.get("languages", [])),
            skills=tuple(skills),
        )
