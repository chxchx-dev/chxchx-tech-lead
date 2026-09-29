from __future__ import annotations

import json
import os
from pathlib import Path

from .models import ProjectInfo

IGNORED_DIRS = {
    ".git", ".venv", "venv", "node_modules", "dist", "build", ".next",
    ".turbo", ".nx", "coverage", "bin", "obj", ".idea", ".vscode",
}


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


def _detect_postgres(root: Path, package: dict) -> bool:
    dependencies = set((package.get("dependencies", {}) or {}).keys())
    dependencies.update((package.get("devDependencies", {}) or {}).keys())
    if dependencies.intersection({"pg", "postgres", "postgresql"}):
        return True

    for filename in ("docker-compose.yml", "docker-compose.yaml", "compose.yml", "compose.yaml"):
        path = root / filename
        if path.exists():
            try:
                if "postgres" in path.read_text(encoding="utf-8").lower():
                    return True
            except OSError:
                pass

    for path in root.rglob("*.csproj"):
        try:
            if "npgsql" in path.read_text(encoding="utf-8").lower():
                return True
        except OSError:
            pass

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


def _source_suffixes(root: Path) -> set[str]:
    suffixes: set[str] = set()
    for current, dirs, files in os.walk(root):
        dirs[:] = [d for d in dirs if d not in IGNORED_DIRS]
        for filename in files:
            suffix = Path(filename).suffix.lower()
            if suffix:
                suffixes.add(suffix)
    return suffixes


def detect_project(root: Path) -> ProjectInfo:
    root = root.resolve()
    info = ProjectInfo(root=root, name=root.name)
    package = _read_package_json(root)
    deps = {}
    deps.update(package.get("dependencies", {}) or {})
    deps.update(package.get("devDependencies", {}) or {})

    if "next" in deps:
        info.stacks.append("nextjs")
    if "react-native" in deps:
        info.stacks.append("react-native")
    if "@nestjs/core" in deps:
        info.stacks.append("nestjs")
    if "vite" in deps and "react" in deps and "next" not in deps:
        info.stacks.append("react-vite")
    if any(root.glob("*.sln")) or any(root.rglob("*.csproj")):
        info.stacks.append("dotnet")
    if (root / "pyproject.toml").exists() or (root / "requirements.txt").exists():
        info.stacks.append("python")
    if (root / "Cargo.toml").exists():
        info.stacks.append("rust")
    if (root / "go.mod").exists():
        info.stacks.append("go")
    if _detect_postgres(root, package):
        info.stacks.append("postgres")
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

    suffixes = _source_suffixes(root)
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
    }
    for suffix, language in mapping.items():
        if suffix in suffixes and language not in info.languages:
            info.languages.append(language)

    if (root / "Dockerfile").exists() or (root / "docker-compose.yml").exists() or (root / "compose.yml").exists():
        info.infrastructure.append("docker")
    if (root / ".github" / "workflows").exists():
        info.infrastructure.append("github-actions")

    if not info.stacks:
        info.stacks.append("generic")

    return info
