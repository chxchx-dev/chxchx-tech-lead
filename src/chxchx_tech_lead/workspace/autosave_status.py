from __future__ import annotations

from dataclasses import dataclass

from chxchx_tech_lead.core.models import ProjectInfo
from chxchx_tech_lead.integrations.mcp_diagnostics import diagnose_project_mcp

from .memory_history import MemoryNote, list_memory_notes


@dataclass(frozen=True, slots=True)
class AutoSaveStatus:
    instructions_ready: bool
    codex_memory_status: str
    claude_memory_status: str
    latest_note: MemoryNote | None

    @property
    def ready(self) -> bool:
        return (
            self.instructions_ready
            and self.codex_memory_status == "OK"
            and self.claude_memory_status == "OK"
        )


def diagnose_autosave(project: ProjectInfo) -> AutoSaveStatus:
    """Summarize configured checkpoint instructions and persisted project notes."""
    root = project.root.resolve()
    try:
        agents_rules = (root / "AGENTS.md").read_text(encoding="utf-8")
        claude_rules = (root / "CLAUDE.md").read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        instructions_ready = False
    else:
        instructions_ready = "write_memory" in agents_rules and "write_memory" in claude_rules

    diagnostics = diagnose_project_mcp(project)
    statuses = {
        item.client: item.status
        for item in diagnostics
        if item.server == "basic-memory"
    }
    notes = list_memory_notes(root)
    return AutoSaveStatus(
        instructions_ready=instructions_ready,
        codex_memory_status=statuses.get("Codex", "FALTA"),
        claude_memory_status=statuses.get("Claude Code", "FALTA"),
        latest_note=notes[0] if notes else None,
    )


def format_autosave_status(status: AutoSaveStatus) -> str:
    overall = "CONFIGURACIÓN LISTA" if status.ready else "REVISAR CONFIGURACIÓN"
    rules = "✓" if status.instructions_ready else "✗"
    codex = _status_icon(status.codex_memory_status)
    claude = _status_icon(status.claude_memory_status)
    latest = status.latest_note
    if latest is None:
        note_line = "Última nota: todavía no hay notas persistidas en .ai/memory"
    else:
        saved = latest.modified_at.astimezone().strftime("%Y-%m-%d %H:%M %Z")
        note_line = f"Última nota observada: {latest.title} · {saved}"
    lines = [
        f"Guardado de contexto · {overall}",
        f"Reglas de guardado: {rules}  |  Codex MCP configurado: {codex} {status.codex_memory_status}  |  "
        f"Claude MCP configurado: {claude} {status.claude_memory_status}",
        note_line,
        "Comprobación estática; la última nota demuestra que hay memoria persistida, no que se guardó cada turno.",
    ]
    if not status.instructions_ready:
        lines.append("Reglas: `chxchx-tech sync --dry-run .` y luego `chxchx-tech sync .`")
    if status.codex_memory_status != "OK":
        lines.append("Codex MCP: `chxchx-tech integrate --dry-run --refresh --client codex .`")
    if status.claude_memory_status != "OK":
        lines.append("Claude MCP: `chxchx-tech integrate --dry-run --refresh --client claude .`")
    return "\n".join(lines)


def _status_icon(value: str) -> str:
    return "✓" if value == "OK" else "!" if value == "AVISO" else "✗"
