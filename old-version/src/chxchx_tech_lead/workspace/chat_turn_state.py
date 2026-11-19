"""Best-effort persistence of the independent Python chat adapter's transcript."""

from __future__ import annotations

from collections.abc import Callable
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from .agent_chat import ChatTurnPlan


def remember(write: Callable[..., None], plan: ChatTurnPlan, session_id: str | None, payload: Any) -> None:
    try:
        write(plan.project_root, plan.agent_id, plan.provider, session_id, payload)
    except OSError:
        pass  # a read-only project must not stop the chat; it just will not be restored
