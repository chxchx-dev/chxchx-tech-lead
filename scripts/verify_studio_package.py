#!/usr/bin/env python3
"""Check the contents and stage exactly one distributable Studio package."""

from __future__ import annotations

import argparse
import os
import shutil
import tarfile
import zipfile
from pathlib import Path


ASSETS = {
    "linux-ubuntu-x86_64": (".tar.gz", "chxchx-studio-ubuntu-x86_64.tar.gz"),
    "linux-fedora-x86_64": (".tar.gz", "chxchx-studio-fedora-x86_64.tar.gz"),
    "windows-x86_64": (".zip", "chxchx-studio-windows-x86_64.zip"),
    "macos-arm64": (".dmg", "chxchx-studio-macos-arm64.dmg"),
    "macos-x86_64": (".dmg", "chxchx-studio-macos-x86_64.dmg"),
}


def package_members(package: Path) -> set[str]:
    if package.suffix == ".zip":
        with zipfile.ZipFile(package) as archive:
            return {name.replace("\\", "/").casefold() for name in archive.namelist()}
    with tarfile.open(package, "r:gz") as archive:
        return {member.name.replace("\\", "/").casefold() for member in archive.getmembers()}


def require_suffix(members: set[str], *suffixes: str) -> None:
    missing = [suffix for suffix in suffixes if not any(name.endswith(suffix) for name in members)]
    if missing:
        raise ValueError("Faltan archivos requeridos: " + ", ".join(missing))


def verify_archive(platform: str, package: Path) -> None:
    members = package_members(package)
    require_suffix(members, "/bin/chxchx-studio.exe" if platform == "windows-x86_64" else "/bin/chxchx-studio")
    if platform == "windows-x86_64":
        require_suffix(
            members,
            "/qt6core.dll",
            "/qt6widgets.dll",
            "/qt6core5compat.dll",
            "/platforms/qwindows.dll",
        )
    elif platform.startswith("linux-"):
        require_suffix(
            members,
            "/qt.conf",
            "/libqt6core.so.6",
            "/libqt6widgets.so.6",
            "/libqt6core5compat.so.6",
            "/platforms/libqxcb.so",
        )


def verify_macos_bundle(bundle: Path) -> None:
    if bundle.suffix != ".app" or not bundle.is_dir():
        raise ValueError(f"No se encontró un bundle .app montado en: {bundle}")
    bundle_contents = bundle / "Contents"
    required = (
        bundle_contents / "MacOS/chxchx-studio",
        bundle_contents / "Frameworks/QtCore.framework",
        bundle_contents / "Frameworks/QtWidgets.framework",
        bundle_contents / "Frameworks/QtCore5Compat.framework",
        bundle_contents / "PlugIns/platforms/libqcocoa.dylib",
    )
    missing = [str(path.relative_to(bundle)) for path in required if not path.exists()]
    executable = bundle_contents / "MacOS/chxchx-studio"
    if executable.exists() and not os.access(executable, os.X_OK):
        missing.append("Contents/MacOS/chxchx-studio (no ejecutable)")
    if missing:
        raise ValueError("El bundle macOS está incompleto: " + ", ".join(missing))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", choices=sorted(ASSETS), required=True)
    parser.add_argument("--package-dir", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--mounted-app", type=Path)
    args = parser.parse_args()

    suffix, output_name = ASSETS[args.platform]
    packages = sorted(path for path in args.package_dir.iterdir() if path.is_file() and path.name.endswith(suffix))
    if len(packages) != 1:
        raise SystemExit(f"Se esperaba un único paquete {suffix}; encontrados: {len(packages)}")
    package = packages[0]

    if args.platform.startswith("macos-"):
        if args.mounted_app is None:
            raise SystemExit("macOS requiere --mounted-app para validar el runtime dentro del DMG.")
        verify_macos_bundle(args.mounted_app)
    else:
        verify_archive(args.platform, package)

    args.output_dir.mkdir(parents=True, exist_ok=True)
    staged = args.output_dir / output_name
    shutil.copy2(package, staged)
    print(f"Paquete validado y preparado para publicar: {staged}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
