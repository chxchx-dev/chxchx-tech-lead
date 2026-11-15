from __future__ import annotations

from ..core.models import ProjectInfo


class TUIUnavailableError(RuntimeError):
    """Textual no está instalado en el entorno actual."""


def run_tui(project: ProjectInfo) -> None:
    """Start the interactive workspace console."""
    try:
        from .application import WorkspaceConsole
    except ImportError as exc:
        raise TUIUnavailableError(
            "Textual no está instalado; instala la dependencia de TUI antes de ejecutar `chxchx-tech tui`."
        ) from exc
    WorkspaceConsole(project).run()
