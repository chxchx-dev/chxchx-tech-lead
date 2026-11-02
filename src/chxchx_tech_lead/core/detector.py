from __future__ import annotations

import copy
import json
import os
import re
import time
from dataclasses import dataclass, field
from pathlib import Path

from .models import ProjectInfo

IGNORED_DIRS = {
    ".git", ".venv", "venv", "node_modules", "dist", "build", ".next",
    ".turbo", ".nx", "coverage", "bin", "obj", ".idea", ".vscode",
    "target", "__pycache__", ".pytest_cache", ".mypy_cache", ".tox", ".gradle", "Pods",
}
_QT_BUILD_NAMES = {"cmakelists.txt", "meson.build"}
_QT_BUILD_SUFFIXES = (".cmake", ".pro", ".pri")
# Status views call detection several times per refresh; a short cache keeps
# them consistent and cheap while long-running UIs still see new files soon.
_CACHE_SECONDS = 5.0
_cache: dict[Path, tuple[float, ProjectInfo]] = {}


@dataclass(slots=True)
class _TreeScan:
    suffixes: set[str] = field(default_factory=set)
    csproj: list[Path] = field(default_factory=list)
    qt_candidates: list[Path] = field(default_factory=list)


def _scan_tree(root: Path) -> _TreeScan:
    """Walk the project once, skipping dependency and build folders."""
    scan = _TreeScan()
    for current, dirs, files in os.walk(root):
        dirs[:] = [directory for directory in dirs if directory not in IGNORED_DIRS]
        for filename in files:
            lowered = filename.casefold()
            suffix = Path(lowered).suffix
            if suffix:
                scan.suffixes.add(suffix)
            if suffix == ".csproj":
                scan.csproj.append(Path(current, filename))
            if lowered in _QT_BUILD_NAMES or lowered.endswith(_QT_BUILD_SUFFIXES):
                scan.qt_candidates.append(Path(current, filename))
    return scan


def _csproj_mentions(scan: _TreeScan, marker: str) -> bool:
    for path in scan.csproj:
        try:
            if marker in path.read_text(encoding="utf-8").lower():
                return True
        except OSError:
            pass
    return False


def _read_package_json(root: Path) -> dict:
    path = root / "package.json"
    if not path.exists():
        return {}
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (json.JSONDecodeError, OSError):
        return {}


def _detect_ai(root: Path, package: dict) -> bool:
    dependencies = set((package.get("dependencies", {}) or {}).keys())
    dependencies.update((package.get("devDependencies", {}) or {}).keys())
    markers = {"openai", "anthropic", "langchain", "llama-index", "transformers"}
    if dependencies.intersection(markers):
        return True

    for filename in ("pyproject.toml", "requirements.txt", "requirements-dev.txt"):
        path = root / filename
        if path.exists():
            try:
                content = path.read_text(encoding="utf-8").lower()
                if any(marker in content for marker in markers):
                    return True
            except OSError:
                pass
    return False


def _detect_postgres(root: Path, package: dict, scan: _TreeScan) -> bool:
    dependencies = set((package.get("dependencies", {}) or {}).keys())
    dependencies.update((package.get("devDependencies", {}) or {}).keys())
    if dependencies.intersection({"pg", "postgres", "postgresql"}):
        return True

    if _csproj_mentions(scan, "npgsql"):
        return True

    for filename in ("pyproject.toml", "requirements.txt", "requirements-dev.txt"):
        path = root / filename
        if path.exists():
            try:
                content = path.read_text(encoding="utf-8").lower()
                if any(marker in content for marker in ("psycopg", "asyncpg")):
                    return True
            except OSError:
                pass
    return False


def _detect_javascript_stacks(package: dict) -> set[str]:
    dependencies = set((package.get("dependencies", {}) or {}).keys())
    dependencies.update((package.get("devDependencies", {}) or {}).keys())
    stacks = set()
    if "next" in dependencies:
        stacks.add("nextjs")
    if "react-native" in dependencies:
        stacks.add("react-native")
    if "@nestjs/core" in dependencies:
        stacks.add("nestjs")
    if "react" in dependencies and "next" not in dependencies:
        stacks.add("react-vite" if "vite" in dependencies else "react")
    if dependencies.intersection({"prisma", "@prisma/client"}):
        stacks.add("prisma")
    if dependencies.intersection({"redis", "ioredis", "@redis/client"}):
        stacks.add("redis")
    return stacks


def _detect_docker(root: Path) -> bool:
    names = {"Dockerfile", "docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"}
    return any((root / name).is_file() for name in names)

def _detect_redis(root: Path, package: dict, scan: _TreeScan) -> bool:
    dependencies = set((package.get("dependencies", {}) or {}).keys())
    dependencies.update((package.get("devDependencies", {}) or {}).keys())
    if dependencies.intersection({"redis", "ioredis", "@redis/client"}):
        return True
    for filename in ("pyproject.toml", "requirements.txt", "requirements-dev.txt"):
        path = root / filename
        if path.exists():
            try:
                content = path.read_text(encoding="utf-8")
                dependency_pattern = r"""(?im)(?:^\s*|["',\[]\s*)redis(?:\s*(?:[<>=!~;]|\[|["']))"""
                if re.search(dependency_pattern, content):
                    return True
            except OSError:
                pass
    return _csproj_mentions(scan, "stackexchange.redis")


def _detect_qt(scan: _TreeScan) -> bool:
    for path in scan.qt_candidates:
        try:
            content = path.read_text(encoding="utf-8", errors="ignore")[:512 * 1024].casefold()
        except OSError:
            continue
        if "qt6" in content or "qt5" in content:
            return True
        if path.suffix.casefold() in {".pro", ".pri"} and "qt +=" in content:
            return True
    return False


def detect_project(root: Path) -> ProjectInfo:
    root = root.resolve()
    cached = _cache.get(root)
    if cached is not None and time.monotonic() - cached[0] < _CACHE_SECONDS:
        return copy.deepcopy(cached[1])
    info = _detect_uncached(root)
    _cache[root] = (time.monotonic(), copy.deepcopy(info))
    return info


def _detect_uncached(root: Path) -> ProjectInfo:
    info = ProjectInfo(root=root, name=root.name)
    package = _read_package_json(root)
    scan = _scan_tree(root)
    suffixes = scan.suffixes
    cpp_suffixes = {".cc", ".cpp", ".cxx", ".c++", ".hh", ".hpp", ".hxx", ".ipp"}
    if suffixes & cpp_suffixes:
        info.languages.append("cpp")
        info.stacks.append("cpp")
    if _detect_qt(scan):
        info.stacks.append("qt")

    info.stacks.extend(sorted(_detect_javascript_stacks(package)))
    if any(root.glob("*.sln")) or scan.csproj:
        info.stacks.append("dotnet")
    if (root / "pyproject.toml").exists() or (root / "requirements.txt").exists():
        info.stacks.append("python")
    if (root / "Cargo.toml").exists():
        info.stacks.append("rust")
    if (root / "go.mod").exists():
        info.stacks.append("go")
    if _detect_postgres(root, package, scan):
        info.stacks.append("postgres")
    if _detect_redis(root, package, scan) and "redis" not in info.stacks:
        info.stacks.append("redis")
    if _detect_docker(root):
        info.stacks.append("docker")
    if _detect_ai(root, package):
        info.stacks.append("ai")

    if (root / "pnpm-lock.yaml").exists():
        info.package_managers.append("pnpm")
    if (root / "yarn.lock").exists():
        info.package_managers.append("yarn")
    if (root / "package-lock.json").exists():
        info.package_managers.append("npm")
    if (root / "uv.lock").exists():
        info.package_managers.append("uv")

    mapping = {
        ".ts": "typescript",
        ".tsx": "typescript",
        ".js": "javascript",
        ".jsx": "javascript",
        ".cs": "csharp",
        ".py": "python",
        ".kt": "kotlin",
        ".swift": "swift",
        ".sql": "sql",
        ".rs": "rust",
        ".go": "go",
        ".cc": "cpp",
        ".cpp": "cpp",
        ".cxx": "cpp",
        ".c++": "cpp",
        ".hh": "cpp",
        ".hpp": "cpp",
        ".hxx": "cpp",
        ".ipp": "cpp",
    }
    for suffix, language in mapping.items():
        if suffix in suffixes and language not in info.languages:
            info.languages.append(language)

    if (root / ".github" / "workflows").exists():
        info.infrastructure.append("github-actions")

    if not info.stacks:
        info.stacks.append("generic")

    return info
