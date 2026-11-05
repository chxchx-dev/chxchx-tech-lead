from dataclasses import replace
import http.server
import threading
from pathlib import Path

import pytest

from chxchx_tech_lead.integrations import studio_update_feed as feed
from chxchx_tech_lead.integrations.studio_channels import DEV, STABLE, channel_of_root, root_name
from chxchx_tech_lead.integrations.studio_update_launcher import switch_launcher

COMMIT = "a" * 40
SHA = "b" * 64


def manifest(version="0.5.0", channel=None, extra=""):
    lines = ["schema_version=1", f"commit={COMMIT}", f"version={version}",
             "asset.cli-wheel=chxchx_tech_lead-0.5.0-py3-none-any.whl", f"sha256.cli-wheel={SHA}",
             "asset.linux-ubuntu-x86_64=chxchx-studio-ubuntu-x86_64.tar.gz", f"sha256.linux-ubuntu-x86_64={SHA}"]
    if channel:
        lines.append(f"channel={channel}")
    return "\n".join(lines) + extra + "\n"


def parse(text, channel):
    return feed.parse_manifest(text, system="Linux", machine="x86_64", channel=channel)


@pytest.fixture(autouse=True)
def ubuntu(monkeypatch):
    monkeypatch.setattr(feed, "_linux_id", lambda: "ubuntu")


def test_roots_are_assigned_to_their_channel():
    assert channel_of_root(Path("dev-0123456789ab")) is DEV
    assert channel_of_root(Path("stable-1.2.3")) is STABLE
    assert channel_of_root(Path("ChxChx Studio Dev-0123456789ab.app")) is DEV
    assert channel_of_root(Path("ChxChx Studio-1.2.3.app")) is STABLE
    assert channel_of_root(Path("stable-1.2")) is None
    assert channel_of_root(Path("other")) is None


def test_root_names():
    assert root_name(STABLE, "1.2.3", COMMIT, mac=False) == "stable-1.2.3"
    assert root_name(DEV, "0.5.0.dev0", COMMIT, mac=False) == "dev-" + COMMIT[:12]
    assert root_name(STABLE, "1.2.3", COMMIT, mac=True) == "ChxChx Studio-1.2.3.app"
    assert root_name(DEV, "x", COMMIT, mac=True) == f"ChxChx Studio Dev-{COMMIT[:12]}.app"


def test_manifest_channel_must_match():
    assert parse(manifest(channel="stable"), "stable").channel == "stable"
    with pytest.raises(ValueError, match="canal"):
        parse(manifest(channel="dev"), "stable")
    assert parse(manifest(), "dev").channel == "dev"  # manifests without channel are the old preview


def test_stable_requires_a_plain_version():
    with pytest.raises(ValueError, match="X.Y.Z"):
        parse(manifest(version="0.5.0.dev0"), "stable")


def test_fetch_falls_back_to_the_old_preview_manifest(monkeypatch):
    monkeypatch.setattr(feed, "platform_asset", lambda *a: ("linux-ubuntu-x86_64", "chxchx-studio-ubuntu-x86_64.tar.gz"))
    served = {"/studio-preview.txt": manifest()}

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            body = served.get(self.path)
            self.send_response(200 if body else 404)
            self.end_headers()
            if body:
                self.wfile.write(body.encode())

        def log_message(self, *args):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    try:
        channel = replace(DEV, base=f"http://127.0.0.1:{server.server_port}")
        assert feed.fetch_release(channel).commit == COMMIT
        served.clear()
        with pytest.raises(ValueError, match="manifiesto"):
            feed.fetch_release(channel)
    finally:
        server.shutdown()


def test_linux_family_follows_id_like(tmp_path, monkeypatch):
    monkeypatch.undo()
    release = tmp_path / "os-release"
    real_read = Path.read_text

    def fake(self, *a, **k):
        return release.read_text() if self.as_posix() == "/etc/os-release" else real_read(self, *a, **k)

    monkeypatch.setattr(Path, "read_text", fake)
    for text, expected in (('ID=linuxmint\nID_LIKE="ubuntu debian"\n', "ubuntu"), ("ID=nobara\nID_LIKE=fedora\n", "fedora"),
                           ("ID=arch\n", "")):
        release.write_text(text)
        assert feed._linux_id() == expected


def test_launcher_switch_is_channel_aware_and_respects_foreign_files(tmp_path, monkeypatch):
    monkeypatch.setenv("HOME", str(tmp_path))
    monkeypatch.setattr(Path, "home", classmethod(lambda cls: tmp_path))
    root = tmp_path / "opt" / "stable-1.2.3"
    (root / ".venv" / "bin").mkdir(parents=True)
    launcher = tmp_path / ".local" / "bin" / "chxchx-studio"
    shim = tmp_path / ".local" / "bin" / "chxchx-tech"
    launcher.parent.mkdir(parents=True)
    shim.write_text("#!/bin/sh\n# CHXCHX_CLI_SHIM\nexec old\n")

    switch_launcher("Linux", root, launcher, STABLE, root)
    text = launcher.read_text()
    assert "CHXCHX_STUDIO_LAUNCHER channel=stable" in text and str(root) in text
    assert "CHXCHX_STUDIO_DEV_LAUNCHER" not in text
    assert str(root / ".venv" / "bin" / "chxchx-tech") in shim.read_text()

    switch_launcher("Linux", root, launcher, STABLE, root)  # idempotent

    launcher.write_text("#!/bin/sh\necho mine\n")
    with pytest.raises(ValueError, match="ajeno"):
        switch_launcher("Linux", root, launcher, STABLE, root)
    assert "mine" in launcher.read_text()


def test_foreign_shim_is_left_alone(tmp_path, monkeypatch):
    monkeypatch.setattr(Path, "home", classmethod(lambda cls: tmp_path))
    launcher = tmp_path / "bin" / "chxchx-studio-dev"
    launcher.parent.mkdir()
    (launcher.parent / "chxchx-tech").write_text("#!/bin/sh\necho uv\n")
    switch_launcher("Linux", tmp_path / "dev-aaaaaaaaaaaa", launcher, DEV, tmp_path)
    assert "echo uv" in (launcher.parent / "chxchx-tech").read_text()
    assert "CHXCHX_STUDIO_DEV_LAUNCHER" in launcher.read_text()


def test_windows_arm64_prefers_a_native_package_and_falls_back_to_x64():
    with_native = manifest(extra=f"\nasset.windows-arm64=chxchx-studio-windows-arm64.zip\nsha256.windows-arm64={SHA}\nasset.windows-x86_64=chxchx-studio-windows-x86_64.zip\nsha256.windows-x86_64={SHA}")
    assert feed.parse_manifest(with_native, system="Windows", machine="ARM64").platform == "windows-arm64"
    x64_only = manifest(extra=f"\nasset.windows-x86_64=chxchx-studio-windows-x86_64.zip\nsha256.windows-x86_64={SHA}")
    assert feed.parse_manifest(x64_only, system="Windows", machine="ARM64").platform == "windows-x86_64"
    with pytest.raises(ValueError):
        feed.platform_asset("Windows", "x86")
