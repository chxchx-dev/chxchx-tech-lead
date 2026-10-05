from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path
from typing import Callable

from ...core.runner import CommandResult, executable, run


class SublimeAdapter:
    """Abre proyectos y archivos mediante el comando opcional `subl`."""

    def __init__(
        self,
        command: str = "subl",
        runner: Callable[..., CommandResult] = run,
        lookup: Callable[[str], str | None] = executable,
    ):
        self.command = command
        self._runner = runner
        self._lookup = lookup

    def available(self) -> bool:
        return self._lookup(self.command) is not None

    def open_project(self, path: Path, dry_run: bool = False) -> CommandResult:
        target = path.expanduser().resolve()
        if not target.is_dir() and not (target.is_file() and target.suffix == ".sublime-project"):
            return CommandResult([self.command, str(target)], 2, "", f"No existe el proyecto: {target}")
        return self._runner([self.command, str(target)], dry_run=dry_run)

    def generate_project(self, root: Path, dry_run: bool = False) -> tuple[Path, CommandResult]:
        project_root = root.expanduser().resolve()
        destination = project_root / ".ai" / "sublime" / f"{project_root.name}.sublime-project"
        command = ["generate-sublime-project", str(destination)]
        if not project_root.is_dir():
            return destination, CommandResult(command, 2, "", f"No existe el proyecto: {project_root}")

        content = _project_content(project_root)
        if dry_run:
            return destination, CommandResult(command, 0, "DRY RUN", "")

        try:
            destination.parent.mkdir(parents=True, exist_ok=True)
            if destination.exists():
                existing = json.loads(destination.read_text(encoding="utf-8"))
                settings = existing.get("settings", {}) if isinstance(existing, dict) else {}
                if not isinstance(settings, dict) or not settings.get("chxchx_tech_lead_managed"):
                    return destination, CommandResult(
                        command, 3, "", f"No sobrescribí un proyecto Sublime no administrado: {destination}"
                    )
                if destination.read_text(encoding="utf-8") == content:
                    return destination, CommandResult(command, 0, "Configuración Sublime sin cambios", "")
                backup = destination.with_suffix(destination.suffix + f".bak-{datetime.now():%Y%m%d-%H%M%S-%f}")
                backup.write_bytes(destination.read_bytes())
            temporary = destination.with_suffix(destination.suffix + ".tmp")
            temporary.write_text(content, encoding="utf-8")
            temporary.replace(destination)
        except (OSError, json.JSONDecodeError) as exc:
            return destination, CommandResult(command, 1, "", f"No pude generar el proyecto Sublime: {exc}")
        return destination, CommandResult(command, 0, f"Proyecto Sublime generado: {destination}", "")

    def open_file(
        self,
        path: Path,
        line: int | None = None,
        column: int | None = None,
        dry_run: bool = False,
    ) -> CommandResult:
        target = path.expanduser().resolve()
        if line is not None and line < 1:
            return CommandResult([self.command, str(target)], 2, "", "La línea debe ser mayor que cero")
        if column is not None and column < 1:
            return CommandResult([self.command, str(target)], 2, "", "La columna debe ser mayor que cero")
        location = str(target)
        if line is not None:
            location += f":{line}"
            if column is not None:
                location += f":{column}"
        return self._runner([self.command, location], dry_run=dry_run)


def _project_content(root: Path) -> str:
    payload = {
        "folders": [
            {
                "path": str(root),
                "folder_exclude_patterns": [
                    ".git", ".hg", ".svn", "node_modules", ".next", "dist", "build",
                    "coverage", ".venv", "venv", ".cache", ".turbo", "turbo",
                    "__pycache__", ".pytest_cache", "bin", "obj",
                ],
                "file_exclude_patterns": ["*.pyc", "*.pyo", "*.log", "*.tmp", "*.swp", "*.swo"],
            }
        ],
        "settings": {"chxchx_tech_lead_managed": True},
    }
    return json.dumps(payload, ensure_ascii=False, indent=2) + "\n"
