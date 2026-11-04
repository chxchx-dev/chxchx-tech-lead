"""Install a validated Studio release beside the running version."""

from __future__ import annotations

import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile
import zipfile

from .studio_update_feed import (
    StudioRelease,
    download_asset,
    fetch_release,
    installed_app_root,
    installed_channel,
    installed_cli_path,
    installed_commit,
)
from .studio_channels import CHANNELS, root_name
from .studio_update_launcher import switch_launcher


def update_status() -> dict[str, object]:
    current = installed_commit()
    latest = fetch_release(installed_channel())
    return _payload(current, latest)


def apply_update() -> dict[str, object]:
    current = installed_commit()
    release = fetch_release(installed_channel())
    result = _payload(current, release)
    if release.commit.startswith(current):
        return result
    base = CHANNELS[release.channel].base
    try:
        with tempfile.TemporaryDirectory(prefix="chxchx-studio-update-") as temporary:
            work = Path(temporary)
            package = work / release.package
            wheel = work / release.wheel
            download_asset(release.package, release.package_sha256, package, base=base)
            download_asset(release.wheel, release.wheel_sha256, wheel, base=base)
            _install_package(release, package, wheel)
    except Exception as exc:
        if isinstance(exc, ValueError):
            raise
        raise ValueError(
            "No se pudo completar la actualización. La versión activa se conservó; "
            f"detalle: {exc}"
        ) from exc
    result["updated"] = True
    return result


def _payload(current: str, release: StudioRelease) -> dict[str, object]:
    return {
        "schema": "chxchx.studio-update",
        "schema_version": 1,
        "current_commit": current,
        "current_version": _installed_version(),
        "latest_commit": release.commit,
        "latest_version": release.version,
        "platform": release.platform,
        "available": not release.commit.startswith(current),
        "updated": False,
    }


def _installed_version() -> str:
    try:
        from importlib.metadata import version

        return version("chxchx-tech-lead")
    except Exception:
        return "desconocida"


def _install_package(release: StudioRelease, package: Path, wheel: Path) -> None:
    system = platform.system()
    current_root = installed_app_root()
    current_cli = installed_cli_path()
    if not current_cli.is_file() or not (current_root / ".chxchx-managed").is_file():
        raise ValueError("La instalación actual de Studio ya no está completa o administrada.")

    channel = CHANNELS[release.channel]
    name = root_name(channel, release.version, release.commit, mac=system == "Darwin")
    if system == "Windows":
        parent = Path(os.environ["LOCALAPPDATA"]) / "Programs" / "ChxChxStudio"
        launcher = parent / "shims" / f"{channel.launcher}.cmd"
    elif system == "Darwin":
        parent = Path.home() / "Applications"
        launcher = Path.home() / ".local" / "bin" / channel.launcher
    else:
        parent = Path.home() / ".local" / "opt" / "chxchx-studio"
        launcher = Path.home() / ".local" / "bin" / channel.launcher
    destination = parent / name
    parent.mkdir(parents=True, exist_ok=True)
    mac_bundle = system == "Darwin"
    cli_root = destination / "Contents" if mac_bundle else destination
    marker = destination / ".chxchx-managed"

    if destination.is_symlink():
        raise ValueError(f"La ruta {destination} es un enlace ajeno; no se reemplazó.")
    if destination.exists():
        if not marker.is_file() or marker.read_text(encoding="utf-8").strip() != release.commit:
            raise ValueError(f"La ruta {destination} ya existe y no pertenece a esta versión de Studio.")
        _require_executable(destination, mac_bundle)
    else:
        staging = Path(tempfile.mkdtemp(prefix=".chxchx-update-stage-", dir=parent))
        try:
            if mac_bundle:
                _extract_dmg(package, staging, destination.name)
            elif system == "Windows":
                _extract_zip(package, staging)
                _flatten_archive_root(staging, "chxchx-studio.exe")
            else:
                with tarfile.open(package, "r:gz") as archive:
                    archive.extractall(staging, filter="data")
                _flatten_archive_root(staging, "chxchx-studio")
            staged_root = staging / destination.name if mac_bundle else staging
            _require_executable(staged_root, mac_bundle)
            (staged_root / ".chxchx-managed").write_text(release.commit + "\n", encoding="utf-8")
            staged_root.replace(destination)
        finally:
            if staging.exists():
                shutil.rmtree(staging, ignore_errors=True)

    _install_cli(cli_root, wheel)
    switch_launcher(system, destination, launcher, channel, cli_root)


def _extract_zip(package: Path, destination: Path) -> None:
    with zipfile.ZipFile(package) as archive:
        for item in archive.infolist():
            candidate = (destination / item.filename).resolve()
            if not candidate.is_relative_to(destination.resolve()):
                raise ValueError("El ZIP de Studio contiene una ruta fuera del directorio de instalación.")
            if item.external_attr >> 16 & 0o170000 == 0o120000:
                raise ValueError("El ZIP de Studio contiene un enlace simbólico no permitido.")
        archive.extractall(destination)


def _flatten_archive_root(staging: Path, executable: str) -> None:
    if (staging / "bin" / executable).is_file():
        return
    candidates = [directory for directory in staging.iterdir() if directory.is_dir() and (directory / "bin" / executable).is_file()]
    if len(candidates) != 1:
        raise ValueError(f"El paquete no contiene bin/{executable} en una raíz reconocible.")
    root = candidates[0]
    for child in root.iterdir():
        child.replace(staging / child.name)
    root.rmdir()


def _extract_dmg(package: Path, staging: Path, app_name: str) -> None:
    mount = staging / "mount"
    mount.mkdir()
    subprocess.run(
        ["hdiutil", "attach", str(package), "-readonly", "-nobrowse", "-mountpoint", str(mount)],
        check=True,
        stdout=subprocess.DEVNULL,
    )
    try:
        bundles = list(mount.glob("*.app"))
        if len(bundles) != 1:
            raise ValueError("El DMG de Studio no contiene una única aplicación .app.")
        subprocess.run(["ditto", str(bundles[0]), str(staging / app_name)], check=True)
    finally:
        subprocess.run(["hdiutil", "detach", str(mount)], check=False, stdout=subprocess.DEVNULL)
        shutil.rmtree(mount, ignore_errors=True)


def _require_executable(root: Path, mac_bundle: bool) -> None:
    executable = root / "Contents" / "MacOS" / "chxchx-studio" if mac_bundle else root / "bin" / (
        "chxchx-studio.exe" if platform.system() == "Windows" else "chxchx-studio"
    )
    if not executable.is_file():
        raise ValueError(f"El paquete de Studio no contiene el ejecutable esperado: {executable}.")
    if platform.system() != "Windows" and not os.access(executable, os.X_OK):
        raise ValueError(f"El ejecutable de Studio no tiene permiso de ejecución: {executable}.")


def _install_cli(cli_root: Path, wheel: Path) -> None:
    uv = shutil.which("uv")
    if not uv:
        candidate = Path.home() / ".local" / "bin" / ("uv.exe" if platform.system() == "Windows" else "uv")
        if candidate.is_file():
            uv = str(candidate)
    if not uv:
        raise ValueError("No se encontró uv para preparar el CLI nuevo; la instalación activa sigue intacta.")
    environment = cli_root / ".venv"
    python = environment / ("Scripts/python.exe" if platform.system() == "Windows" else "bin/python")
    try:
        subprocess.run([uv, "venv", "--clear", "--python", "3.11", str(environment)], check=True)
        subprocess.run(
            [uv, "pip", "install", "--python", str(python), "--reinstall-package", "chxchx-tech-lead", str(wheel)],
            check=True,
        )
    except subprocess.CalledProcessError as exc:
        raise ValueError(f"Falló la preparación del bridge CLI nuevo (código {exc.returncode}).") from exc
    command = environment / ("Scripts/chxchx-tech.exe" if platform.system() == "Windows" else "bin/chxchx-tech")
    if not command.is_file() or (platform.system() != "Windows" and not os.access(command, os.X_OK)):
        raise ValueError("El wheel nuevo no dejó un comando chxchx-tech ejecutable.")
    try:
        subprocess.run(
            [str(command), "--help"],
            check=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            timeout=20,
        )
    except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as exc:
        raise ValueError("El bridge CLI nuevo no pudo iniciarse; la versión activa sigue intacta.") from exc
