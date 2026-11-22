#!/usr/bin/env python3
"""Build an offline bundle of pinned Basic Memory and Serena tools for a Studio release."""

from __future__ import annotations

import argparse
import hashlib
import os
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

PYTHON = {
    "basic-memory": "3.12",
    "serena-agent": "3.13",
}
VERSIONS = {
    "basic-memory": "1.0.0",
    "serena-agent": "2.1.0",
}
PLATFORMS = {
    "linux-x86_64": (".tar.gz", "gztar"),
    "windows-x86_64": (".zip", "zip"),
    "macos-arm64": (".tar.gz", "gztar"),
    "macos-x86_64": (".tar.gz", "gztar"),
}


def run(arguments: list[str], *, env: dict[str, str]) -> str:
    try:
        result = subprocess.run(arguments, check=True, text=True, capture_output=True, env=env)
    except subprocess.CalledProcessError as error:
        detail = (error.stderr or error.stdout or "").strip()
        raise RuntimeError(f"{arguments[1]} terminó con {error.returncode}: {detail}") from error
    return result.stdout.strip()


def content_id(directory: Path, platform: str, release_id: str) -> str:
    digest = hashlib.sha256(platform.encode("utf-8"))
    for path in sorted(item for item in directory.rglob("*") if item.is_file()):
        relative = path.relative_to(directory).as_posix().encode("utf-8")
        digest.update(relative + b"\0" + str(stat.S_IMODE(path.stat().st_mode)).encode() + b"\0")
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
    return f"{release_id}-{digest.hexdigest()[:16]}"


def validate_links(directory: Path) -> None:
    root = directory.resolve(strict=True)
    for path in directory.rglob("*"):
        if path.is_symlink() and not path.resolve(strict=True).is_relative_to(root):
            raise ValueError(f"El runtime contiene un enlace fuera del bundle: {path}")


def add_wheel_hashes(lock: Path, wheelhouse: Path) -> None:
    """Allow locally built wheels only after adding their content hash to the lock."""
    lines = lock.read_text(encoding="utf-8").splitlines()
    starts = [
        index
        for index, line in enumerate(lines)
        if line and not line[0].isspace() and not line.startswith("#")
    ]
    blocks = [
        (start, starts[position + 1] if position + 1 < len(starts) else len(lines))
        for position, start in enumerate(starts)
    ]
    additions: list[tuple[int, str]] = []
    for wheel in sorted(wheelhouse.glob("*.whl")):
        parts = wheel.name[:-4].split("-")
        if len(parts) < 5:
            raise ValueError(f"Nombre de wheel no válido: {wheel.name}")
        package = re.sub(r"[-_.]+", "-", parts[0]).lower()
        version = parts[1]
        matching: list[tuple[int, int]] = []
        for start, end in blocks:
            match = re.match(r"([^=]+)==([^ ;\\]+)", lines[start])
            if match and re.sub(r"[-_.]+", "-", match.group(1)).lower() == package and match.group(2) == version:
                matching.append((start, end))
        if not matching:
            raise ValueError(f"El wheel {wheel.name} no corresponde a ningún pin del lock {lock}.")
        digest = hashlib.sha256(wheel.read_bytes()).hexdigest()
        hash_value = f"--hash=sha256:{digest}"
        for start, end in matching:
            if any(hash_value in line for line in lines[start:end]):
                continue
            insertion = next(
                (index for index in range(start + 1, end) if lines[index].lstrip().startswith("# via")),
                end,
            )
            previous = insertion - 1
            while previous > start and not lines[previous].strip():
                previous -= 1
            if not lines[previous].rstrip().endswith("\\"):
                lines[previous] = lines[previous].rstrip() + " " + chr(92)
            additions.append((insertion, f"    {hash_value}"))
    for index, line in reversed(additions):
        lines.insert(index, line)
    lock.write_text("\n".join(lines) + "\n", encoding="utf-8")


def build(platform: str, bundle_id: str, output: Path, archive: Path) -> None:
    if platform not in PLATFORMS:
        raise ValueError(f"Plataforma de bundle desconocida: {platform}")
    if not bundle_id or any(character not in "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-" for character in bundle_id):
        raise ValueError("bundle-id solo admite letras, números, punto, guion y guion bajo.")
    output = output.resolve()
    archive = archive.resolve()
    if output.exists() or archive.exists():
        raise ValueError(f"No se sobrescriben artefactos existentes: {output} / {archive}")

    uv = shutil.which("uv")
    if not uv:
        raise ValueError("Se requiere uv en PATH para construir el bundle.")
    output.mkdir(parents=True)
    python_root = output / "python"
    python_root.mkdir()
    shutil.copy2(uv, output / ("uv.exe" if platform.startswith("windows-") else "uv"))

    env = os.environ.copy()
    env["UV_PYTHON_INSTALL_DIR"] = str(python_root)
    env["UV_PYTHON_DOWNLOADS"] = "manual"
    run([uv, "python", "install", "3.12", "3.13", "--install-dir", str(python_root)], env=env)
    interpreters: dict[str, str] = {}
    for version in ("3.12", "3.13"):
        executable = Path(run([uv, "python", "find", version, "--managed-python", "--no-python-downloads", "--resolve-links"], env=env))
        try:
            interpreters[version] = executable.relative_to(output).as_posix()
        except ValueError as error:
            raise ValueError(f"uv encontró Python fuera del bundle: {executable}") from error
        if not executable.is_file():
            raise ValueError(f"No se encontró el intérprete empaquetado: {executable}")

    lock_paths: dict[str, str] = {}
    wheel_paths: dict[str, str] = {}
    requirements_dir = Path(__file__).resolve().parent / "required-tools"
    for package, version in VERSIONS.items():
        requirement = requirements_dir / f"{package}.in"
        expected_requirement = f"{package}=={version}"
        if expected_requirement not in requirement.read_text(encoding="utf-8").splitlines():
            raise ValueError(f"{requirement} debe fijar exactamente {expected_requirement}.")
        py_version = PYTHON[package]
        stem = package.replace("-", "_")
        lock = output / "locks" / f"{stem}.txt"
        wheelhouse = output / "wheels" / stem
        lock.parent.mkdir(parents=True, exist_ok=True)
        wheelhouse.mkdir(parents=True, exist_ok=True)
        compile_args = [
            uv,
            "pip",
            "compile",
            str(requirement),
            "--universal",
            "--python-version",
            py_version,
            "--generate-hashes",
            "--output-file",
            str(lock),
        ]
        run(compile_args, env=env)
        interpreter = output / interpreters[py_version]
        run(
            [
                uv,
                "run",
                "--no-project",
                "--python",
                str(interpreter),
                "--with",
                "pip",
                "--",
                "python",
                "-m",
                "pip",
                "wheel",
                "--require-hashes",
                "--no-deps",
                "--wheel-dir",
                str(wheelhouse),
                "--requirement",
                str(lock),
            ],
            env=env,
        )
        if not any(wheelhouse.glob("*.whl")):
            raise ValueError(f"El bundle de {package} no contiene wheels.")
        add_wheel_hashes(lock, wheelhouse)
        lock_paths[package] = lock.relative_to(output).as_posix()
        wheel_paths[package] = wheelhouse.relative_to(output).as_posix()

    validate_links(output)
    uv_name = "uv.exe" if platform.startswith("windows-") else "uv"
    final_bundle_id = content_id(output, platform, bundle_id)
    manifest = [
        "schema_version=1",
        f"bundle_id={final_bundle_id}",
        f"platform={platform}",
        f"uv={uv_name}",
        f"python.basic-memory={interpreters['3.12']}",
        f"python.serena-agent={interpreters['3.13']}",
    ]
    for package in VERSIONS:
        manifest.extend(
            (
                f"version.{package}={VERSIONS[package]}",
                f"lock.{package}={lock_paths[package]}",
                f"wheelhouse.{package}={wheel_paths[package]}",
            )
        )
    (output / "bundle-manifest.txt").write_text("\n".join(manifest) + "\n", encoding="utf-8")

    archive.parent.mkdir(parents=True, exist_ok=True)
    _, archive_format = PLATFORMS[platform]
    if archive_format == "zip":
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as bundle:
            for path in sorted(output.rglob("*")):
                if path.is_file():
                    bundle.write(path, path.relative_to(output).as_posix())
    else:
        with tarfile.open(archive, "w:gz") as bundle:
            for path in sorted(output.rglob("*")):
                if path.is_file():
                    bundle.add(path, arcname=path.relative_to(output).as_posix(), recursive=False)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--platform", required=True, choices=sorted(PLATFORMS))
    parser.add_argument("--bundle-id", required=True)
    parser.add_argument("--output", type=Path, required=True, help="Directory embedded or extracted by the installer.")
    parser.add_argument("--archive", type=Path, required=True, help="Release archive verified by the signed manifest.")
    arguments = parser.parse_args(argv)
    try:
        build(arguments.platform, arguments.bundle_id, arguments.output, arguments.archive)
    except (OSError, RuntimeError, ValueError) as error:
        print(f"No pude construir el bundle de herramientas: {error}", file=sys.stderr)
        return 1
    print(f"Bundle {arguments.platform} creado: {arguments.archive}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
