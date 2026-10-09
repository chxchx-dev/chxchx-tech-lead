from __future__ import annotations

from pathlib import Path

import typer

from ..ui.cli_output import console, print_agent_statuses as _print_agent_statuses
from ..cli_registry import agent_app, workspace_app
from ..workspace.handoff import update_handoff
from ..workspace.project_context import workspace_service_for as _workspace_service
from ..workspace.service import WorkspaceOperationError
from .resource_guard import guard_cli_agent_start

@agent_app.command("start")
def agent_start(
    agent_id: str | None = typer.Argument(None, metavar="ID"),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el inicio sin ejecutarlo."),
    all_agents: bool = typer.Option(False, "--all", help="Inicia todos los agentes configurados."),
    preset: str | None = typer.Option(None, "--preset", help="Inicia el preset declarado en el proyecto."),
    new_chat: bool = typer.Option(False, "--new-chat", help="Abre una conversación nueva recuperando el contexto persistido del proyecto."),
    force: bool = typer.Option(False, "--force", help="Confirma el inicio cuando excede el presupuesto RAM/agentes."),
):
    """Inicia un agente, o todos los agentes configurados, dentro del workspace."""
    selected = sum(value is not None for value in (agent_id, preset)) + int(all_agents)
    if selected > 1:
        console.print("[red]✗ Usa un ID, `--all` o `--preset`, no varios.[/]")
        raise typer.Exit(code=2)
    if selected == 0:
        console.print("[red]✗ Indica un ID, usa `--all` o `--preset`.[/]")
        raise typer.Exit(code=2)
    try:
        service = _workspace_service(path)
        guard_cli_agent_start(
            service,
            (agent_id,) if agent_id else None,
            preset_id=preset,
            new_chat=new_chat,
            dry_run=dry_run,
            force=force,
        )
        if preset is not None:
            if new_chat:
                console.print("[red]✗ --new-chat se admite con un ID o --all, no con --preset.[/]")
                raise typer.Exit(code=2)
            results = service.start_preset(preset, dry_run=dry_run)
        elif all_agents:
            results = service.start_agents(dry_run=dry_run, new_chat=new_chat)
        else:
            result = service.start_agent(agent_id, dry_run=dry_run, new_chat=new_chat)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    except ValueError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    if preset is not None or all_agents:
        for current_id, current_result in results:
            console.print(f"[green]✓[/] {current_id}: {'DRY RUN: ' if dry_run else ''}{' '.join(current_result.command)}")
    else:
        console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}{' '.join(result.command)}")


@agent_app.command("attach")
def agent_attach(
    agent_id: str = typer.Argument(..., metavar="ID"),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Previsualiza la preparación y el attach."),
    force: bool = typer.Option(False, "--force", help="Confirma el inicio pese a avisos del RAM Governor."),
):
    """Prepara el workspace y adjunta una terminal al pane del agente."""
    try:
        service = _workspace_service(path)
        guard_cli_agent_start(service, dry_run=dry_run, force=force)
        service.prepare_agents(dry_run=dry_run)
        result = service.attach_agent(agent_id, dry_run=dry_run)
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    except ValueError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}{' '.join(result.command)}")


@agent_app.command("preflight")
def agent_preflight(
    agent_ids: list[str] = typer.Option(..., "--agent", help="ID de cada agente que Studio abrirá."),
    path: Path = typer.Option(Path.cwd(), "--path", exists=True, file_okay=False, resolve_path=True),
    additional_active: int = typer.Option(0, "--additional-active", min=0,
        help="Sesiones de agente que Studio ya mantiene abiertas."),
):
    """Verifica trust y RAM para una o varias sesiones embebidas; no ejecuta agentes."""
    try:
        service = _workspace_service(path)
        inspection = service.inspect()
        if not inspection.trusted:
            raise WorkspaceOperationError(
                "El proyecto no tiene trust. Márcalo como confiable antes de iniciar agentes."
            )
        if inspection.config is None:
            raise WorkspaceOperationError("No hay configuración válida de workspace")
        configured = {agent.id: agent for agent in inspection.config.agents}
        unknown = sorted(set(agent_ids) - configured.keys())
        if unknown:
            raise WorkspaceOperationError(f"Agentes no configurados: {', '.join(unknown)}")
        shell_agents = [agent_id for agent_id in agent_ids if configured[agent_id].shell]
        if shell_agents:
            raise WorkspaceOperationError(
                f"Studio no integra comandos shell personalizados: {', '.join(shell_agents)}"
            )
        guard_cli_agent_start(
            service,
            tuple(agent_ids),
            new_chat=True,
            dry_run=True,
            additional_active=additional_active,
        )
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    if not agent_ids:
        console.print("[red]✗ Indica al menos un --agent.[/]")
        raise typer.Exit(code=2)
    console.print(f"[green]✓[/] Preflight Studio listo: {', '.join(agent_ids)}. No se inició ningún proceso.")

@agent_app.command("list")
def agent_list(
    path: str = typer.Option(".", "--path"),
):
    """Lista disponibilidad y estado de los agentes sin iniciarlos."""
    try:
        _print_agent_statuses(_workspace_service(path).agent_statuses())
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)

@agent_app.command("status")
def agent_status(
    path: str = typer.Option(".", "--path"),
):
    """Alias explícito de `agent list`."""
    agent_list(path=path)

@workspace_app.command("handoff")
def workspace_handoff(
    path: str = typer.Argument("."),
    summary: str = typer.Option("Estado del workspace actualizado.", "--summary"),
    pending: str = typer.Option("Revisar el siguiente entregable.", "--pending"),
    validation: str = typer.Option("Ejecutar las pruebas relevantes antes de continuar.", "--validation"),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra el cambio sin escribirlo."),
):
    """Actualiza el bloque administrado del handoff del proyecto."""
    try:
        service = _workspace_service(path)
        inspection = service.inspect()
        changed = update_handoff(
            service.project,
            inspection.state.status,
            service.agent_statuses(),
            summary=summary,
            pending=pending,
            validation=validation,
            dry_run=dry_run,
        )
    except WorkspaceOperationError as exc:
        console.print(f"[red]✗ {exc}[/]")
        raise typer.Exit(code=1)
    console.print(f"[green]✓[/] {'DRY RUN: ' if dry_run else ''}{'handoff actualizado' if changed else 'handoff sin cambios'}")
