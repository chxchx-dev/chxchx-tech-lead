from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path

from .paths import home_dir


@dataclass(frozen=True, slots=True)
class Profile:
    name: str
    stacks: tuple[str, ...]
    priority: int
    rules: tuple[str, ...] = ()


def _read_profiles(path: Path) -> dict[str, Profile]:
    if not path.exists():
        return {}
    try:
        raw = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError):
        return {}

    profiles: dict[str, Profile] = {}
    for name, values in (raw.get("profiles", {}) or {}).items():
        if not isinstance(values, dict):
            continue
        stacks = tuple(str(stack) for stack in values.get("stacks", []) or [])
        rules = tuple(str(rule) for rule in values.get("rules", []) or [])
        priority = int(values.get("priority", len(stacks)))
        profiles[name] = Profile(name=name, stacks=stacks, priority=priority, rules=rules)
    return profiles


def profile_paths() -> list[Path]:
    builtin = Path(__file__).with_name("profiles.toml")
    custom_dir = home_dir() / "profiles"
    return [builtin, *sorted(custom_dir.glob("*.toml"))]


def load_profiles() -> dict[str, Profile]:
    profiles: dict[str, Profile] = {}
    for path in profile_paths():
        profiles.update(_read_profiles(path))
    return profiles


def resolve_profile(stacks: list[str] | tuple[str, ...]) -> str:
    available = load_profiles()
    stack_set = set(stacks)
    matches = [
        profile
        for profile in available.values()
        if set(profile.stacks).issubset(stack_set)
    ]
    if not matches:
        return "generic"
    selected = max(matches, key=lambda profile: (profile.priority, len(profile.stacks), profile.name))
    return selected.name


def profile_rules(name: str) -> tuple[str, ...]:
    return load_profiles().get(name, Profile(name, (), 0)).rules
