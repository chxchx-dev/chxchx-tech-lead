"""Workspace TUI mount, teardown, and visible-panel refresh lifecycle."""

from __future__ import annotations

from textual.app import ScreenStackError
from textual.widgets import DataTable, Static, TabbedContent


def mount_workspace(app) -> None:
    app.sub_title = app.subtitle
    setup_tables(app)
    setup_panel_titles(app)
    app._refresh_project_console()
    app.set_interval(1, app._refresh_project_output)
    app.refresh_dashboard()


def unmount_workspace(app) -> None:
    for timer in (
        app._tab_refresh_timer,
        app._chat_search_timer,
        app._memory_search_timer,
    ):
        if timer is not None:
            timer.stop()
    app._tab_refresh_timer = None
    app._chat_search_timer = None
    app._memory_search_timer = None


def activate_tab(app, event: TabbedContent.TabActivated) -> None:
    if event.tabbed_content.id not in {"tabs", "work-tabs", "more-tabs"}:
        return
    if app._tab_refresh_timer is not None:
        app._tab_refresh_timer.stop()
    app._tab_refresh_timer = app.set_timer(0.05, lambda: refresh_visible_panel(app))


def refresh_visible_panel(app) -> None:
    app._tab_refresh_timer = None
    if app._palette_open or app._governor_pending:
        return
    try:
        section = app._query("#tabs", TabbedContent).active
    except ScreenStackError:
        return
    if section == "projects":
        app._refresh_projects()
    elif section == "skills":
        app._refresh_skills()
    elif section == "work":
        if app._query("#work-tabs", TabbedContent).active == "console":
            app._refresh_project_console()
        else:
            app.refresh_dashboard()
    elif section == "more":
        active = app._query("#more-tabs", TabbedContent).active
        if active == "resources":
            app.refresh_dashboard()
        elif active == "skills":
            app._refresh_skills()
        elif active == "handoff":
            app._refresh_handoff()
        elif active == "memory":
            app._refresh_memory_history()
        elif active == "conversations":
            app._refresh_conversations()
        elif active == "errors":
            app._refresh_errors()


def setup_tables(app) -> None:
    columns = {
        "#overview-processes": ("ID", "Estado", "PID", "Puerto", "RAM", "CPU"),
        "#processes-table": ("ID", "Estado", "PID", "Puerto", "RAM", "CPU"),
        "#chat-list": ("Agente", "Último uso", "Chat", "Último mensaje"),
        "#projects-table": ("Actual", "Alias", "Proyecto", "Estado", "Perfil", "Ruta"),
        "#agents-table": ("Agente", "CLI", "Disponible", "Sesión", "Pane", "Preset"),
        "#resources-processes": ("Proceso", "Estado", "PID", "RAM RSS", "CPU"),
        "#resources-all-projects": (
            "Actual", "Alias", "Proyecto", "Estado", "Activos", "RAM RSS", "CPU",
        ),
        "#memory-list": ("Nota", "Modificada", "Resumen"),
        "#console-processes": ("ID", "Comando configurado", "Auto al abrir", "Estado"),
        "#errors-table": ("Fecha UTC", "Proyecto", "Acción", "Error"),
        "#skills-table": ("Activa", "Skill", "Stack", "Descripción"),
        "#packs-table": ("Tech Pack", "Coincidencias", "Skills"),
    }
    for selector, headings in columns.items():
        app._query(selector, DataTable).add_columns(*headings)


def setup_panel_titles(app) -> None:
    titles = {
        "#summary": "WORKSPACE",
        "#overview-processes": "PROCESOS",
        "#overview-resources": "RECURSOS",
        "#projects-table": "PROYECTOS",
        "#agents-table": "AGENTES",
        "#console-processes": "COMANDOS DEL PROYECTO",
        "#console-output": "TERMINAL · SALIDA EN VIVO",
        "#processes-table": "PROCESOS",
        "#resources-detail": "SISTEMA",
        "#resources-project-summary": "PROYECTO",
        "#resources-processes": "PROCESOS",
        "#resources-all-summary": "GLOBAL",
        "#resources-all-projects": "PROYECTOS",
        "#memory-list": "NOTAS",
        "#memory-detail": "DETALLE",
        "#chat-list": "CHATS",
        "#chat-detail": "CONVERSACIÓN",
        "#errors-table": "ERRORES RECIENTES",
        "#error-detail": "DETALLE DEL ERROR",
        "#skills-table": "CATÁLOGO",
        "#packs-table": "RECOMENDADOS PARA EL PROYECTO",
        "#skill-detail": "DETALLE DE SKILL",
        "#pack-detail": "DETALLE DE TECH PACK",
        "#brand-banner": "CHXCHX",
    }
    for selector, title in titles.items():
        app.screen.query_one(selector).border_title = title
    app._query("#handoff", Static).border_title = "HANDOFF"
