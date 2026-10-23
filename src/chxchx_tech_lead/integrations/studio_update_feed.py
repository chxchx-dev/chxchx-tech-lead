"""Read and validate the public Studio release manifest."""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import os
from pathlib import Path
import platform
import re
import sys
import urllib.request


OWNER = "chxchx-dev"
REPOSITORY = "chxchx-tech-lead"
RELEASE_TAG = "studio-dev-preview"
RELEASE_BASE = f"https://github.com/{OWNER}/{REPOSITORY}/releases/download/{RELEASE_TAG}"
MANIFEST_SCHEMA = 1
PLATFORMS = {
    ("Linux", "x86_64", "ubuntu"): ("linux-ubuntu-x86_64", "chxchx-studio-ubuntu-x86_64.tar.gz"),
    ("Linux", "x86_64", "fedora"): ("linux-fedora-x86_64", "chxchx-studio-fedora-x86_64.tar.gz"),
    ("Darwin", "arm64", ""): ("macos-arm64", "chxchx-studio-macos-arm64.dmg"),
    ("Darwin", "x86_64", ""): ("macos-x86_64", "chxchx-studio-macos-x86_64.dmg"),
    ("Windows", "AMD64", ""): ("windows-x86_64", "chxchx-studio-windows-x86_64.zip"),
}


@dataclass(frozen=True)
class StudioRelease:
    commit: str
    version: str
    platform: str
    package: str
    package_sha256: str
    wheel: str
    wheel_sha256: str


def platform_asset(system: str | None = None, machine: str | None = None) -> tuple[str, str]:
    system = system or platform.system()
    machine = machine or platform.machine()
    architecture = machine.casefold()
    if architecture in {"amd64", "x64"}:
        architecture = "x86_64"
    if system == "Windows":
        architecture = "AMD64" if architecture == "x86_64" else machine
    os_id = ""
    if system == "Linux":
        os_id = _linux_id()
    try:
        platform_key = (system, architecture, os_id)
        if system == "Windows":
            platform_key = (system, architecture, "")
        return PLATFORMS[platform_key]
    except KeyError as exc:
        raise ValueError(f"No hay un paquete de Studio para {system}/{machine} ({os_id or 'sistema'}).") from exc


def _linux_id() -> str:
    try:
        for line in Path("/etc/os-release").read_text(encoding="utf-8").splitlines():
            if line.startswith("ID="):
                return line.partition("=")[2].strip().strip('"').casefold()
    except OSError:
        pass
    return ""


def parse_manifest(text: str, *, system: str | None = None, machine: str | None = None) -> StudioRelease:
    values: dict[str, str] = {}
    for line in text.splitlines():
        if not line or line.startswith("#"):
            continue
        key, separator, value = line.partition("=")
        if not separator or not re.fullmatch(r"[a-z0-9.-]+", key) or key in values:
            raise ValueError("El manifiesto de Studio contiene una línea inválida o una clave repetida.")
        values[key] = value

    required = ("schema_version", "commit", "version", "asset.cli-wheel", "sha256.cli-wheel")
    if any(not values.get(key) for key in required):
        raise ValueError("El manifiesto de Studio está incompleto.")
    if values["schema_version"] != str(MANIFEST_SCHEMA):
        raise ValueError(f"Versión de manifiesto no compatible: {values['schema_version']}.")
    if not re.fullmatch(r"[0-9a-f]{40}", values["commit"]):
        raise ValueError("El manifiesto de Studio contiene un commit inválido.")
    if not re.fullmatch(r"[0-9A-Za-z.+-]+", values["version"]):
        raise ValueError("El manifiesto de Studio contiene una versión inválida.")
    wheel = values["asset.cli-wheel"]
    if not re.fullmatch(r"chxchx_tech_lead-[A-Za-z0-9.+!_-]+-py3-none-any\.whl", wheel):
        raise ValueError("El manifiesto de Studio contiene un wheel inválido.")
    if not re.fullmatch(r"[0-9a-f]{64}", values["sha256.cli-wheel"]):
        raise ValueError("El manifiesto de Studio contiene un checksum inválido.")

    platform_key, expected_package = platform_asset(system, machine)
    package = values.get(f"asset.{platform_key}", "")
    package_sha = values.get(f"sha256.{platform_key}", "")
    if package != expected_package or not re.fullmatch(r"[0-9a-f]{64}", package_sha):
        raise ValueError(f"El manifiesto no contiene un paquete válido para {platform_key}.")
    return StudioRelease(
        commit=values["commit"],
        version=values["version"],
        platform=platform_key,
        package=package,
        package_sha256=package_sha,
        wheel=wheel,
        wheel_sha256=values["sha256.cli-wheel"],
    )


def fetch_release(*, timeout: float = 20.0) -> StudioRelease:
    request = urllib.request.Request(
        f"{RELEASE_BASE}/studio-preview.txt",
        headers={"Cache-Control": "no-cache", "User-Agent": "ChxChx-Studio-Updater"},
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        content = response.read(64 * 1024 + 1)
    if len(content) > 64 * 1024:
        raise ValueError("El manifiesto de Studio supera el tamaño permitido.")
    return parse_manifest(content.decode("utf-8"))


def download_asset(name: str, expected_sha256: str, destination: Path, *, timeout: float = 90.0) -> None:
    if "/" in name or "\\" in name or name in {"", ".", ".."}:
        raise ValueError("Nombre de asset inválido.")
    request = urllib.request.Request(
        f"{RELEASE_BASE}/{name}", headers={"User-Agent": "ChxChx-Studio-Updater"}
    )
    digest = hashlib.sha256()
    with urllib.request.urlopen(request, timeout=timeout) as response, destination.open("wb") as output:
        while chunk := response.read(1024 * 1024):
            digest.update(chunk)
            output.write(chunk)
    if digest.hexdigest() != expected_sha256:
        destination.unlink(missing_ok=True)
        raise ValueError(f"El SHA-256 de {name} no coincide; la descarga se descartó.")


def installed_app_root() -> Path:
    """Find a managed install from the running CLI path, including Finder launches."""
    candidates = [os.environ.get("CHXCHX_TECH_CLI"), sys.argv[0], sys.executable]
    for candidate in candidates:
        if not candidate:
            continue
        try:
            binary = Path(candidate).expanduser().resolve()
        except (OSError, RuntimeError):
            continue
        for ancestor in binary.parents:
            if ancestor.name != ".venv":
                continue
            root = ancestor.parent
            if root.name == "Contents":
                root = root.parent
            if (root / ".chxchx-managed").is_file() and _is_expected_install_root(root):
                return root
    raise ValueError("No se detectó una instalación administrada de Studio desde su instalador.")


def installed_cli_path() -> Path:
    root = installed_app_root()
    cli_root = root / "Contents" if root.suffix.casefold() == ".app" else root
    binary = cli_root / ".venv" / (
        "Scripts/chxchx-tech.exe" if platform.system() == "Windows" else "bin/chxchx-tech"
    )
    if not binary.is_file() or (platform.system() != "Windows" and not os.access(binary, os.X_OK)):
        raise ValueError("No se encontró el bridge CLI de la instalación actual de Studio.")
    return binary


def _is_expected_install_root(root: Path) -> bool:
    system = platform.system()
    if system == "Windows":
        local_app_data = os.environ.get("LOCALAPPDATA")
        if not local_app_data:
            return False
        base = Path(local_app_data) / "Programs" / "ChxChxStudio"
        return root.parent == base and re.fullmatch(r"dev-[0-9a-f]{12}", root.name) is not None
    if system == "Darwin":
        base = Path.home() / "Applications"
        return root.parent == base and re.fullmatch(
            r"ChxChx Studio Dev-[0-9a-f]{12}\.app", root.name, re.IGNORECASE
        ) is not None
    base = Path.home() / ".local" / "opt" / "chxchx-studio"
    return root.parent == base and re.fullmatch(r"dev-[0-9a-f]{12}", root.name) is not None


def installed_commit() -> str:
    root = installed_app_root()
    marker = root / ".chxchx-managed"
    try:
        commit = marker.read_text(encoding="utf-8").strip()
    except OSError as exc:
        raise ValueError("No se encontró el marcador de una instalación administrada de Studio.") from exc
    if re.fullmatch(r"[0-9a-f]{40}", commit):
        return commit
    match = re.search(
        r"(?:dev-|Studio Dev-)([0-9a-f]{12})(?:\.app)?$",
        root.name,
        re.IGNORECASE,
    )
    if not commit and match:
        return match.group(1).lower()
    raise ValueError("La instalación actual de Studio tiene un marcador inválido.")
