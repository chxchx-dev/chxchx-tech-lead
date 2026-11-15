from __future__ import annotations

from pathlib import Path

from chxchx_tech_lead.integrations.studio_update_launcher import SHIM_MARK, _rewrite_shim


def _managed(path: Path, body: str = "exec old\n") -> None:
    path.write_text(f"#!/usr/bin/env bash\n# {SHIM_MARK}\n{body}", encoding="utf-8")
    path.chmod(0o755)


def _package(root: Path, *, rust: bool) -> Path:
    (root / ".venv" / "bin").mkdir(parents=True)
    (root / ".venv" / "bin" / "chxchx-tech").write_text("#!/bin/sh\n", encoding="utf-8")
    if rust:
        (root / "bin").mkdir()
        (root / "bin" / "chxchx-tech").write_text("#!/bin/sh\n", encoding="utf-8")
    return root


def test_a_package_with_the_rust_command_makes_it_the_command_and_creates_the_short_name(tmp_path: Path) -> None:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    _managed(bin_dir / "chxchx-tech")
    root = _package(tmp_path / "app", rust=True)

    _rewrite_shim("Linux", bin_dir, root)

    for name in ("chxchx-tech", "chxchx"):
        text = (bin_dir / name).read_text(encoding="utf-8")
        assert f'exec "{root}/bin/chxchx-tech"' in text, name
        assert f'CHXCHX_TECH_CLI="{root}/.venv/bin/chxchx-tech"' in text, name
        assert SHIM_MARK in text


def test_a_package_without_it_keeps_the_python_command(tmp_path: Path) -> None:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    _managed(bin_dir / "chxchx-tech")
    root = _package(tmp_path / "app", rust=False)

    _rewrite_shim("Linux", bin_dir, root)

    assert (bin_dir / "chxchx-tech").read_text(encoding="utf-8").endswith(f'exec "{root}/.venv/bin/chxchx-tech" "$@"\n')


def test_foreign_files_are_never_replaced(tmp_path: Path) -> None:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    _managed(bin_dir / "chxchx-tech")
    (bin_dir / "chxchx").write_text("#!/bin/sh\necho mine\n", encoding="utf-8")
    root = _package(tmp_path / "app", rust=True)

    _rewrite_shim("Linux", bin_dir, root)

    assert (bin_dir / "chxchx").read_text(encoding="utf-8") == "#!/bin/sh\necho mine\n"
    # And without our own main shim nothing is touched at all.
    other = tmp_path / "other"
    other.mkdir()
    (other / "chxchx-tech").write_text("#!/bin/sh\necho uv\n", encoding="utf-8")
    _rewrite_shim("Linux", other, root)
    assert not (other / "chxchx").exists()


def test_the_macos_package_keeps_its_binary_inside_the_bundle(tmp_path: Path) -> None:
    bin_dir = tmp_path / "bin"
    bin_dir.mkdir()
    _managed(bin_dir / "chxchx-tech")
    contents = tmp_path / "ChxChx Studio-0.6.0.app" / "Contents"
    (contents / ".venv" / "bin").mkdir(parents=True)
    (contents / ".venv" / "bin" / "chxchx-tech").write_text("#!/bin/sh\n", encoding="utf-8")
    (contents / "MacOS").mkdir()
    (contents / "MacOS" / "chxchx-tech").write_text("#!/bin/sh\n", encoding="utf-8")

    _rewrite_shim("Darwin", bin_dir, contents)

    assert f'exec "{contents}/MacOS/chxchx-tech"' in (bin_dir / "chxchx-tech").read_text(encoding="utf-8")
