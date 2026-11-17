from dataclasses import replace
from types import SimpleNamespace

import pytest

from chxchx_tech_lead.integrations import studio_update_install as update
from chxchx_tech_lead.integrations import studio_update_msi as msi
from chxchx_tech_lead.integrations.studio_channels import STABLE
from chxchx_tech_lead.integrations.studio_update_feed import StudioRelease

COMMIT = "a" * 40
NEXT_COMMIT = "b" * 40
SHA = "c" * 64


def release() -> StudioRelease:
    return StudioRelease(
        commit=NEXT_COMMIT,
        version="0.5.1",
        platform="windows-x86_64",
        package="chxchx-studio-windows-x86_64.zip",
        package_sha256=SHA,
        wheel="chxchx_tech_lead-0.5.1-py3-none-any.whl",
        wheel_sha256=SHA,
        channel="stable",
        windows_msi="chxchx-studio-windows-x86_64.msi",
        windows_msi_sha256=SHA,
    )


def test_msi_install_downloads_the_named_asset_and_reports_reboot(tmp_path, monkeypatch):
    artifact = release()
    package = tmp_path / "chxchx-studio-windows-x86_64.msi"
    calls = {}

    def download(name, checksum, destination, *, base):
        calls["download"] = (name, checksum, base)
        destination.write_bytes(b"verified MSI")

    monkeypatch.setattr(msi.platform, "system", lambda: "Windows")
    monkeypatch.setattr(msi, "download_asset", download)
    msiexec = tmp_path / "msiexec.exe"
    msiexec.write_bytes(b"test executable")
    monkeypatch.setattr(msi.shutil, "which", lambda _name: str(msiexec))
    def run(command, **kwargs):
        calls.update(command=command, options=kwargs)
        return SimpleNamespace(returncode=3010)

    monkeypatch.setattr(msi.subprocess, "run", run)

    assert msi.install_windows_msi(artifact, tmp_path, base="https://example.test/release") is True
    assert calls["download"] == (artifact.windows_msi, SHA, "https://example.test/release")
    assert calls["command"] == [str(msiexec), "/i", str(package), "/qn", "/norestart"]
    assert calls["options"]["creationflags"] == 0x08000000


def test_msi_install_rejects_missing_release_asset(tmp_path, monkeypatch):
    artifact = release()
    artifact = replace(artifact, windows_msi=None, windows_msi_sha256=None)
    monkeypatch.setattr(msi.platform, "system", lambda: "Windows")
    with pytest.raises(ValueError, match="no incluye el MSI"):
        msi.install_windows_msi(artifact, tmp_path, base="https://example.test/release")


def test_msi_managed_update_calls_windows_installer_not_the_zip_path(tmp_path, monkeypatch):
    root = tmp_path / "stable-0.5.0"
    root.mkdir()
    (root / ".chxchx-managed").write_text(COMMIT)
    (root / ".chxchx-msi").write_text("channel=stable\n")
    artifact = release()
    applied = []

    monkeypatch.setattr(update, "installed_commit", lambda: COMMIT)
    monkeypatch.setattr(update, "installed_channel", lambda: STABLE)
    monkeypatch.setattr(update, "installed_app_root", lambda: root)
    monkeypatch.setattr(update, "fetch_release", lambda _channel: artifact)
    monkeypatch.setattr(update, "install_windows_msi", lambda got, _work, *, base: applied.append((got, base)) or True)
    monkeypatch.setattr(update, "download_asset", lambda *_args, **_kwargs: (_ for _ in ()).throw(AssertionError("ZIP route used")))

    result = update.apply_update()

    assert result["updated"] is True
    assert result["restart_required"] is True
    assert applied == [(artifact, update.CHANNELS["stable"].base)]
