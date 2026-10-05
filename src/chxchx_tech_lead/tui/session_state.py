"""Typed mutable state shared by the TUI application and its feature mixins."""

from __future__ import annotations

from dataclasses import dataclass, field

from textual.timer import Timer

from ..integrations.chat_history import Conversation
from ..skills import PackRecommendation, SkillRecommendation, SkillRegistry, TechPackRegistry
from ..workspace.agent_status import AgentRuntimeStatus
from ..workspace.error_cache import CachedError
from ..workspace.manager import WorkspaceInspection
from ..workspace.memory_history import MemoryNote
from ..workspace.models import ProcessConfig


@dataclass
class WorkspaceSessionState:
    last_inspection: WorkspaceInspection | None = None
    dashboard_pending: bool = False
    dashboard_refresh_again: bool = False
    last_agents: list[AgentRuntimeStatus] = field(default_factory=list)
    memory_notes: dict[str, MemoryNote] = field(default_factory=dict)
    skill_recommendations: dict[str, SkillRecommendation] = field(default_factory=dict)
    skill_active: set[str] = field(default_factory=set)
    skill_registry: SkillRegistry | None = None
    tech_pack_registry: TechPackRegistry | None = None
    pack_matches: dict[str, PackRecommendation] = field(default_factory=dict)
    conversations: dict[str, Conversation] = field(default_factory=dict)
    cached_errors: dict[str, CachedError] = field(default_factory=dict)
    palette_open: bool = False
    attach_pending: bool = False
    start_pending: bool = False
    operation_pending: bool = False
    setup_pending: bool = False
    governor_pending: bool = False
    console_process_id: str | None = None
    console_processes: dict[str, ProcessConfig] = field(default_factory=dict)
    console_pending: bool = False
    console_refresh_again: bool = False
    console_output_pending: bool = False
    conversation_refresh_generation: int = 0
    memory_refresh_generation: int = 0
    chat_search_timer: Timer | None = None
    memory_search_timer: Timer | None = None
    tab_refresh_timer: Timer | None = None


class SessionField:
    """Descriptor that preserves legacy mixin attributes over typed state."""

    def __set_name__(self, owner, name: str) -> None:
        self.name = name.removeprefix("_")

    def __get__(self, instance, owner=None):
        if instance is None:
            return self
        return getattr(instance._session_state, self.name)

    def __set__(self, instance, value) -> None:
        setattr(instance._session_state, self.name, value)
