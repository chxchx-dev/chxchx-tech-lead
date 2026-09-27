from __future__ import annotations

import sys
from collections.abc import Sequence


def agent_pane_command(
    agent_id: str,
    command: Sequence[str],
    *,
    label: str,
    logo: str,
) -> list[str]:
    """Build the command used by both initial and dynamic agent panes."""
    return [
        sys.executable,
        "-m",
        "chxchx_tech_lead.workspace.agent_pane",
        "--name",
        agent_id,
        "--label",
        label,
        "--logo",
        logo,
        "--",
        *command,
    ]
