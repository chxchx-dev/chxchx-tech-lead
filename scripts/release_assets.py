#!/usr/bin/env python3
"""Stage the files of a Studio release and write its manifest and checksums.

Used by CI for every channel:

* ``stable``: a ``vX.Y.Z`` tag. The tag must match the version in ``pyproject.toml``.
* ``dev``:    the rolling ``studio-dev-preview`` from the ``dev`` branch.
* ``ci``:     a throw-away staging used by the installer smoke tests.

The manifest (``studio-manifest.txt``) is what the installers trust: it lists each
package with its SHA-256, the commit, the version and the channel. The dev channel
also keeps publishing ``studio-preview.txt``, the name older installers read.
"""

from __future__ import annotations

import argparse
import hashlib
import re
import shutil
import sys
import tomllib
from pathlib import Path

MANIFEST_SCHEMA = 1
MANIFEST_NAME = "studio-manifest.txt"
LEGACY_MANIFEST_NAME = "studio-preview.txt"
DEV_TAG = "studio-dev-preview"

# platform key -> (directory of the downloaded artifact, glob inside it, published name)
PACKAGES = {
    "linux-ubuntu-x86_64": ("linux-ubuntu-x86_64", "*.tar.gz", "chxchx-studio-ubuntu-x86_64.tar.gz"),
    "linux-fedora-x86_64": ("linux-fedora-x86_64", "*.tar.gz", "chxchx-studio-fedora-x86_64.tar.gz"),
    "windows-x86_64": ("windows-x86_64", "*.zip", "chxchx-studio-windows-x86_64.zip"),
    "windows-setup": ("windows-x86_64", "*-setup.exe", "chxchx-studio-windows-x86_64-setup.exe"),
    "windows-arm64": ("windows-arm64", "*.zip", "chxchx-studio-windows-arm64.zip"),
    "macos-arm64": ("macos-arm64", "*.dmg", "chxchx-studio-macos-arm64.dmg"),
    "macos-x86_64": ("macos-x86_64", "*.dmg", "chxchx-studio-macos-x86_64.dmg"),
}
# Included only when its artifact was downloaded: the native ARM64 build is not required for a release.
OPTIONAL_PACKAGES = {"windows-arm64"}
INSTALLERS = {"install-sh": "install.sh", "install-ps1": "install.ps1"}
STABLE_TAG = re.compile(r"v(\d+)\.(\d+)\.(\d+)")
STABLE_VERSION = re.compile(r"\d+\.\d+\.\d+")
WHEEL_PATTERN = "chxchx_tech_lead-*-py3-none-any.whl"


class ReleaseError(ValueError):
    """Raised when the inputs cannot make a trustworthy release."""


def project_version(root: Path) -> str:
    data = tomllib.loads((root / "pyproject.toml").read_text(encoding="utf-8"))
    return str(data["project"]["version"])


def check_tag(tag: str, version: str) -> None:
    """A stable tag must be `vX.Y.Z` and name exactly the version the code declares."""
    if not STABLE_TAG.fullmatch(tag):
        raise ReleaseError(f"El tag `{tag}` no tiene la forma vX.Y.Z.")
    if not STABLE_VERSION.fullmatch(version):
        raise ReleaseError(
            f"La versión de pyproject.toml es `{version}`: una versión estable debe ser X.Y.Z, sin sufijos como .dev0."
        )
    if tag[1:] != version:
        raise ReleaseError(f"El tag `{tag}` no coincide con la versión `{version}` de pyproject.toml.")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def _single(directory: Path, pattern: str, what: str) -> Path:
    matches = sorted(path for path in directory.rglob(pattern) if path.is_file())
    if len(matches) != 1:
        raise ReleaseError(f"Se esperaba un único {what} ({pattern}) en {directory}; hay {len(matches)}.")
    return matches[0]


def stage(
    *,
    root: Path,
    downloads: Path,
    output: Path,
    channel: str,
    tag: str,
    commit: str,
    require: set[str] | None = None,
) -> Path:
    """Copy the assets into `output` and write the manifest. Returns the manifest path."""
    if channel not in {"stable", "dev", "ci"}:
        raise ReleaseError(f"Canal desconocido: {channel}")
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ReleaseError("El commit debe ser un SHA completo de 40 caracteres.")
    version = project_version(root)
    if channel == "stable":
        check_tag(tag, version)
    elif channel == "dev" and tag != DEV_TAG:
        raise ReleaseError(f"El canal dev usa el tag {DEV_TAG}.")

    output.mkdir(parents=True, exist_ok=True)
    files: dict[str, str] = {}
    wanted = require if require is not None else set(PACKAGES)
    for key, (directory, pattern, name) in PACKAGES.items():
        folder = downloads / directory
        if (key in OPTIONAL_PACKAGES or key not in wanted) and not folder.exists():
            continue
        source = _single(folder, pattern, f"paquete {key}")
        shutil.copy2(source, output / name)
        files[key] = name
    wheel = _single(downloads / "cli-wheel", WHEEL_PATTERN, "wheel del CLI")
    shutil.copy2(wheel, output / wheel.name)
    files["cli-wheel"] = wheel.name
    for key, name in INSTALLERS.items():
        shutil.copy2(root / "scripts" / name, output / name)
        files[key] = name

    lines = [
        f"schema_version={MANIFEST_SCHEMA}",
        f"channel={channel}",
        f"tag={tag}",
        f"commit={commit}",
        f"version={version}",
    ]
    checksums = []
    for key, name in files.items():
        digest = sha256(output / name)
        lines.extend((f"asset.{key}={name}", f"sha256.{key}={digest}"))
        checksums.append(f"{digest}  {name}")
    manifest = output / MANIFEST_NAME
    manifest.write_text("\n".join(lines) + "\n", encoding="utf-8")
    if channel == "dev":
        shutil.copy2(manifest, output / LEGACY_MANIFEST_NAME)
    (output / "SHA256SUMS").write_text("\n".join(sorted(checksums, key=lambda row: row.split()[1])) + "\n", encoding="utf-8")
    return manifest


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--channel", choices=("stable", "dev", "ci"), required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--check-version", action="store_true", help="Only check that the tag matches pyproject.toml.")
    parser.add_argument("--commit")
    parser.add_argument("--downloads", type=Path)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    try:
        if args.check_version:
            check_tag(args.tag, project_version(args.root))
            print(f"OK: {args.tag} coincide con pyproject.toml.")
            return 0
        if not (args.commit and args.downloads and args.output):
            parser.error("--commit, --downloads y --output son obligatorios.")
        manifest = stage(root=args.root, downloads=args.downloads, output=args.output,
                         channel=args.channel, tag=args.tag, commit=args.commit)
    except ReleaseError as exc:
        print(f"Error: {exc}", file=sys.stderr)
        return 1
    print(f"Manifiesto escrito: {manifest}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
