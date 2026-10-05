from __future__ import annotations

from ..cli_context import (
    agent_app,
    Path,
    WorkspaceOperationError,
    console,
    typer,
    update_handoff,
    workspace_app,
    _print_agent_statuses,
    _workspace_service,
)
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
