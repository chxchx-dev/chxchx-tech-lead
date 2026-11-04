from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

SCRIPT = Path(__file__).resolve().parents[2] / "scripts" / "release_assets.py"
spec = importlib.util.spec_from_file_location("release_assets", SCRIPT)
release_assets = importlib.util.module_from_spec(spec)
sys.modules["release_assets"] = release_assets
spec.loader.exec_module(release_assets)

COMMIT = "a" * 40


def make_root(tmp_path: Path, version: str = "0.5.0") -> Path:
    root = tmp_path / "repo"
    (root / "scripts").mkdir(parents=True)
    (root / "pyproject.toml").write_text(f'[project]\nname = "chxchx-tech-lead"\nversion = "{version}"\n', encoding="utf-8")
    (root / "scripts" / "install.sh").write_text("#!/usr/bin/env bash\n", encoding="utf-8")
    (root / "scripts" / "install.ps1").write_text("# installer\n", encoding="utf-8")
    return root


def make_downloads(tmp_path: Path, *, skip: str | None = None, duplicate: str | None = None) -> Path:
    downloads = tmp_path / "downloads"
    for key, (directory, pattern, _name) in release_assets.PACKAGES.items():
        if key == skip:
            continue
        folder = downloads / directory
        folder.mkdir(parents=True, exist_ok=True)
        (folder / pattern.replace("*", f"pkg-{key}")).write_bytes(key.encode())
        if key == duplicate:
            (folder / pattern.replace("*", f"other-{key}")).write_bytes(b"x")
    wheel_dir = downloads / "cli-wheel"
    wheel_dir.mkdir(parents=True)
    (wheel_dir / "chxchx_tech_lead-0.5.0-py3-none-any.whl").write_bytes(b"wheel")
    return downloads


def manifest_values(path: Path) -> dict[str, str]:
    return dict(line.split("=", 1) for line in path.read_text(encoding="utf-8").splitlines())


def test_stable_release_writes_a_complete_manifest_and_checksums(tmp_path: Path) -> None:
    root, downloads, output = make_root(tmp_path), make_downloads(tmp_path), tmp_path / "out"

    manifest = release_assets.stage(root=root, downloads=downloads, output=output, channel="stable", tag="v0.5.0", commit=COMMIT)
    values = manifest_values(manifest)

    assert values["channel"] == "stable" and values["tag"] == "v0.5.0" and values["version"] == "0.5.0" and values["commit"] == COMMIT
    assert values["asset.linux-ubuntu-x86_64"] == "chxchx-studio-ubuntu-x86_64.tar.gz"
    assert values["asset.install-sh"] == "install.sh" and values["asset.install-ps1"] == "install.ps1"
    assert values["asset.cli-wheel"] == "chxchx_tech_lead-0.5.0-py3-none-any.whl"
    for key in ("linux-ubuntu-x86_64", "linux-fedora-x86_64", "windows-x86_64", "windows-setup", "macos-arm64", "macos-x86_64", "cli-wheel", "install-sh", "install-ps1"):
        assert len(values[f"sha256.{key}"]) == 64 and (output / values[f"asset.{key}"]).is_file()
    sums = (output / "SHA256SUMS").read_text(encoding="utf-8").splitlines()
    assert f"{values['sha256.install-sh']}  install.sh" in sums
    assert not (output / "studio-preview.txt").exists()  # only the dev channel keeps the legacy name


def test_dev_release_keeps_the_legacy_manifest_name(tmp_path: Path) -> None:
    root = make_root(tmp_path, "0.5.0.dev0")
    manifest = release_assets.stage(root=root, downloads=make_downloads(tmp_path), output=tmp_path / "out", channel="dev", tag="studio-dev-preview", commit=COMMIT)

    assert manifest.read_text(encoding="utf-8") == (tmp_path / "out" / "studio-preview.txt").read_text(encoding="utf-8")
    assert manifest_values(manifest)["channel"] == "dev" and manifest_values(manifest)["version"] == "0.5.0.dev0"


@pytest.mark.parametrize(("tag", "version", "message"), [
    ("v0.5.1", "0.5.0", "no coincide"),
    ("0.5.0", "0.5.0", "vX.Y.Z"),
    ("v0.5", "0.5.0", "vX.Y.Z"),
    ("v0.5.0-rc1", "0.5.0", "vX.Y.Z"),
    ("v0.5.0", "0.5.0.dev0", "sin sufijos"),
])
def test_a_stable_tag_must_match_the_declared_version(tmp_path: Path, tag: str, version: str, message: str) -> None:
    with pytest.raises(release_assets.ReleaseError, match=message):
        release_assets.stage(root=make_root(tmp_path, version), downloads=make_downloads(tmp_path), output=tmp_path / "out",
                             channel="stable", tag=tag, commit=COMMIT)
    assert not (tmp_path / "out" / "studio-manifest.txt").exists()  # nothing is published when the check fails


def test_every_package_must_exist_exactly_once(tmp_path: Path) -> None:
    root = make_root(tmp_path)
    with pytest.raises(release_assets.ReleaseError, match="hay 0"):
        release_assets.stage(root=root, downloads=make_downloads(tmp_path, skip="macos-arm64"), output=tmp_path / "a",
                             channel="stable", tag="v0.5.0", commit=COMMIT)
    with pytest.raises(release_assets.ReleaseError, match="hay 2"):
        release_assets.stage(root=root, downloads=make_downloads(tmp_path / "dup", duplicate="windows-x86_64"), output=tmp_path / "b",
                             channel="stable", tag="v0.5.0", commit=COMMIT)


def test_commit_and_channel_are_validated(tmp_path: Path) -> None:
    root, downloads = make_root(tmp_path), make_downloads(tmp_path)
    with pytest.raises(release_assets.ReleaseError, match="40 caracteres"):
        release_assets.stage(root=root, downloads=downloads, output=tmp_path / "o", channel="stable", tag="v0.5.0", commit="abc123")
    with pytest.raises(release_assets.ReleaseError, match="Canal"):
        release_assets.stage(root=root, downloads=downloads, output=tmp_path / "o", channel="beta", tag="v0.5.0", commit=COMMIT)
    with pytest.raises(release_assets.ReleaseError, match="studio-dev-preview"):
        release_assets.stage(root=root, downloads=downloads, output=tmp_path / "o", channel="dev", tag="v0.5.0", commit=COMMIT)


def test_command_line_check_version(tmp_path: Path, capsys) -> None:
    root = make_root(tmp_path)

    assert release_assets.main(["--root", str(root), "--channel", "stable", "--tag", "v0.5.0", "--check-version"]) == 0
    assert release_assets.main(["--root", str(root), "--channel", "stable", "--tag", "v9.9.9", "--check-version"]) == 1
    assert "no coincide" in capsys.readouterr().err
