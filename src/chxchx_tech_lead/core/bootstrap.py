from __future__ import annotations

from dataclasses import dataclass, field

from .backup import backup_project
from .managed import upsert_managed_block
from .models import ProjectInfo
from .project_config import ensure_project_config, project_config_needs_update
from .registry import register_project
from .templates import claude_body, create_project_structure, project_rule_body
from ..integrations.basic_memory import available as basic_memory_available
from ..integrations.basic_memory import ensure_project as ensure_memory_project
from ..integrations.installers import install_tool
from ..integrations.mcp import integrate as integrate_mcp
from ..integrations.mcp import write_opencode_example


@dataclass
class SetupResult:
    actions: list[str] = field(default_factory=list)
    warnings: list[str] = field(default_factory=list)
    backup: str | None = None


def initialize_project(
    info: ProjectInfo,
    *,
    dry_run: bool = False,
    no_backup: bool = False,
    minimal: bool = False,
) -> SetupResult:
    result = SetupResult()
    planned_structure = create_project_structure(info, dry_run=True, minimal=minimal)
    planned_config = project_config_needs_update(info)
    planned_agents = planned_claude = planned_opencode = False
    if not minimal:
        planned_agents = upsert_managed_block(
            info.root / "AGENTS.md", "project-rules", project_rule_body(info), dry_run=True
        )
        planned_claude = upsert_managed_block(
            info.root / "CLAUDE.md", "claude-rules", claude_body(), dry_run=True
        )
        planned_opencode = _opencode_example_needs_update(info)

    if not dry_run and not no_backup and any(
        (planned_structure, planned_config, planned_agents, planned_claude, planned_opencode)
    ):
        result.backup = backup_project(info.root)

    result.actions.extend(create_project_structure(info, dry_run=dry_run, minimal=minimal))
    planned_memory = not (info.root / ".ai" / "memory").exists()
    ensure_project_config(info, dry_run=dry_run)
    if planned_config or planned_memory:
        result.actions.append("ensure .ai/chxchx-tech.toml + .ai/memory")
    if basic_memory_available():
        memory_result = ensure_memory_project(info, dry_run=dry_run)
        if memory_result is not None:
            if memory_result.returncode != 0:
                result.warnings.append(
                    f"Basic Memory project: {memory_result.stderr or memory_result.stdout}"
                )
            elif not memory_result.skipped:
                result.actions.append("ensure Basic Memory project")
    else:
        result.warnings.append("Basic Memory no está instalado; ejecuta la acción Instalar herramientas.")

    if not minimal:
        if upsert_managed_block(
            info.root / "AGENTS.md", "project-rules", project_rule_body(info), dry_run=dry_run
        ):
            result.actions.append("update AGENTS.md")
        if upsert_managed_block(
            info.root / "CLAUDE.md", "claude-rules", claude_body(), dry_run=dry_run
        ):
            result.actions.append("update CLAUDE.md")
    if register_project(info, dry_run=dry_run):
        result.actions.append("register project")
    if not minimal and write_opencode_example(info, dry_run=dry_run):
        result.actions.append("ensure OpenCode MCP example")
    if dry_run:
        result.actions.append("DRY RUN: no se escribieron cambios")
    if not result.actions:
        result.actions.append("Todo está actualizado")
    return result


def install_base_tools(*, dry_run: bool = False) -> SetupResult:
    result = SetupResult()
    for tool in ("basic-memory", "serena"):
        installed = install_tool(tool, dry_run=dry_run)
        if installed.returncode != 0:
            result.warnings.append(f"{tool}: {installed.stderr or 'falló la instalación'}")
        elif installed.skipped:
            result.actions.append(f"{tool} ya está instalado")
        else:
            result.actions.append(f"{'DRY RUN ' if dry_run else ''}{' '.join(installed.command)}")
    return result


def configure_integrations(
    info: ProjectInfo, *, clients: tuple[str, ...] = ("claude", "codex", "opencode"),
    dry_run: bool = False, refresh: bool = False,
) -> SetupResult:
    result = SetupResult()
    for client in clients:
        try:
            entries = integrate_mcp(info, client, dry_run=dry_run, refresh=refresh)
        except RuntimeError as exc:
            result.warnings.append(f"{client}: {exc}")
            continue
        for entry in entries:
            if entry.returncode == 0:
                result.actions.append(" ".join(entry.command))
            else:
                result.warnings.append(f"{client}: {entry.stderr or 'falló la integración'}")
    return result


def _opencode_example_needs_update(info: ProjectInfo) -> bool:
    target = info.root / ".ai" / "integrations" / "opencode-mcp.example.json"
    return not target.exists() or write_opencode_example(info, dry_run=True)
