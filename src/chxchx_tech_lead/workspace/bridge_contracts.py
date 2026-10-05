"""Versioned JSON schema names and top-level contracts for UI clients."""

from __future__ import annotations

from typing import Final, Literal, NotRequired, TypedDict

SCHEMA_VERSION: Final = 1

PROJECT_STATUS_SCHEMA: Final[Literal["chxchx.project-status"]] = "chxchx.project-status"
RESOURCES_OVERVIEW_SCHEMA: Final[Literal["chxchx.resources-overview"]] = "chxchx.resources-overview"
HANDOFF_SCHEMA: Final[Literal["chxchx.handoff"]] = "chxchx.handoff"
MEMORY_SCHEMA: Final[Literal["chxchx.memory"]] = "chxchx.memory"
CONVERSATIONS_SCHEMA: Final[Literal["chxchx.conversations"]] = "chxchx.conversations"
CONVERSATION_SCHEMA: Final[Literal["chxchx.conversation"]] = "chxchx.conversation"
ERRORS_SCHEMA: Final[Literal["chxchx.errors"]] = "chxchx.errors"
ERROR_SCHEMA: Final[Literal["chxchx.error"]] = "chxchx.error"


class BridgeErrorPayload(TypedDict):
    schema: Literal["chxchx.error"]
    schema_version: int
    error: str


class AgentStatusPayload(TypedDict):
    id: str
    command: str
    arguments: list[str]
    shell: bool
    cwd: str
    available: bool
    session: str
    pane: str
    preset: str
    version: NotRequired[str | None]


class ProjectStatusPayload(TypedDict):
    schema: Literal["chxchx.project-status"]
    schema_version: int
    project: dict[str, object]
    workspace: dict[str, object]
    registered_projects: list[dict[str, object]]
    skills: list[dict[str, object]]
    enabled_skill_count: int
    packs: list[dict[str, object]]
    agents: list[AgentStatusPayload]
    processes: list[dict[str, object]]
    resources: dict[str, object]


class ResourcesOverviewPayload(TypedDict):
    schema: Literal["chxchx.resources-overview"]
    schema_version: int
    system: dict[str, object]
    projects: list[dict[str, object]]


class HandoffPayload(TypedDict):
    schema: Literal["chxchx.handoff"]
    schema_version: int
    exists: bool
    path: str
    content: str
    summary: NotRequired[str]
    pending: NotRequired[str]
    validation: NotRequired[str]


class MemoryPayload(TypedDict):
    schema: Literal["chxchx.memory"]
    schema_version: int
    project: str
    query: str
    notes: list[dict[str, str]]


class ConversationsPayload(TypedDict):
    schema: Literal["chxchx.conversations"]
    schema_version: int
    project: str
    query: str
    conversations: list[dict[str, str]]


class ConversationPayload(TypedDict):
    schema: Literal["chxchx.conversation"]
    schema_version: int
    provider: str
    session_id: str
    title: str
    modified_at: str
    transcript_path: str
    messages: list[dict[str, str]]


class ErrorsPayload(TypedDict):
    schema: Literal["chxchx.errors"]
    schema_version: int
    project: str
    cache_path: str
    errors: list[dict[str, str]]
