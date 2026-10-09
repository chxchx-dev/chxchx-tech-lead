"""Typer app tree for the public CLI command groups."""

import typer


app = typer.Typer(
    no_args_is_help=True,
    help="Bootstrapper personal para tu entorno de desarrollo asistido por agentes.",
)
projects_app = typer.Typer(no_args_is_help=True, help="Administra proyectos registrados.")
workspace_app = typer.Typer(no_args_is_help=True, help="Inspecciona y administra el workspace del proyecto.")
process_app = typer.Typer(no_args_is_help=True, help="Administra procesos configurados del workspace.")
editor_app = typer.Typer(no_args_is_help=True, help="Administra el editor configurado.")
agent_app = typer.Typer(no_args_is_help=True, help="Administra agentes configurados.")
skill_app = typer.Typer(no_args_is_help=True, help="Busca y recomienda skills del proyecto.")
pack_app = typer.Typer(no_args_is_help=True, help="Detecta y aplica Tech Packs de skills.")
bridge_app = typer.Typer(no_args_is_help=True, help="API JSON local para interfaces de ChxChx.")

app.add_typer(projects_app, name="projects")
app.add_typer(workspace_app, name="workspace")
app.add_typer(process_app, name="process")
app.add_typer(editor_app, name="editor")
app.add_typer(agent_app, name="agent")
app.add_typer(skill_app, name="skill")
app.add_typer(pack_app, name="pack")
app.add_typer(bridge_app, name="bridge")
