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


from .studio_channels import DEV, Channel, channel_of_root

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
    channel: str = "dev"


def platform_asset(system: str | None = None, machine: str | None = None) -> tuple[str, str]:
    system = system or platform.system()
    machine = machine or platform.machine()
    architecture = machine.casefold()
    if architecture in {"amd64", "x64"}:
        architecture = "x86_64"
    if system == "Windows":
        # ARM64 runs the x64 package under emulation until there is a native build.
        architecture = "AMD64" if architecture in {"x86_64", "arm64", "aarch64"} else machine
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
    """`ubuntu` or `fedora` for the package family; derivatives map through ID_LIKE."""
    try:
        text = Path("/etc/os-release").read_text(encoding="utf-8")
    except OSError:
        return ""
    fields = {key: value.strip().strip('"').casefold() for key, _, value in (line.partition("=") for line in text.splitlines())}
    words = f"{fields.get('ID', '')} {fields.get('ID_LIKE', '')}".split()
    for family, names in (("ubuntu", {"ubuntu", "debian"}), ("fedora", {"fedora", "rhel", "centos"})):
        if names & set(words):
            return family
    return ""


def parse_manifest(
    text: str, *, system: str | None = None, machine: str | None = None, channel: str = "dev"
) -> StudioRelease:
    values: dict[str, str] = {}
    for line in text.splitlines():
        if not line or line.startswith("#"):
            continue
        key, separator, value = line.partition("=")
        if not separator or not re.fullmatch(r"[a-z0-9_.-]+", key) or key in values:
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
    if values.get("channel", channel) != channel:
        raise ValueError(f"El manifiesto es del canal {values['channel']}, no de {channel}.")
    if channel == "stable" and not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", values["version"]):
        raise ValueError("La versión estable debe tener la forma X.Y.Z.")
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
        channel=channel,
    )


def fetch_release(channel: Channel = DEV, *, timeout: float = 20.0) -> StudioRelease:
    last_error: Exception | None = None
    for name in channel.manifests:
        request = urllib.request.Request(
            f"{channel.base}/{name}",
            headers={"Cache-Control": "no-cache", "User-Agent": "ChxChx-Studio-Updater"},
        )
        try:
            with urllib.request.urlopen(request, timeout=timeout) as response:
                content = response.read(64 * 1024 + 1)
        except OSError as exc:  # el preview anterior solo publicaba studio-preview.txt
            last_error = exc
            continue
        if len(content) > 64 * 1024:
            raise ValueError("El manifiesto de Studio supera el tamaño permitido.")
        return parse_manifest(content.decode("utf-8"), channel=channel.name)
    raise ValueError(f"No se pudo leer el manifiesto del canal {channel.name}: {last_error}")


def download_asset(
    name: str, expected_sha256: str, destination: Path, *, base: str | None = None, timeout: float = 90.0
) -> None:
    if "/" in name or "\\" in name or name in {"", ".", ".."}:
        raise ValueError("Nombre de asset inválido.")
    request = urllib.request.Request(
        f"{base or DEV.base}/{name}", headers={"User-Agent": "ChxChx-Studio-Updater"}
    )
    digest = hashlib.sha256()
    with urllib.request.urlopen(request, timeout=timeout) as response, destination.open("wb") as output:
        while chunk := response.read(1024 * 1024):
            digest.update(chunk)
            output.write(chunk)
    if digest.hexdigest() != expected_sha256:
        destination.unlink(missing_ok=True)
        raise ValueError(f"El SHA-256 de {name} no coincide; la descarga se descartó.")


def installed_channel() -> Channel:
    channel = channel_of_root(installed_app_root())
    if channel is None:
        raise ValueError("No se reconoció el canal de la instalación de Studio.")
    return channel


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
    channel = channel_of_root(root)
    if channel is None:
        return False
    system = platform.system()
    if system == "Windows":
        local_app_data = os.environ.get("LOCALAPPDATA")
        base = Path(local_app_data) / "Programs" / "ChxChxStudio" if local_app_data else None
        return root.parent == base and re.fullmatch(channel.root_pattern, root.name) is not None
    if system == "Darwin":
        return root.parent == Path.home() / "Applications" and re.fullmatch(channel.app_pattern, root.name, re.IGNORECASE) is not None
    base = Path.home() / ".local" / "opt" / "chxchx-studio"
    return root.parent == base and re.fullmatch(channel.root_pattern, root.name) is not None


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
