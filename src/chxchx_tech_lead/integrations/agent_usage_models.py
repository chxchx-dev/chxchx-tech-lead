"""Data models shared by local agent usage readers."""

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class LocalContextUsage:
    provider: str
    input_tokens: int | None = None
    context_window: int | None = None
    used_percent: float | None = None
    updated_at: float = 0.0


@dataclass(frozen=True, slots=True)
class LiveAgentUsage:
    context_used_percent: float | None = None
    context_remaining_percent: float | None = None
    input_tokens: int | None = None
    context_window: int | None = None
    five_hour_used_percent: float | None = None
    five_hour_resets_at: float | None = None
    seven_day_used_percent: float | None = None
    seven_day_resets_at: float | None = None
    updated_at: float = 0.0
