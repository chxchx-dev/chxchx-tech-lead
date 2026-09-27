from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from ..core.config_migrations import ConfigMigrationError, migrate_project_config
from ..core.models import ProjectInfo
from ..core.trust import is_trusted
from .models import WorkspaceConfig, WorkspaceConfigError
from .state import WorkspaceState, load_state


@dataclass(frozen=True, slots=True)
class WorkspaceInspection:
    project_root: Path
    config_path: Path
    config: WorkspaceConfig | None
    state: WorkspaceState
    trusted: bool
    warnings: tuple[str, ...] = ()
    error: str | None = None


class WorkspaceManager:
    """Lee y valida un workspace; todavía no ejecuta procesos ni adapters externos."""

    def __init__(self, project: ProjectInfo):
        self.project = project

    def inspect(self) -> WorkspaceInspection:
        migration = None
        config = None
        warnings: list[str] = []
        error = None
        try:
            migration = migrate_project_config(self.project, dry_run=True)
            raw = migration.content
            import tomllib

            config = WorkspaceConfig.from_mapping(
                tomllib.loads(raw).get("workspace"),
                project_root=self.project.root,
            )
            if migration.source_version == 1:
                warnings.append(".ai/chxchx-tech.toml requiere migración de v1 a v2")
        except (ConfigMigrationError, WorkspaceConfigError) as exc:
            error = str(exc)
        return WorkspaceInspection(
            project_root=self.project.root,
            config_path=self.project.root / ".ai" / "chxchx-tech.toml",
            config=config,
            state=load_state(self.project.root),
            trusted=is_trusted(self.project.root),
            warnings=tuple(warnings),
            error=error,
        )
