from __future__ import annotations

from pathlib import Path

from chxchx_tech_lead.core.runner import CommandResult
from chxchx_tech_lead.integrations import installers


def test_msi_helper_uses_the_shared_tools_directory(tmp_path: Path, monkeypatch) -> None:
    install_root = tmp_path / "Programs" / "ChxChxStudio" / "stable-1.2.3"
    python = install_root / ".venv" / "python" / "python.exe"
    python.parent.mkdir(parents=True)
    (python.parent / "uv.exe").touch()
    (python.parent / "uv.exe").touch()
    (install_root / ".chxchx-msi").write_text("", encoding="utf-8")
    monkeypatch.setattr(installers.sys, "executable", str(python))
    monkeypatch.setenv("UV_TOOL_BIN_DIR", str(tmp_path / "external-tools"))

    assert installers._tool_bin_directory() == install_root.parent / "tools"
    assert installers._uv_command() == str(python.parent / "uv.exe")
    assert installers._uv_command() == str(python.parent / "uv.exe")


def test_install_tool_passes_the_tool_directory_to_uv_and_restores_environment(tmp_path: Path, monkeypatch) -> None:
    tool_bin = tmp_path / "user-tools"
    uv = tmp_path / "uv" / "uv.exe"
    original_path = str(tmp_path / "original-path")
    monkeypatch.setenv("PATH", original_path)
    monkeypatch.delenv("UV_TOOL_BIN_DIR", raising=False)
    monkeypatch.setattr(installers, "_tool_bin_directory", lambda: tool_bin)
    monkeypatch.setattr(installers, "_uv_command", lambda: str(uv))
    monkeypatch.setattr(installers, "_tool_is_installed", lambda _name, _directory: False)
    observed: dict[str, object] = {}

    def fake_run(command, **kwargs):
        observed["command"] = command
        observed["path"] = installers.os.environ["PATH"]
        observed["tool_bin"] = installers.os.environ["UV_TOOL_BIN_DIR"]
        observed["kwargs"] = kwargs
        return CommandResult(list(command), 0, "ok", "")

    monkeypatch.setattr(installers, "run", fake_run)

    result = installers.install_tool("basic-memory")

    assert result.returncode == 0
    assert observed["command"] == [
        str(uv), "tool", "install", "-p", "3.12", "--prerelease", "allow", "basic-memory"
    ]
    assert str(tool_bin) in str(observed["path"]).split(installers.os.pathsep)
    assert str(uv.parent) in str(observed["path"]).split(installers.os.pathsep)
    assert observed["tool_bin"] == str(tool_bin)
    assert observed["kwargs"] == {"timeout": 1200}
    assert installers.os.environ["PATH"] == original_path
    assert "UV_TOOL_BIN_DIR" not in installers.os.environ


def test_install_tool_skips_an_existing_required_extra(tmp_path: Path, monkeypatch) -> None:
    tool_bin = tmp_path / "tools"
    tool_bin.mkdir()
    (tool_bin / "serena.exe").touch()
    monkeypatch.setattr(installers, "_tool_bin_directory", lambda: tool_bin)
    monkeypatch.setattr(installers, "_uv_command", lambda: None)

    result = installers.install_tool("serena")

    assert result.returncode == 0 and result.skipped
