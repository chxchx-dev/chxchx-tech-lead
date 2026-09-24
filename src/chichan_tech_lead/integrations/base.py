from __future__ import annotations

from dataclasses import dataclass

from chichan_tech_lead.core.runner import executable, run, CommandResult


@dataclass(slots=True)
class ToolCheck:
    name: str
    command: str
    installed: bool


def check(name: str, command: str) -> ToolCheck:
    return ToolCheck(name=name, command=command, installed=executable(command) is not None)


def execute(command: list[str], dry_run: bool = False) -> CommandResult:
    return run(command, dry_run=dry_run)
