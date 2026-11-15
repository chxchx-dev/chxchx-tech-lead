import json
from threading import Event
from pathlib import Path

from chxchx_tech_lead.adapters.terminal.subprocess import SubprocessAdapter
from chxchx_tech_lead.adapters.terminal.zellij import ZellijAdapter
from chxchx_tech_lead.core.runner import CommandResult


def test_zellij_adapter_parses_sessions_and_generates_pane_command(tmp_path: Path):
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        if command[-1] == "list-sessions":
            return CommandResult(list(command), 0, "demo [Created 2026-09-26 12:00]\nother\n", "")
        return CommandResult(list(command), 0, "created", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")

    assert adapter.session_exists("demo") is True
    assert adapter.session_exists("missing") is False
    result = adapter.run_in_session("demo", ["npm", "run", "dev"], tmp_path, pane_name="frontend")

    assert result.returncode == 0
    assert result.command[-5:] == ["--name", "frontend", "--", "npm", "run", "dev"][-5:]
    assert result.command[:5] == ["zellij", "--session", "demo", "action", "new-pane"]


def test_zellij_adapter_matches_colored_session_names():
    def fake_runner(command, **kwargs):
        return CommandResult(
            list(command),
            0,
            "\x1b[32;1mbokana\x1b[m [Created 2m ago] (\x1b[31;1mEXITED\x1b[m - attach to resurrect)",
            "",
        )

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")

    assert adapter.session_status("bokana") == "exited"


def test_zellij_status_probe_has_a_short_timeout():
    calls = []

    def fake_runner(command, **kwargs):
        calls.append(kwargs)
        return CommandResult(list(command), 0, "demo\n", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")

    assert adapter.session_exists("demo")
    assert calls[0]["timeout"] == 3


def test_zellij_pane_direction_can_be_configured(tmp_path: Path):
    calls = []

    def fake_runner(command, **kwargs):
        calls.append(command)
        return CommandResult(list(command), 0, "started", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")
    adapter.run_in_session("demo", ["codex"], tmp_path, direction="right")

    assert "--direction" in calls[0]
    assert calls[0][calls[0].index("--direction") + 1] == "right"


def test_zellij_exited_session_is_not_active_and_is_recreated(tmp_path: Path):
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        if command[-1] == "list-sessions":
            output = "demo [Created] (EXITED - attach to resurrect)\n" if len(calls) < 4 else ""
            return CommandResult(list(command), 0, output, "")
        return CommandResult(list(command), 0, "created", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")

    assert adapter.session_status("demo") == "exited"
    assert adapter.session_exists("demo") is False
    result = adapter.create_session("demo", tmp_path, layout="layout { pane }")

    assert result.returncode == 0
    assert [command for command, _kwargs in calls if command[:2] == ["zellij", "delete-session"]] == [
        ["zellij", "delete-session", "--force", "demo"]
    ]


def test_zellij_attach_is_marked_interactive():
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        return CommandResult(list(command), 0, "", "")

    ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij").attach_session("demo")

    assert calls[0][1]["interactive"] is True
    assert calls[0][0] == ["zellij", "attach", "--force-run-commands", "demo"]


def test_zellij_attach_focuses_tab_and_agent_after_client_connects():
    calls = []
    attached = Event()
    focused_pane = Event()
    layout_added = Event()

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        if command[:2] == ["zellij", "attach"]:
            attached.set()
            assert focused_pane.wait(2)
            return CommandResult(list(command), 0, "attached", "")
        if "new-tab" in command:
            layout_added.set()
            return CommandResult(list(command), 0, "1", "")
        if "list-tabs" in command:
            active = attached.is_set()
            tabs = [{"name": "Terminales", "active": active}]
            if layout_added.is_set():
                tabs.append({"name": "Agentes", "active": active, "selectable_tiled_panes_count": 1})
            return CommandResult(
                list(command),
                0,
                json.dumps(tabs),
                "",
            )
        if "list-panes" in command:
            return CommandResult(
                list(command),
                0,
                json.dumps(
                    [
                        {
                            "id": 8,
                            "tab_name": "Agentes",
                            "pane_command": "python -m chxchx_tech_lead.workspace.agent_pane --name codex -- codex",
                        }
                    ]
                ),
                "",
            )
        if "focus-pane-id" in command:
            focused_pane.set()
        return CommandResult(list(command), 0, "", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")
    result = adapter.attach_session(
        "demo",
        focus_tab="Agentes",
        focus_pane="codex",
        tab_layout="layout { pane }",
    )

    assert result.returncode == 0
    focus_tab = [command for command, _kwargs in calls if "go-to-tab-name" in command]
    assert focus_tab == [["zellij", "--session", "demo", "action", "go-to-tab-name", "--create", "Agentes"]]
    assert any(command[-2:] == ["focus-pane-id", "8"] for command, _kwargs in calls)
    assert any("new-tab" in command and "--name" in command for command, _kwargs in calls)


def test_zellij_add_layout_tab_names_new_tab():
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        return CommandResult(list(command), 0, "1", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")
    result = adapter.add_layout_tab("demo", "Agentes", "layout { pane }")

    assert result.returncode == 0
    assert calls[0][0][:7] == [
        "zellij",
        "--session",
        "demo",
        "action",
        "new-tab",
        "--name",
        "Agentes",
    ]
    assert calls[0][0][-2] == "--layout"


def test_zellij_lists_all_panes_for_agent_status():
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        return CommandResult(list(command), 0, "PANE_ID TITLE COMMAND\n1 codex codex\n", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")
    result = adapter.list_panes("demo")

    assert result.returncode == 0
    assert calls[0][0] == ["zellij", "--session", "demo", "action", "list-panes", "--all"]


def test_zellij_can_restore_last_focused_pane():
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        return CommandResult(list(command), 0, "focused", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")
    result = adapter.focus_last_pane("demo")

    assert result.returncode == 0
    assert calls[0][0] == ["zellij", "--session", "demo", "action", "focus-last-pane"]


def test_zellij_focuses_named_terminal_pane():
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        if command[-1] == "--json":
            return CommandResult(
                list(command),
                0,
                '[{"pane_id":"terminal_7","pane_name":"terminal"}]',
                "",
            )
        return CommandResult(list(command), 0, "focused", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")
    result = adapter.focus_terminal_pane("demo")

    assert result.returncode == 0
    assert calls[1][0] == ["zellij", "--session", "demo", "action", "focus-pane-id", "terminal_7"]

def test_zellij_focuses_selected_agent_pane():
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        if command[-1] == "--json":
            return CommandResult(
                list(command),
                0,
                '[{"pane_id":"codex_3","pane_name":"codex"}]',
                "",
            )
        return CommandResult(list(command), 0, "focused", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")
    result = adapter.focus_named_pane("demo", "codex")

    assert result.returncode == 0
    assert calls[1][0] == ["zellij", "--session", "demo", "action", "focus-pane-id", "codex_3"]


def test_zellij_focuses_shell_from_documented_json_fields():
    raw = '[{"id":1,"is_plugin":false,"title":"/bin/bash","pane_command":"bash"}]'

    assert ZellijAdapter._find_terminal_pane_id(raw) == "1"


def test_zellij_session_can_start_with_a_layout(tmp_path: Path):
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        return CommandResult(list(command), 0, "created", "")

    adapter = ZellijAdapter(runner=fake_runner, lookup=lambda _: "/usr/bin/zellij")
    result = adapter.create_session("demo", tmp_path, layout='layout { pane }')

    assert result.returncode == 0
    create_call = calls[-1][0]
    assert create_call[:3] == ["zellij", "--layout-string", "layout { pane }"]
    assert create_call[-2:] == ["--create-background", "demo"]


def test_subprocess_adapter_is_a_non_persistent_fallback(tmp_path: Path):
    calls = []

    def fake_runner(command, **kwargs):
        calls.append((command, kwargs))
        return CommandResult(list(command), 0, "ok", "")

    adapter = SubprocessAdapter(runner=fake_runner)
    result = adapter.run_in_session("demo", ["python", "--version"], tmp_path)

    assert adapter.available() is True
    assert adapter.session_exists("demo") is False
    assert result.returncode == 0
    assert calls[0][1]["cwd"] == tmp_path
