from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Any

from .models import ProjectInfo


def suggested_processes(info: ProjectInfo) -> list[dict[str, Any]]:
    """Return conservative development commands inferred from project markers.

    These suggestions are stored in the project config but are only executed
    after the user trusts the project and explicitly starts its workspace.
    """
    root = info.root.resolve()
    package_path = root / "package.json"
    package: dict[str, Any] = {}
    try:
        package = json.loads(package_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        pass

    if not isinstance(package, dict):
        package = {}
    scripts = package.get("scripts", {})
    scripts = scripts if isinstance(scripts, dict) else {}
    dependencies: dict[str, Any] = {}
    for section in ("dependencies", "devDependencies"):
        values = package.get(section, {})
        if isinstance(values, dict):
            dependencies.update(values)

    manager = next(
        (candidate for candidate in ("pnpm", "yarn", "npm") if candidate in info.package_managers),
        "npm",
    )
    node_script = _node_script(info, scripts, dependencies)
    if node_script:
        command = _node_command(manager, node_script)
        primary_stack = next(
            (stack for stack in info.stacks if stack in {"react-native", "nextjs", "react-vite", "nestjs"}),
            "",
        )
        label = (
            "Expo / React Native"
            if "expo" in dependencies
            else {
                "react-native": "React Native / Metro",
                "nextjs": "Next.js",
                "react-vite": "Vite",
                "nestjs": "NestJS",
            }.get(primary_stack, "Node.js")
        )
        return [{
            "id": "dev-server",
            "label": label,
            "command": command,
            "cwd": ".",
            "auto_start": True,
        }]

    projects = sorted(_csproj_files(root))
    if len(projects) == 1:
        return [{
            "id": "dotnet-watch",
            "label": ".NET watch",
            "command": ["dotnet", "watch", "run"],
            "cwd": projects[0].parent.relative_to(root).as_posix() or ".",
            "auto_start": True,
        }]

    if (root / "manage.py").is_file():
        command = ["uv", "run", "python", "manage.py", "runserver"] if (root / "uv.lock").exists() else ["python", "manage.py", "runserver"]
        return [{"id": "django", "label": "Django", "command": command, "cwd": ".", "auto_start": True}]

    if (root / "Cargo.toml").is_file():
        return [{"id": "cargo-run", "label": "Rust", "command": ["cargo", "run"], "cwd": ".", "auto_start": True}]

    if (root / "go.mod").is_file() and (root / "main.go").is_file():
        return [{"id": "go-run", "label": "Go", "command": ["go", "run", "."], "cwd": ".", "auto_start": True}]

    return []


def _node_script(
    info: ProjectInfo,
    scripts: dict[str, Any],
    dependencies: dict[str, Any],
) -> str | None:
    available = {str(name) for name in scripts}
    if "react-native" in info.stacks or "expo" in dependencies:
        candidates = ("start", "dev")
    elif "nestjs" in info.stacks:
        candidates = ("start:dev", "dev", "start")
    elif "nextjs" in info.stacks or "react-vite" in info.stacks:
        candidates = ("dev", "start")
    else:
        candidates = ("dev", "start")
    return next((name for name in candidates if name in available), None)


def _node_command(manager: str, script: str) -> list[str]:
    if manager == "npm" and script == "start":
        return ["npm", "start"]
    if manager == "pnpm" and script == "start":
        return ["pnpm", "start"]
    if manager == "yarn" and script == "start":
        return ["yarn", "start"]
    return [manager, "run", script]


_IGNORED_PARTS = {".git", "node_modules", "bin", "obj", ".venv", "venv"}


def _csproj_files(root: Path) -> list[Path]:
    """Find project files without descending into dependency folders."""
    found: list[Path] = []
    for current, dirs, files in os.walk(root):
        dirs[:] = [directory for directory in dirs if directory not in _IGNORED_PARTS]
        found.extend(Path(current, name) for name in files if name.casefold().endswith(".csproj"))
    return found
