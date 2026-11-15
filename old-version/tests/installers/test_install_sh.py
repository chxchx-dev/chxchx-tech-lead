"""The Linux/macOS installer, run for real against a fake release served on localhost."""

from __future__ import annotations

import shutil
import sys
import time
from pathlib import Path

import pytest

pytestmark = [
    pytest.mark.skipif(sys.platform != "linux", reason="los paquetes falsos son de Linux; macOS se prueba con los reales en CI"),
    pytest.mark.skipif(shutil.which("curl") is None or shutil.which("bash") is None, reason="necesita curl y bash"),
]
FLAGS = ("--no-deps",)


def install(box, release, *args: str, **extra: str):
    return box.run(*FLAGS, *args, release=release, **extra)


def test_dry_run_verifies_the_download_and_changes_nothing(box, make_release) -> None:
    release = make_release()

    result = install(box, release, "--dry-run")

    assert result.returncode == 0, result.stderr
    assert "DRY RUN" in result.stdout and "0.5.0" in result.stdout and "linux-ubuntu-x86_64" in result.stdout
    assert box.tree() == set()  # not even ~/.local was created


def test_a_stable_install_sets_up_studio_the_command_and_the_app_menu(box, make_release) -> None:
    release = make_release()

    result = install(box, release)

    assert result.returncode == 0, result.stderr
    root = box.home / ".local/opt/chxchx-studio/stable-0.5.0"
    assert (root / ".chxchx-managed").read_text().strip() == release.commit
    assert (root / ".chxchx-install").read_text().splitlines() == ["channel=stable", "version=0.5.0", f"commit={release.commit}"]
    studio = box.command("chxchx-studio", "/ruta")
    assert studio.returncode == 0 and "chxchx-studio 0.5.0" in studio.stdout and "args=/ruta" in studio.stdout
    cli_path = studio.stdout.split("cli=")[1].split()[0]
    assert cli_path == str(root / ".venv/bin/chxchx-tech") and Path(cli_path).is_file()
    assert box.command("chxchx-tech", "--version").stdout.strip() == "chxchx-tech 0.5.0"
    desktop = (box.home / ".local/share/applications/chxchx-studio.desktop").read_text()
    assert 'Exec="%s" %%f' % (box.home / ".local/bin/chxchx-studio") in desktop and "Name=ChxChx Studio\n" in desktop
    assert "CHXCHX_STUDIO_DESKTOP" in desktop and "Icon=chxchx-studio" in desktop
    assert (box.home / ".local/share/icons/hicolor/256x256/apps/chxchx-studio.png").is_file()
    assert "quedó instalado" in result.stdout and "desinstalar" in result.stdout


def test_installing_again_is_safe_and_update_needs_a_previous_install(box, make_release) -> None:
    release = make_release()
    refused = install(box, release, "--update")
    assert refused.returncode != 0 and "--update requiere una instalación previa" in refused.stderr and box.tree() == set()

    assert install(box, release).returncode == 0
    first = box.tree()
    assert install(box, release).returncode == 0 and install(box, release, "--update").returncode == 0
    assert box.tree() == first


def test_the_dev_preview_installs_next_to_stable_without_touching_it(box, make_release) -> None:
    stable, dev = make_release(), make_release(version="0.5.1", channel="dev")
    assert install(box, stable).returncode == 0

    result = install(box, dev, "--channel", "dev")

    assert result.returncode == 0, result.stderr
    assert (box.home / f".local/opt/chxchx-studio/dev-{dev.commit[:12]}/bin/chxchx-studio").is_file()
    assert (box.home / ".local/opt/chxchx-studio/stable-0.5.0/bin/chxchx-studio").is_file()
    assert "chxchx-studio 0.5.0" in box.command("chxchx-studio").stdout and "chxchx-studio 0.5.1" in box.command("chxchx-studio-dev").stdout
    assert "Name=ChxChx Studio Dev\n" in (box.home / ".local/share/applications/chxchx-studio-dev.desktop").read_text()
    assert box.command("chxchx-tech").stdout.strip() == "chxchx-tech 0.5.1"  # the command follows the latest install


def test_a_tampered_download_is_rejected_before_anything_is_installed(box, make_release) -> None:
    release = make_release(corrupt=True)

    result = install(box, release)

    assert result.returncode != 0 and "SHA-256 de Studio no coincide" in result.stderr
    assert box.tree() == {".local"} or not any("chxchx" in path for path in box.tree())
    assert not (box.home / ".local/bin/chxchx-studio").exists()


def test_files_that_are_not_chxchx_are_never_overwritten(box, make_release) -> None:
    release = make_release()
    bin_dir = box.home / ".local/bin"
    bin_dir.mkdir(parents=True)
    (bin_dir / "chxchx-studio").write_text("#!/bin/sh\necho mio\n")

    refused = install(box, release)
    assert refused.returncode != 0 and "no está administrado por ChxChx" in refused.stderr
    assert (bin_dir / "chxchx-studio").read_text() == "#!/bin/sh\necho mio\n" and not (box.home / ".local/opt").exists()

    (bin_dir / "chxchx-studio").unlink()
    (bin_dir / "chxchx-tech").write_text("#!/bin/sh\necho uv\n")
    kept = install(box, release)
    assert kept.returncode == 0 and "no lo reemplacé" in kept.stderr
    assert (bin_dir / "chxchx-tech").read_text() == "#!/bin/sh\necho uv\n"  # a uv-installed command stays


def test_cli_only_installs_just_the_command(box, make_release) -> None:
    release = make_release()

    result = install(box, release, "--cli-only")

    assert result.returncode == 0, result.stderr
    assert box.command("chxchx-tech").stdout.strip() == "chxchx-tech 0.5.0"
    assert (box.home / ".local/opt/chxchx-tech/stable-0.5.0/.chxchx-managed").is_file()
    assert not (box.home / ".local/opt/chxchx-studio").exists() and not (box.home / ".local/share").exists()
    assert not (box.home / ".local/bin/chxchx-studio").exists()


def test_only_the_new_and_the_previous_version_are_kept(box, make_release) -> None:
    for version in ("0.5.0", "0.5.1", "0.5.2"):
        assert install(box, make_release(version=version)).returncode == 0
        time.sleep(0.05)

    kept = sorted(path.name for path in (box.home / ".local/opt/chxchx-studio").iterdir())

    assert kept == ["stable-0.5.1", "stable-0.5.2"]
    assert "chxchx-studio 0.5.2" in box.command("chxchx-studio").stdout


def test_uninstall_removes_only_what_chxchx_installed(box, make_release) -> None:
    assert install(box, make_release()).returncode == 0
    assert install(box, make_release(version="0.5.1", channel="dev"), "--channel", "dev").returncode == 0
    (box.home / ".local/bin/otro").write_text("mio")
    data = box.home / ".chxchx-tech-lead/trusted_projects.json"
    data.parent.mkdir()
    data.write_text("{}")

    listing = box.run("--uninstall", "--dry-run")
    assert listing.returncode == 0 and "Se quitaría" in listing.stdout and "DRY RUN" in listing.stdout
    assert (box.home / ".local/opt/chxchx-studio").exists() and (box.home / ".local/bin/chxchx-tech").exists()

    result = box.run("--uninstall")

    assert result.returncode == 0, result.stderr
    assert not (box.home / ".local/opt").exists()
    assert sorted(path.name for path in (box.home / ".local/bin").iterdir()) == ["otro"]
    assert not list((box.home / ".local/share/applications").glob("chxchx*")) and not (box.home / ".local/share/icons/hicolor/256x256/apps/chxchx-studio.png").exists()
    assert data.is_file() and "No se tocaron tus datos" in result.stdout
    assert "No había nada instalado" in box.run("--uninstall").stdout  # a second uninstall is a no-op


def test_uninstall_ignores_look_alikes_that_are_not_managed(box) -> None:
    fake = box.home / ".local/opt/chxchx-studio/stable-9.9.9"
    fake.mkdir(parents=True)
    (fake / "importante.txt").write_text("no es de ChxChx")  # no .chxchx-managed marker

    result = box.run("--uninstall")

    assert result.returncode == 0 and (fake / "importante.txt").is_file()


def test_urls_follow_the_channel_and_version(box) -> None:
    def url(*args: str) -> str:
        env = box.env()
        env.pop("CHXCHX_TEST_BASE", None)
        import subprocess
        return subprocess.run(["bash", str(Path(__file__).resolve().parents[2] / "scripts/install.sh"), "--print-urls", *args],
                              env=env, capture_output=True, text=True, timeout=30).stdout.strip()

    releases = "https://github.com/chxchx-dev/chxchx-tech-lead/releases"
    assert url() == f"{releases}/latest/download/studio-manifest.txt"
    assert url("--version", "0.5.0") == f"{releases}/download/v0.5.0/studio-manifest.txt"
    assert url("--channel", "dev") == f"{releases}/download/studio-dev-preview/studio-manifest.txt"


@pytest.mark.parametrize(("args", "message"), [
    (("--channel", "beta"), "Canal no válido"),
    (("--version", "1.2"), "X.Y.Z"),
    (("--version", "0.5.0", "--channel", "dev"), "solo existe en el canal stable"),
    (("--uninstall", "--update"), "no se combina"),
    (("--channel",), "necesita un valor"),
    (("--nada",), "Opción no reconocida"),
])
def test_bad_arguments_fail_with_a_clear_message(box, args: tuple[str, ...], message: str) -> None:
    result = box.run(*args)

    assert result.returncode != 0 and message in result.stderr and box.tree() == set()


def test_a_pinned_version_must_match_the_release(box, make_release) -> None:
    result = install(box, make_release(), "--version", "0.5.1")

    assert result.returncode != 0 and "Pediste 0.5.1 pero el release es 0.5.0" in result.stderr


def test_the_test_hook_only_accepts_localhost(box, make_release) -> None:
    result = box.run("--dry-run", CHXCHX_TEST_BASE="http://evil.example.com")

    assert result.returncode != 0 and "solo admite" in result.stderr


def test_refuses_to_run_as_root(box, make_release) -> None:
    box.fake_tool("id", 'if [ "$1" = "-u" ]; then echo 0; else /usr/bin/id "$@"; fi')

    result = install(box, make_release())

    assert result.returncode != 0 and "usuario normal" in result.stderr and box.tree() == {"."} - {"."}


def test_an_old_glibc_is_reported_and_cli_only_still_works(box, make_release) -> None:
    box.fake_tool("getconf", 'echo "glibc 2.31"')
    release = make_release()

    refused = install(box, release)
    assert refused.returncode != 0 and "glibc es 2.31" in refused.stderr and "--cli-only" in refused.stderr
    assert install(box, release, "--cli-only").returncode == 0


def test_unsupported_hardware_gets_the_cli_only_way_out(box, make_release) -> None:
    box.fake_tool("uname", 'if [ "$1" = "-m" ]; then echo aarch64; else /usr/bin/uname "$@"; fi')
    release = make_release()

    refused = install(box, release)
    assert refused.returncode != 0 and "No hay paquete de escritorio para Linux/aarch64" in refused.stderr
    assert install(box, release, "--cli-only").returncode == 0 and box.command("chxchx-tech").returncode == 0


@pytest.mark.parametrize(("os_release", "expected"), [
    ('ID=ubuntu\nID_LIKE=debian\n', "linux-ubuntu-x86_64"),
    ('ID=pop\nID_LIKE="ubuntu debian"\n', "linux-ubuntu-x86_64"),
    ('ID=fedora\n', "linux-fedora-x86_64"),
    ('ID=rocky\nID_LIKE="rhel centos fedora"\n', "linux-fedora-x86_64"),
])
def test_distributions_map_to_the_right_package(box, make_release, tmp_path: Path, os_release: str, expected: str) -> None:
    release_file = tmp_path / "os-release"
    release_file.write_text(os_release)
    env = {"CHXCHX_OS_RELEASE": str(release_file), "CHXCHX_LINUX_FLAVOR": ""}

    result = install(box, make_release(), "--dry-run", **env)

    assert result.returncode == 0, result.stderr
    assert expected in result.stdout


def test_an_unknown_distribution_explains_the_options(box, make_release, tmp_path: Path) -> None:
    release_file = tmp_path / "os-release"
    release_file.write_text("ID=arch\n")

    result = install(box, make_release(), "--dry-run", CHXCHX_OS_RELEASE=str(release_file), CHXCHX_LINUX_FLAVOR="")

    assert result.returncode != 0 and "No reconozco tu distribución (arch)" in result.stderr and "CHXCHX_LINUX_FLAVOR" in result.stderr
