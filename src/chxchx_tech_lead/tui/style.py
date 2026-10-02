TUI_CSS = """
    Screen {
        background: #282a36;
        color: #ececf1;
    }
    Header {
        background: #282a36;
        color: #ececf1;
        border-bottom: solid #777b8e;
    }
    HeaderIcon, HeaderClock {
        background: #282a36;
        color: #aeb1c2;
    }
    Footer {
        background: #282a36;
        color: #aeb1c2;
        border-top: solid #777b8e;
    }
    FooterKey {
        background: #282a36;
        color: #aeb1c2;
    }
    #tabs { height: 1fr; background: #282a36; }
    ContentTabs {
        background: #282a36;
        border-bottom: solid #777b8e;
    }
    Tab {
        background: #282a36;
        color: #aeb1c2;
        padding: 0 2;
    }
    Tab:hover {
        background: #353746;
        color: #ffffff;
    }
    Tab.-active, Tabs:focus Tab.-active {
        background: #3a3d4d;
        color: #ffffff;
        text-style: bold;
    }
    Underline { display: none; }
    TabPane {
        background: #282a36;
        padding: 1 2;
    }
    .summary {
        height: auto;
        min-height: 3;
        margin-bottom: 1;
        padding: 0 1;
        background: #2e303d;
        border: solid #777b8e;
    }
    .usage-guide {
        height: 1fr;
        min-height: 12;
        padding: 1 2;
        background: #2e303d;
        border: solid #777b8e;
        overflow-y: auto;
    }
    .input-label {
        width: auto;
        padding: 1 1 0 0;
        color: #aeb1c2;
    }
    .handoff-label {
        height: 1;
        padding: 0 1;
        color: #aeb1c2;
    }
    #brand {
        height: 1;
        padding: 0 1;
        color: #ddd6ff;
        text-style: bold;
    }
    #brand-banner {
        height: auto;
        padding: 1 2;
        background: #2e303d;
        color: #ececf1;
        border: solid #aeb1c2;
    }
    #memory-list { width: 48%; min-width: 30; }
    #chat-list { width: 55%; min-width: 35; }
    #console-processes { height: 10; min-height: 5; margin-bottom: 1; }
    #console-output {
        height: 1fr;
        min-height: 8;
        padding: 1 2;
        background: #20222c;
        color: #d8dae4;
        border: solid #aeb1c2;
        overflow-y: auto;
    }
    #chat-detail, #memory-detail, #error-detail {
        width: 1fr;
        height: 1fr;
        margin-left: 1;
        padding: 1 2;
        background: #2e303d;
        border: solid #777b8e;
        overflow-y: auto;
    }
    .toolbar {
        height: auto;
        margin: 0 0 1 0;
        overflow-x: auto;
    }
    .toolbar Button { margin-right: 1; }
    Button, Button.-primary, Button.-success, Button.-error {
        width: auto;
        min-width: 12;
        height: 3;
        padding: 0 1;
        background: #303240;
        color: #ececf1;
        border: solid #818598;
        text-style: none;
    }
    Button.-primary { border: solid #b4a5f2; color: #ddd6ff; }
    Button.-success { border: solid #8bbdad; color: #b9ded1; }
    Button.-error { border: solid #d49ca6; color: #f1c3cb; }
    Button:hover, Button.-primary:hover, Button.-success:hover, Button.-error:hover,
    Button:focus, Button.-primary:focus, Button.-success:focus, Button.-error:focus {
        background: #444758;
        color: #ffffff;
        border: solid #ececf1;
        text-style: bold;
    }
    Button:disabled { color: #777b8e; border: solid #555868; }
    Input {
        height: 3;
        background: #303240;
        color: #ececf1;
        border: solid #777b8e;
    }
    Input:focus { border: solid #d6caff; background: #353746; }
    .wide { width: 1fr; height: 1fr; }
    .side-panel {
        width: 30;
        height: 1fr;
        margin-left: 1;
        padding: 1 2;
        background: #2e303d;
        border: solid #777b8e;
    }
    DataTable {
        height: 1fr;
        min-height: 8;
        background: #2e303d;
        color: #ececf1;
        border: solid #777b8e;
    }
    DataTable > .datatable--header {
        background: #3a3d4d;
        color: #ececf1;
        text-style: bold;
    }
    DataTable > .datatable--even-row { background: #303240; }
    DataTable > .datatable--odd-row { background: #2e303d; }
    DataTable > .datatable--cursor, DataTable:focus > .datatable--cursor {
        background: #555970;
        color: #ffffff;
    }
    DataTable > .datatable--hover { background: #444758; }
    #log {
        height: 3;
        padding: 0 2;
        background: #303240;
        color: #d6caff;
        border-top: solid #777b8e;
    }
    #project-ref { width: 1fr; }
    #errors-table { height: 1fr; }
    Static#handoff {
        height: 1fr;
        padding: 1 2;
        background: #2e303d;
        border: solid #777b8e;
    }
    .field { width: 1fr; }
    """

TUI_BINDINGS = [
        ("q", "quit", "Salir"),
        ("ctrl+p", "command_palette", "Comandos"),
        ("r", "refresh", "Actualizar"),
        ("1", "show_overview", "Resumen"),
        ("2", "show_projects", "Proyectos"),
        ("3", "show_agents", "Agentes"),
        ("4", "show_processes", "Procesos"),
        ("5", "show_resources", "Recursos"),
        ("6", "show_handoff", "Handoff"),
        ("7", "show_memory", "Notas"),
        ("8", "show_conversations", "Chats"),
        ("9", "show_brand", "Marca"),
        ("0", "show_setup", "Configuración"),
        ("f1", "show_guide", "Guía de uso"),
        ("f2", "show_errors", "Errores"),
        ("g", "attach_agent_terminal", "Terminal del agente"),
        ("o", "open_workspace", "Abrir"),
        ("j", "attach_workspace", "Terminales"),
        ("n", "open_terminal", "Nueva terminal Zellij"),
        ("y", "trust_workspace", "Confiar"),
        ("s", "start_project", "Iniciar proyecto"),
        ("t", "start_workspace_all", "Abrir Zellij + agentes"),
        ("a", "start_agents", "Agentes"),
        ("c", "start_agent_selected", "Agente"),
        ("i", "start_process", "Proceso"),
        ("k", "stop_process", "Detener proceso"),
        ("u", "suspend_workspace", "Suspender"),
        ("v", "resume_workspace", "Reanudar"),
        ("x", "stop_workspace", "Detener"),
        ("h", "write_handoff", "Guardar handoff"),
        ("e", "open_editor", "Sublime"),
    ]
