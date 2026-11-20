"""Fixtures for the installer tests: a fake release served over HTTP, a fake `uv` and an isolated HOME."""

from __future__ import annotations

import http.server
import importlib.util
import io
import os
import shutil
import socketserver
import stat
import subprocess
import sys
import tarfile
import threading
from dataclasses import dataclass
from functools import partial
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parents[2]
SCRIPTS = REPO / "packaging"
spec = importlib.util.spec_from_file_location("release_assets", SCRIPTS / "release_assets.py")
release_assets = importlib.util.module_from_spec(spec)
sys.modules["release_assets"] = release_assets
spec.loader.exec_module(release_assets)

PNG = bytes.fromhex("89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d49444154789c6360000002000001e221bc330000000049454e44ae426082")
FAKE_STUDIO = """#!/usr/bin/env bash
echo "chxchx-studio {version} adapter=${{CHXCHX_AGENT_ADAPTER:-none}} args=$*"
"""
FAKE_UV = """#!/usr/bin/env bash
# A stand-in for `uv`: a plain venv, and a `chxchx-agent-adapter` command that speaks `contract`.
case "$1" in
  venv)
    target="${@: -1}"
    "$FAKE_PYTHON" -m venv --without-pip "$target"
    ;;
  pip)
    python=""
    wheel="${@: -1}"
    for ((i = 1; i <= $#; i++)); do
      if [ "${!i}" = "--python" ]; then j=$((i + 1)); python="${!j}"; fi
    done
    version="$(basename "$wheel" | sed -E 's/^chxchx_agent_adapter-([^-]+)-.*/\\1/')"
    printf '#!/usr/bin/env bash\\necho "{\\"schema\\":\\"chxchx.agent-adapter\\",\\"contract_version\\":1,\\"version\\":\\"%s\\",\\"providers\\":[\\"Codex\\",\\"OpenCode\\"]}"\\n' "$version" > "$(dirname "$python")/chxchx-agent-adapter"
    chmod +x "$(dirname "$python")/chxchx-agent-adapter"
    ;;
  tool)
    if [ "$2" != "install" ]; then echo "uv falso: $*" >&2; exit 2; fi
    mkdir -p "$UV_TOOL_BIN_DIR"
    case "${@: -1}" in
      basic-memory)
        for command in basic-memory bm; do
          printf '#!/usr/bin/env bash\\necho %s instalado\\n' "$command" > "$UV_TOOL_BIN_DIR/$command"
          chmod +x "$UV_TOOL_BIN_DIR/$command"
        done
        ;;
      serena-agent)
        printf '#!/usr/bin/env bash\\necho serena instalado\\n' > "$UV_TOOL_BIN_DIR/serena"
        chmod +x "$UV_TOOL_BIN_DIR/serena"
        ;;
      *) echo "uv falso: paquete desconocido ${@: -1}" >&2; exit 2 ;;
    esac
    ;;
  *) echo "uv falso: $*" >&2; exit 2 ;;
esac
"""


@dataclass
class Release:
    path: Path
    version: str
    commit: str
    channel: str

    @property
    def name(self) -> str:
        return self.path.name


def _executable(path: Path, text: str) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)
    return path


def _studio_tarball(path: Path, version: str) -> None:
    top = f"chxchx-studio-{version}-Linux-x86_64"
    with tarfile.open(path, "w:gz") as archive:
        def add(name: str, data: bytes, mode: int = 0o644) -> None:
            info = tarfile.TarInfo(f"{top}/{name}")
            info.size, info.mode = len(data), mode
            archive.addfile(info, io.BytesIO(data))
        add("bin/chxchx-studio", FAKE_STUDIO.format(version=version).encode(), 0o755)
        for name in ("chxchx-tech", "chxchx", "chxchx-future"):
            if name == "chxchx-tech":
                command = (
                    '#!/usr/bin/env bash\n'
                    'if [ "$1 $2" = "tools install" ]; then\n'
                    '  uv tool install -p 3.12 --prerelease allow basic-memory || exit $?\n'
                    '  uv tool install -p 3.13 serena-agent\n'
                    '  exit $?\n'
                    'fi\n'
                    f'echo "chxchx-tech {version} adapter=${{CHXCHX_AGENT_ADAPTER:-none}}"\n'
                )
            else:
                command = f'#!/usr/bin/env bash\necho "{name} {version} args=$*"\n'
            add(f"bin/{name}", command.encode(), 0o755)
        add("share/icons/hicolor/256x256/apps/chxchx-studio.png", PNG)
        add("qt.conf", b"[Paths]\n")


@pytest.fixture
def server_root(tmp_path: Path):
    """Serves tmp_path/releases over HTTP on 127.0.0.1 and yields (directory, base URL)."""
    directory = tmp_path / "releases"
    directory.mkdir()

    class Quiet(http.server.SimpleHTTPRequestHandler):
        def log_message(self, *args):  # keep test output clean
            pass

    handler = partial(Quiet, directory=str(directory))
    server = socketserver.ThreadingTCPServer(("127.0.0.1", 0), handler)
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    yield directory, f"http://127.0.0.1:{server.server_address[1]}"
    server.shutdown()
    server.server_close()


@pytest.fixture
def make_release(tmp_path: Path, server_root):
    directory, _base = server_root

    def build(*, version: str = "0.5.0", channel: str = "stable", commit: str | None = None, corrupt: bool = False) -> Release:
        commit = commit or ("1" if channel == "stable" else "2") * 20 + version.replace(".", "").ljust(20, "0")[:20]
        commit = commit[:40].ljust(40, "a")
        tag = "studio-dev-preview" if channel == "dev" else f"v{version}"
        label = f"{channel}-{version}-{commit[:6]}"
        root = tmp_path / f"repo-{label}"
        (root / "packaging").mkdir(parents=True)
        (root / "Cargo.toml").write_text(f'[package]\nname = "chxchx-tech"\nversion = "{version}"\n', encoding="utf-8")
        shutil.copy2(SCRIPTS / "install.sh", root / "packaging" / "install.sh")
        (root / "packaging" / "install.ps1").write_text("# placeholder\n", encoding="utf-8")
        downloads = tmp_path / f"downloads-{label}"
        for key, (folder, pattern, _name) in release_assets.PACKAGES.items():
            target = downloads / folder
            target.mkdir(parents=True, exist_ok=True)
            artifact = target / pattern.replace("*", f"package-{key}")
            if key.startswith("linux-"):
                _studio_tarball(artifact, version)
            else:
                artifact.write_bytes(key.encode())
        wheels = downloads / "agent-adapter-wheel"
        wheels.mkdir(parents=True)
        (wheels / f"chxchx_agent_adapter-{version}-py3-none-any.whl").write_bytes(b"fake wheel " + version.encode())
        output = directory / label
        release_assets.stage(root=root, downloads=downloads, output=output, channel=channel, tag=tag, commit=commit)
        if corrupt:  # the manifest keeps the original checksum, so this download must be rejected
            with (output / "chxchx-studio-ubuntu-x86_64.tar.gz").open("ab") as stream:
                stream.write(b"tampered")
        return Release(output, version, commit, channel)

    return build


@pytest.fixture
def sandbox(tmp_path: Path):
    """An isolated HOME, a PATH with a fake `uv` first, and a helper that runs the installer."""
    home = tmp_path / "home"
    home.mkdir()
    fakebin = tmp_path / "fakebin"
    _executable(fakebin / "uv", FAKE_UV)

    class Sandbox:
        def __init__(self) -> None:
            self.home, self.fakebin = home, fakebin
            self.base = ""

        def env(self, release: Release | None = None, **extra: str) -> dict[str, str]:
            env = {key: value for key, value in os.environ.items() if not key.startswith(("CHXCHX_", "XDG_"))}
            env.update(HOME=str(home), PATH=f"{fakebin}{os.pathsep}{os.environ['PATH']}", SHELL="/bin/bash", FAKE_PYTHON=sys.executable,
                       CHXCHX_LINUX_FLAVOR="ubuntu")
            if release is not None:
                env["CHXCHX_TEST_BASE"] = f"{self.base}/{release.name}"
            env.update(extra)
            return env

        def run(self, *args: str, release: Release | None = None, **extra: str) -> subprocess.CompletedProcess[str]:
            return subprocess.run(["bash", str(SCRIPTS / "install.sh"), *args], env=self.env(release, **extra),
                                  capture_output=True, text=True, timeout=120)

        def command(self, name: str, *args: str) -> subprocess.CompletedProcess[str]:
            return subprocess.run([str(home / ".local" / "bin" / name), *args], env=self.env(), capture_output=True, text=True, timeout=30)

        def tree(self) -> set[str]:
            return {str(path.relative_to(home)) for path in home.rglob("*")}

        def fake_tool(self, name: str, body: str) -> None:
            _executable(fakebin / name, f"#!/usr/bin/env bash\n{body}\n")

    box = Sandbox()
    return box


@pytest.fixture
def box(sandbox, server_root):
    sandbox.base = server_root[1]
    return sandbox
