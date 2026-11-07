"""Find agent CLIs the way a terminal would, even when Studio was not started from one.

An app launched from the Dock, the application menu or a launcher gets a minimal PATH
(on macOS just `/usr/bin:/bin:/usr/sbin:/sbin`), so `opencode`, `codex`, `claude` and the
`node` they run on are not found although they work in the person's terminal. The usual
install folders are searched after PATH, so anything already on PATH keeps priority.
"""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path


def _node_version_folders(home: Path) -> list[Path]:
    """`~/.nvm/versions/node/*/bin`, newest first (nvm does not put any of them on a GUI app's PATH)."""
    base = home / ".nvm" / "versions" / "node"
    try:
        versions = sorted((item for item in base.iterdir() if item.is_dir()), key=lambda item: item.name, reverse=True)
    except OSError:
        return []
    return [item / "bin" for item in versions]


def extra_directories(home: Path | None = None) -> list[Path]:
    home = home or Path.home()
    folders = [
        home / ".opencode" / "bin", home / ".local" / "bin", home / ".claude" / "local", home / ".bun" / "bin",
        home / ".npm-global" / "bin", home / ".volta" / "bin", home / ".cargo" / "bin", home / ".asdf" / "shims",
        home / ".local" / "share" / "pnpm", home / ".local" / "share" / "fnm" / "aliases" / "default" / "bin",
        *_node_version_folders(home),
    ]
    if sys.platform == "darwin":
        folders += [Path("/opt/homebrew/bin"), Path("/opt/homebrew/sbin"), Path("/usr/local/bin")]
    elif os.name == "nt":
        for variable in ("APPDATA", "LOCALAPPDATA"):
            if os.environ.get(variable):
                folders += [Path(os.environ[variable]) / "npm"]
        folders += [home / "AppData" / "Roaming" / "npm"]
    else:
        folders += [Path("/usr/local/bin"), Path("/snap/bin"), Path("/home/linuxbrew/.linuxbrew/bin")]
    return folders


def search_path(base: str | None = None, home: Path | None = None) -> str:
    """PATH followed by the install folders that exist and are not already in it."""
    current = os.environ.get("PATH", "") if base is None else base
    present = [item for item in current.split(os.pathsep) if item]
    known = {os.path.normcase(item) for item in present}
    for folder in extra_directories(home):
        text = str(folder)
        if os.path.normcase(text) not in known and folder.is_dir():
            present.append(text)
            known.add(os.path.normcase(text))
    return os.pathsep.join(present)


def find_executable(name: str) -> str | None:
    """Full path of `name` (honouring PATHEXT on Windows), or None."""
    return shutil.which(name) or shutil.which(name, path=search_path())


def with_search_path(environment: dict[str, str] | None = None) -> dict[str, str]:
    """A copy of the environment whose PATH also reaches the install folders (so `#!/usr/bin/env node` works)."""
    result = dict(os.environ if environment is None else environment)
    key = next((name for name in result if name.upper() == "PATH"), "PATH")  # Windows spells it `Path`
    result[key] = search_path(result.get(key, ""))
    return result
