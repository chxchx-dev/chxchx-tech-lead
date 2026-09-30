from __future__ import annotations

from ..cli_context import (
    Path,
    Table,
    WorkspaceManager,
    app,
    backup_project,
    basic_memory_available,
    check_tools,
    claude_body,
    console,
    create_project_structure,
    diagnose_project_mcp,
    ensure_memory_project,
    ensure_project_config,
    install_tool,
    project_config_needs_update,
    project_rule_body,
    register_project,
    typer,
    upsert_managed_block,
    write_opencode_example,
    _opencode_example_needs_update,
    _project,
    _show_project,
)

@app.command("install")
def install_command(
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra acciones sin ejecutarlas."),
):
    """Instala las herramientas base administradas por uv."""
    for tool in ("basic-memory", "serena"):
        console.print(f"[cyan]→[/] {tool}")
        try:
            result = install_tool(tool, dry_run=dry_run)
        except RuntimeError as exc:
            console.print(f"[red]✗[/] {exc}")
            raise typer.Exit(code=1)
        if result.returncode == 0:
            if result.skipped:
                console.print(f"[green]✓[/] {tool} ya está instalado")
            else:
                console.print(f"[green]✓[/] {'DRY RUN ' if dry_run else ''}{' '.join(result.command)}")
        else:
            console.print(f"[red]✗[/] {' '.join(result.command)}")
            if result.stderr:
                console.print(result.stderr)

@app.command("init")
def init_command(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Muestra cambios sin escribirlos."),
    no_backup: bool = typer.Option(False, "--no-backup", help="No crea backup antes de modificar archivos administrados."),
    minimal: bool = typer.Option(
        False,
        "--minimal",
        help="Deja solo .ai/chxchx-tech.toml y .ai/memory en el proyecto.",
    ),
):
    """Inicializa el proyecto; usa --minimal para una huella compacta."""
    info = _project(path)
    _show_project(info)

    planned_structure = create_project_structure(info, dry_run=True, minimal=minimal)
    planned_config = project_config_needs_update(info)
    planned_agents = False
    planned_claude = False
    planned_opencode = False
    if not minimal:
        planned_agents = upsert_managed_block(
            info.root / "AGENTS.md", "project-rules", project_rule_body(info), dry_run=True
        )
        planned_claude = upsert_managed_block(
            info.root / "CLAUDE.md", "claude-rules", claude_body(), dry_run=True
        )
        planned_opencode = _opencode_example_needs_update(info)

    if not dry_run and not no_backup and any(
        (planned_structure, planned_config, planned_agents, planned_claude, planned_opencode)
    ):
        backup = backup_project(info.root)
        if backup:
            console.print(f"[dim]Backup: {backup}[/]")

    actions = create_project_structure(info, dry_run=dry_run, minimal=minimal)
    planned_memory = not (info.root / ".ai" / "memory").exists()
    ensure_project_config(info, dry_run=dry_run)
    if planned_config or planned_memory:
        actions.append("ensure .ai/chxchx-tech.toml + .ai/memory")
    if basic_memory_available():
        memory_result = ensure_memory_project(info, dry_run=dry_run)
        if memory_result is not None:
            if memory_result.returncode != 0:
                console.print(f"[yellow]! Basic Memory project:[/] {memory_result.stderr or memory_result.stdout}")
            elif not memory_result.skipped:
                actions.append("ensure Basic Memory project")
    else:
        console.print("[yellow]! Basic Memory no está instalado; ejecuta `chxchx-tech install`.[/]")

    if not minimal:
        if upsert_managed_block(info.root / "AGENTS.md", "project-rules", project_rule_body(info), dry_run=dry_run):
            actions.append("update AGENTS.md")
        if upsert_managed_block(info.root / "CLAUDE.md", "claude-rules", claude_body(), dry_run=dry_run):
            actions.append("update CLAUDE.md")
    if register_project(info, dry_run=dry_run):
        actions.append("register project")
    if not minimal and write_opencode_example(info, dry_run=dry_run):
        actions.append("ensure OpenCode MCP example")

    if actions:
        for action in actions:
            console.print(f"[green]✓[/] {action}")
    else:
        console.print("[green]✓ Todo está actualizado.[/]")

    if dry_run:
        console.print("[yellow]No se realizaron cambios (--dry-run).[/]")

@app.command()
def setup(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
    dry_run: bool = typer.Option(False, "--dry-run", help="Previsualiza instalación, init e integraciones."),
    minimal: bool = typer.Option(False, "--minimal", help="Usa la preparación compacta del proyecto."),
):
    """Flujo completo: herramientas base + init del proyecto + MCP."""
    console.print("[bold cyan]1/3 Herramientas base[/]")
    install_command(dry_run=dry_run)
    console.print("\n[bold cyan]2/3 Inicialización del proyecto[/]")
    init_command(path=path, dry_run=dry_run, no_backup=False, minimal=minimal)
    console.print("\n[bold cyan]3/3 Integraciones MCP[/]")
    from .integrations import integrate as integrate_command

    integrate_command(path=path, client="all", dry_run=dry_run, refresh=False)

@app.command()
def doctor(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
):
    """Diagnostica herramientas globales y configuración del proyecto."""
    table = Table(title="ChxChx Tech Lead — Doctor")
    table.add_column("Componente")
    table.add_column("Estado")
    table.add_column("Comando")
    for item in check_tools():
        table.add_row(item.name, "✓" if item.installed else "✗", item.command)
    console.print(table)

    info = _project(path)
    console.print()
    _show_project(info)
    checks = {
        "AGENTS.md": info.root / "AGENTS.md",
        "CLAUDE.md": info.root / "CLAUDE.md",
        ".ai": info.root / ".ai",
        ".ai/chxchx-tech.toml": info.root / ".ai" / "chxchx-tech.toml",
        ".ai/memory": info.root / ".ai" / "memory",
        ".ai/integrations/opencode-mcp.example.json": info.root / ".ai" / "integrations" / "opencode-mcp.example.json",
        "docs/adr": info.root / "docs" / "adr",
    }
    for label, item in checks.items():
        mark = "[green]✓[/]" if item.exists() else "[yellow]![/]"
        console.print(f"{mark} {label}")
    if (info.root / ".ai" / "chxchx-tech.toml").exists() and project_config_needs_update(info):
        console.print("[yellow]! .ai/chxchx-tech.toml está desactualizado; ejecuta `chxchx-tech init`.[/]")

    console.print("\n[bold]WORKSPACE[/]")
    inspection = WorkspaceManager(info).inspect()
    config_path = info.root / ".ai" / "chxchx-tech.toml"
    if not config_path.exists():
        console.print("[yellow]! .ai/chxchx-tech.toml no existe; ejecuta `chxchx-tech init`.[/]")
    elif inspection.error:
        console.print(f"[red]✗ Configuración workspace:[/] {inspection.error}")
    else:
        console.print(f"[green]✓[/] .ai/chxchx-tech.toml válido")
        trust_mark = "[green]✓[/]" if inspection.trusted else "[yellow]![/]"
        trust_text = "confiable" if inspection.trusted else "no confiable; ejecuta `chxchx-tech workspace trust`"
        console.print(f"{trust_mark} trust: {trust_text}")
        if inspection.config is not None:
            for item in (*inspection.config.processes, *inspection.config.agents):
                cwd = (info.root / item.cwd).resolve()
                mark = "[green]✓[/]" if cwd.is_dir() else "[red]✗[/]"
                console.print(f"{mark} {item.id} cwd: {cwd}")
        for warning in inspection.warnings:
            console.print(f"[yellow]! {warning}[/]")

    console.print("\n[bold]MCP POR PROYECTO[/]")
    diagnostics = Table(title="Basic Memory y Serena · configuración (solo lectura)")
    diagnostics.add_column("Cliente")
    diagnostics.add_column("Servidor")
    diagnostics.add_column("Estado")
    diagnostics.add_column("Alcance")
    diagnostics.add_column("Detalle")
    for item in diagnose_project_mcp(info):
        style = {"OK": "green", "AVISO": "yellow", "FALTA": "yellow", "ERROR": "red"}.get(
            item.status, "white"
        )
        diagnostics.add_row(
            item.client,
            item.server,
            f"[{style}]{item.status}[/{style}]",
            item.scope,
            item.detail,
        )
    console.print(diagnostics)
    console.print(
        "[dim]Solo lectura: no inicia agentes/servidores ni imprime variables de entorno. "
        "Codex debe confiar en el proyecto para cargar su configuración local.[/]"
    )

@app.command()
def status(
    path: Path = typer.Argument(Path.cwd(), exists=True, file_okay=False, resolve_path=True),
):
    """Resume el proyecto actual."""
    info = _project(path)
    _show_project(info)
    console.print(f"[bold]AI init:[/] {'sí' if (info.root / '.ai').exists() else 'no'}")
    console.print(f"[bold]AGENTS.md:[/] {'sí' if (info.root / 'AGENTS.md').exists() else 'no'}")
    console.print(f"[bold]CLAUDE.md:[/] {'sí' if (info.root / 'CLAUDE.md').exists() else 'no'}")
