"""Point the launcher, the macOS link and the `chxchx-tech` shim at a freshly installed Studio."""

from __future__ import annotations

import os
from pathlib import Path

from .studio_channels import Channel

LAUNCHER_MARKS = ("CHXCHX_STUDIO_LAUNCHER", "CHXCHX_STUDIO_DEV_LAUNCHER")
SHIM_MARK = "CHXCHX_CLI_SHIM"


def switch_launcher(system: str, destination: Path, launcher: Path, channel: Channel, cli_root: Path) -> None:
    launcher.parent.mkdir(parents=True, exist_ok=True)
    _require_managed(launcher, LAUNCHER_MARKS, "launcher")

    link = Path.home() / "Applications" / f"{channel.label}.app" if system == "Darwin" else None
    previous_link: str | None = None
    if link is not None and (link.exists() or link.is_symlink()):
        if not link.is_symlink():
            raise ValueError(f"Ya existe una app ajena en {link}; no se reemplazó.")
        previous_link = os.readlink(link)
        old_target = Path(previous_link)
        if not old_target.is_absolute():
            old_target = link.parent / old_target
        old_target = old_target.resolve()
        if old_target.parent != destination.parent or not (old_target / ".chxchx-managed").is_file():
            raise ValueError(f"El enlace {link} apunta a una app ajena; no se reemplazó.")

    marks = f"CHXCHX_STUDIO_LAUNCHER channel={channel.name}" + ("\nCHXCHX_STUDIO_DEV_LAUNCHER" if channel.name == "dev" else "")
    temporary = launcher.with_name(launcher.name + f".tmp-{os.getpid()}")
    link_tmp = link.with_name(link.name + f".tmp-{os.getpid()}") if link is not None else None
    try:
        if system == "Windows":
            _write_windows(temporary, marks, destination)
        else:
            app_root = str(link) if link is not None else str(destination)
            _write_posix(temporary, marks, app_root, mac=link is not None)
        if link is not None and link_tmp is not None:
            link_tmp.unlink(missing_ok=True)
            link_tmp.symlink_to(destination)
            os.replace(link_tmp, link)
        os.replace(temporary, launcher)
        _rewrite_shim(system, launcher.parent, cli_root)
    except OSError:
        if link is not None and previous_link is not None and link_tmp is not None:
            link_tmp.unlink(missing_ok=True)
            link_tmp.symlink_to(previous_link)
            os.replace(link_tmp, link)
        elif link is not None and previous_link is None:
            link.unlink(missing_ok=True)
        raise
    finally:
        temporary.unlink(missing_ok=True)
        if link_tmp is not None:
            link_tmp.unlink(missing_ok=True)


def _require_managed(path: Path, marks: tuple[str, ...], what: str) -> None:
    if path.is_symlink():
        raise ValueError(f"El {what} {path} es un enlace ajeno; no se reemplazó.")
    if path.exists():
        if not path.is_file():
            raise ValueError(f"La ruta {path} no es un archivo de {what}; no se reemplazó.")
        content = path.read_text(encoding="utf-8", errors="replace")
        if not any(mark in content for mark in marks):
            raise ValueError(f"Ya existe un {what} ajeno en {path}; no se reemplazó.")


def _write_windows(path: Path, marks: str, destination: Path) -> None:
    mark_lines = "".join(f"rem {mark}\r\n" for mark in marks.splitlines())
    path.write_text(
        f"@echo off\r\n{mark_lines}"
        f'set "APP_ROOT={destination}"\r\n'
        'set "CHXCHX_TECH_CLI=%APP_ROOT%\\.venv\\Scripts\\chxchx-tech.exe"\r\n'
        '"%APP_ROOT%\\bin\\chxchx-studio.exe" %*\r\n',
        encoding="ascii",
    )


def _write_posix(path: Path, marks: str, app_root: str, *, mac: bool) -> None:
    mark_lines = "".join(f"# {mark}\n" for mark in marks.splitlines())
    contents = f'#!/usr/bin/env bash\n{mark_lines}APP_ROOT="{app_root}"\nexport PATH="$HOME/.local/bin:$PATH"\n'
    if mac:
        contents += 'export CHXCHX_TECH_CLI="$APP_ROOT/Contents/.venv/bin/chxchx-tech"\n'
        contents += 'exec "$APP_ROOT/Contents/MacOS/chxchx-studio" "$@"\n'
    else:
        contents += 'export CHXCHX_TECH_CLI="$APP_ROOT/.venv/bin/chxchx-tech"\n'
        contents += 'for qt_lib in "$APP_ROOT/lib" "$APP_ROOT/lib64"; do\n'
        contents += '  [ -d "$qt_lib" ] && LD_LIBRARY_PATH="$qt_lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"\n'
        contents += 'done\nexport LD_LIBRARY_PATH\n'
        contents += 'exec "$APP_ROOT/bin/chxchx-studio" "$@"\n'
    path.write_text(contents, encoding="utf-8")
    path.chmod(0o755)


def _rewrite_shim(system: str, directory: Path, cli_root: Path) -> None:
    """Keep `chxchx-tech` on the new version, but only if ChxChx wrote that shim."""
    windows = system == "Windows"
    shim = directory / ("chxchx-tech.cmd" if windows else "chxchx-tech")
    if shim.is_symlink() or not shim.is_file() or SHIM_MARK not in shim.read_text(encoding="utf-8", errors="replace"):
        return
    binary = cli_root / ".venv" / ("Scripts/chxchx-tech.exe" if windows else "bin/chxchx-tech")
    temporary = shim.with_name(shim.name + f".tmp-{os.getpid()}")
    try:
        if windows:
            temporary.write_text(f'@echo off\r\nrem {SHIM_MARK}\r\n"{binary}" %*\r\n', encoding="ascii")
        else:
            temporary.write_text(f'#!/usr/bin/env bash\n# {SHIM_MARK}\nexec "{binary}" "$@"\n', encoding="utf-8")
            temporary.chmod(0o755)
        os.replace(temporary, shim)
    finally:
        temporary.unlink(missing_ok=True)
