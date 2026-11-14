"""Rich presentation helpers shared by the CLI command groups."""

from rich.console import Console
from rich.table import Table

from .theme import cli_theme


console = Console(theme=cli_theme())


def show_project(info) -> None:
    console.print(f"[bold]Proyecto:[/] {info.name}")
    console.print(f"[bold]Perfil:[/] {info.profile_name}")
    console.print(f"[bold]Stacks:[/] {', '.join(info.stacks)}")
    console.print(f"[bold]Lenguajes:[/] {', '.join(info.languages) or 'no detectados'}")


def print_recipe(action, agents, attached) -> None:
    print_workspace_action(action)
    if action.session_created and action.inspection.config and action.inspection.config.agents:
        console.print(
            f"[green]✓[/] agentes en layout: {', '.join(agent.id for agent in action.inspection.config.agents)}"
        )
    for agent_id, result in agents:
        console.print(f"[green]✓[/] agente {agent_id}: {' '.join(result.command)}")
    if attached is not None:
        console.print("[green]✓[/] sesión adjuntada")


def print_workspace_action(action) -> None:
    for message in action.messages:
        console.print(f"[green]✓[/] {message}")
    for result in action.process_results:
        console.print(f"[green]✓[/] {result.message}")


def print_agent_statuses(statuses) -> None:
    if not statuses:
        console.print("[yellow]No hay agentes configurados.[/]")
        return
    table = Table(title="Agentes")
    table.add_column("ID")
    table.add_column("Disponible")
    table.add_column("Sesión")
    table.add_column("Pane")
    table.add_column("Preset")
    table.add_column("Versión")
    for agent in statuses:
        table.add_row(
            agent.id,
            "✓" if agent.available else "✗",
            agent.session,
            agent.pane,
            agent.preset,
            (agent.version or "-").strip().splitlines()[0][:60],
        )
    console.print(table)
