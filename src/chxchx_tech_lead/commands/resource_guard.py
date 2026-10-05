from __future__ import annotations

import typer

from ..ui.cli_output import console
from ..workspace.resources import ResourceManager


def agent_start_warnings(
    service,
    target_ids: tuple[str, ...] | None = None,
    *,
    preset_id: str | None = None,
    new_chat: bool = False,
) -> tuple[str, ...]:
    """Return current memory and agent-budget warnings for a CLI launch."""
    inspection = service.inspect()
    config = inspection.config
    if config is None:
        return ()
    configured = {agent.id for agent in config.agents}
    if preset_id is not None:
        selected = set(config.presets.get(preset_id, ())) & configured
    else:
        selected = configured if target_ids is None else set(target_ids) & configured
    if not selected:
        return ()
    active = {
        agent.id
        for agent in service.agent_statuses(probe_versions=False)
        if agent.pane == "RUNNING" and agent.id in configured
    }
    planned = len(selected) if new_chat else len(selected - active)
    manager = ResourceManager(config.resources)
    return manager.agent_start_warnings(len(active), planned)


def guard_cli_agent_start(
    service,
    target_ids: tuple[str, ...] | None = None,
    *,
    preset_id: str | None = None,
    new_chat: bool = False,
    dry_run: bool = False,
    force: bool = False,
) -> None:
    warnings = agent_start_warnings(
        service, target_ids, preset_id=preset_id, new_chat=new_chat
    )
    if not warnings:
        return
    console.print("[yellow]RAM Governor: lanzamiento sobre el presupuesto[/]")
    for warning in warnings:
        console.print(f"[yellow]! {warning}[/]")
    if dry_run:
        console.print("[dim]Dry-run: no se iniciarán agentes.[/]")
        return
    if not force:
        console.print("[red]Inicio cancelado. Repite con --force para confirmar.[/]")
        raise typer.Exit(code=2)
    console.print("[yellow]--force confirma el inicio pese a los avisos.[/]")
