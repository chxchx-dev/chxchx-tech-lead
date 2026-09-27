from __future__ import annotations

from datetime import datetime, timezone
from pathlib import Path

from ..core.managed import upsert_managed_block
from ..core.models import ProjectInfo
from .agent_status import AgentRuntimeStatus
from .models import WorkspaceStatus


def render_handoff(
    info: ProjectInfo,
    status: WorkspaceStatus,
    agents: list[AgentRuntimeStatus],
    *,
    summary: str,
    pending: str,
    validation: str,
) -> str:
    rows = "\n".join(
        f"- `{agent.id}`: {'disponible' if agent.available else 'no disponible'}; "
        f"sesión `{agent.session}`; pane `{agent.pane}`"
        for agent in agents
    ) or "- No hay agentes configurados."
    return f"""## Contexto administrado por chxchx-tech

Actualizado: `{datetime.now(timezone.utc).isoformat()}`
Proyecto: `{info.name}`
Perfil: `{info.profile_name}`
Workspace: `{status.value}`

### Último cambio

- {summary}

### Agentes

{rows}

### Pendiente

- {pending}

### Validación

- {validation}
"""


def update_handoff(
    info: ProjectInfo,
    status: WorkspaceStatus,
    agents: list[AgentRuntimeStatus],
    *,
    summary: str,
    pending: str,
    validation: str,
    dry_run: bool = False,
) -> bool:
    return upsert_managed_block(
        info.root / ".ai" / "HANDOFF.md",
        "workspace-handoff",
        render_handoff(
            info,
            status,
            agents,
            summary=summary,
            pending=pending,
            validation=validation,
        ),
        dry_run=dry_run,
    )
