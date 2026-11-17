"""Apply a verified per-user MSI for an installation owned by Windows Installer."""

from __future__ import annotations

import os
from pathlib import Path
import platform
import shutil
import subprocess

from .studio_update_feed import StudioRelease, download_asset


def install_windows_msi(release: StudioRelease, work: Path, *, base: str) -> bool:
    """Install the release MSI without elevation; return whether Windows needs a reboot."""
    if platform.system() != "Windows":
        raise ValueError("El paquete MSI solo se puede aplicar en Windows.")
    if not release.windows_msi or not release.windows_msi_sha256:
        raise ValueError("El manifiesto de este release no incluye el MSI necesario para actualizar esta instalación.")

    package = work / release.windows_msi
    download_asset(release.windows_msi, release.windows_msi_sha256, package, base=base)
    msiexec = shutil.which("msiexec.exe") or str(Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32" / "msiexec.exe")
    if not Path(msiexec).is_file():
        raise ValueError("No se encontró Windows Installer (msiexec.exe).")
    try:
        result = subprocess.run(
            [msiexec, "/i", str(package), "/qn", "/norestart"],
            check=False,
            capture_output=True,
            timeout=20 * 60,
            creationflags=0x08000000,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise ValueError(f"No se pudo iniciar el MSI verificado: {exc}") from exc
    if result.returncode not in {0, 3010, 1641}:
        raise ValueError(f"Windows Installer no pudo actualizar ChxChx (código {result.returncode}).")
    return result.returncode in {3010, 1641}
