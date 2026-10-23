import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path


VERIFIER = Path(__file__).parents[1] / "scripts" / "verify_studio_package.py"


def _make_zip(path: Path, names: tuple[str, ...]) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        for name in names:
            archive.writestr(name, "package fixture")


def _make_tgz(path: Path, names: tuple[str, ...]) -> None:
    with tarfile.open(path, "w:gz") as archive:
        for name in names:
            source = path.parent / Path(name).name
            source.write_text("package fixture", encoding="utf-8")
            archive.add(source, arcname=name)
            source.unlink()


def test_windows_package_requires_qt_runtime_and_stages_only_zip(tmp_path: Path):
    package_dir = tmp_path / "packages"
    package_dir.mkdir()
    _make_zip(
        package_dir / "cpack-output.zip",
        (
            "chxchx-studio/bin/chxchx-studio.exe",
            "chxchx-studio/bin/Qt6Core.dll",
            "chxchx-studio/bin/Qt6Widgets.dll",
            "chxchx-studio/bin/Qt6Core5Compat.dll",
            "chxchx-studio/bin/platforms/qwindows.dll",
        ),
    )
    (package_dir / "_CPack_Packages").mkdir()
    output_dir = tmp_path / "staged"
    result = subprocess.run(
        [sys.executable, str(VERIFIER), "--platform", "windows-x86_64", "--package-dir", str(package_dir), "--output-dir", str(output_dir)],
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0, result.stderr
    assert [item.name for item in output_dir.iterdir()] == ["chxchx-studio-windows-x86_64.zip"]


def test_ubuntu_package_fails_if_qt_platform_plugin_is_missing(tmp_path: Path):
    package_dir = tmp_path / "packages"
    package_dir.mkdir()
    _make_tgz(
        package_dir / "cpack-output.tar.gz",
        (
            "chxchx-studio/bin/chxchx-studio",
            "chxchx-studio/bin/qt.conf",
            "chxchx-studio/lib/libQt6Core.so.6",
            "chxchx-studio/lib/libQt6Widgets.so.6",
            "chxchx-studio/lib/libQt6Core5Compat.so.6",
        ),
    )
    result = subprocess.run(
        [sys.executable, str(VERIFIER), "--platform", "linux-ubuntu-x86_64", "--package-dir", str(package_dir), "--output-dir", str(tmp_path / "staged")],
        capture_output=True,
        text=True,
    )

    assert result.returncode != 0
    assert "libqxcb" in result.stderr


def test_macos_bundle_requires_qt_frameworks_and_platform_plugin(tmp_path: Path):
    package_dir = tmp_path / "packages"
    package_dir.mkdir()
    (package_dir / "cpack-output.dmg").touch()
    bundle = tmp_path / "ChxChx Studio.app"
    for relative in (
        "Contents/MacOS/chxchx-studio",
        "Contents/Frameworks/QtCore.framework/Versions/A/QtCore",
        "Contents/Frameworks/QtWidgets.framework/Versions/A/QtWidgets",
        "Contents/Frameworks/QtCore5Compat.framework/Versions/A/QtCore5Compat",
        "Contents/PlugIns/platforms/libqcocoa.dylib",
    ):
        target = bundle / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        target.touch()
        if relative == "Contents/MacOS/chxchx-studio":
            target.chmod(0o755)

    result = subprocess.run(
        [
            sys.executable,
            str(VERIFIER),
            "--platform",
            "macos-arm64",
            "--package-dir",
            str(package_dir),
            "--output-dir",
            str(tmp_path / "staged"),
            "--mounted-app",
            str(bundle),
        ],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    assert (tmp_path / "staged/chxchx-studio-macos-arm64.dmg").exists()
