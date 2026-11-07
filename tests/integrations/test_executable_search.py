"""Agent CLIs are found where their installers put them, even when Studio started with a minimal PATH."""

from __future__ import annotations

import os
import stat
import sys
from pathlib import Path

import pytest

from chxchx_tech_lead.integrations import executable_search
from chxchx_tech_lead.integrations.agent_catalog import executable_available, resolve_executable
from chxchx_tech_lead.integrations.executable_search import extra_directories, find_executable, search_path, with_search_path

pytestmark = pytest.mark.skipif(os.name == "nt", reason="uses POSIX executables and folders")


def _install(folder: Path, name: str) -> Path:
    folder.mkdir(parents=True)
    program = folder / name
    program.write_text("#!/bin/sh\nexit 0\n", encoding="utf-8")
    program.chmod(program.stat().st_mode | stat.S_IXUSR)
    return program


@pytest.fixture
def launcher_environment(tmp_path: Path, monkeypatch) -> Path:
    """A home with nothing installed and the PATH a Dock or menu launcher gives."""
    monkeypatch.setattr(Path, "home", classmethod(lambda cls: tmp_path))
    monkeypatch.setenv("PATH", "/usr/bin:/bin")
    return tmp_path


def test_opencode_in_its_own_install_folder_is_found_without_the_shell_profile(launcher_environment: Path) -> None:
    assert not executable_available("opencode-xyz")
    program = _install(launcher_environment / ".opencode" / "bin", "opencode-xyz")

    assert executable_available("opencode-xyz") and find_executable("opencode-xyz") == str(program)
    assert resolve_executable("opencode-xyz") == str(program)


def test_what_is_already_on_path_wins_over_the_install_folders(launcher_environment: Path, monkeypatch) -> None:
    first = _install(launcher_environment / "first", "agente-xyz")
    _install(launcher_environment / ".local" / "bin", "agente-xyz")
    monkeypatch.setenv("PATH", f"{first.parent}{os.pathsep}/usr/bin")

    assert find_executable("agente-xyz") == str(first)


def test_nvm_versions_are_searched_newest_first_and_missing_folders_are_ignored(launcher_environment: Path) -> None:
    old = _install(launcher_environment / ".nvm" / "versions" / "node" / "v18.0.0" / "bin", "claude-xyz")
    new = _install(launcher_environment / ".nvm" / "versions" / "node" / "v22.1.0" / "bin", "claude-xyz")

    assert find_executable("claude-xyz") == str(new) and old.exists()
    assert str(launcher_environment / ".bun" / "bin") not in search_path()  # not created: not added


def test_the_agent_gets_a_path_that_reaches_node_and_nothing_is_duplicated(launcher_environment: Path) -> None:
    _install(launcher_environment / ".nvm" / "versions" / "node" / "v22.1.0" / "bin", "node")

    environment = with_search_path({"PATH": "/usr/bin:/bin", "HOME": "/x"})
    again = with_search_path(environment)

    assert str(launcher_environment / ".nvm" / "versions" / "node" / "v22.1.0" / "bin") in environment["PATH"].split(os.pathsep)
    assert again["PATH"] == environment["PATH"] and environment["HOME"] == "/x"
    assert extra_directories(launcher_environment)  # the list is never empty


def test_the_chat_launches_the_cli_with_the_extended_path(launcher_environment: Path, isolated_home) -> None:
    """The agent is started with a PATH that holds its own folder, so its `#!/usr/bin/env node` finds node."""
    from types import SimpleNamespace

    from chxchx_tech_lead.workspace import agent_chat

    _install(launcher_environment / ".opencode" / "bin", "opencode")
    service = SimpleNamespace(
        inspect=lambda: SimpleNamespace(trusted=True, config=SimpleNamespace(agents=[SimpleNamespace(id="opencode", command=["opencode"], cwd=".")])),
        project=SimpleNamespace(root=launcher_environment),
    )

    plan = agent_chat.plan_chat_turn(service, "opencode", mode="read-only", session_id=None)

    assert plan.argv[0] == str(launcher_environment / ".opencode" / "bin" / "opencode")
