from __future__ import annotations

import os
from pathlib import Path

import pytest

from chxchx_tech_lead.integrations.studio_update_prune import prune_stale_installs


def _install(parent: Path, name: str, age: int) -> Path:
    root = parent / name
    root.mkdir(parents=True)
    marker = root / ".chxchx-managed"
    marker.write_text("a" * 40 + "\n", encoding="utf-8")
    (root / "payload").write_text("x", encoding="utf-8")
    os.utime(marker, (age, age))
    return root


def test_only_the_newest_and_the_running_install_remain(tmp_path: Path) -> None:
    old = _install(tmp_path, "ChxChx Studio-0.5.2.app", 100)
    running = _install(tmp_path, "ChxChx Studio-0.5.3.app", 200)
    newest = _install(tmp_path, "ChxChx Studio-0.6.0.app", 300)
    removed = prune_stale_installs(newest, "stable", keep=(running,))
    assert removed == [old]
    assert running.exists() and newest.exists() and not old.exists()
    # Next update: the previous running install is no longer protected.
    assert prune_stale_installs(newest, "stable", keep=(newest,)) == [running]


def test_foreign_folders_and_other_channels_are_never_touched(tmp_path: Path) -> None:
    newest = _install(tmp_path, "stable-0.6.0", 300)
    foreign = tmp_path / "stable-0.5.0"
    foreign.mkdir()  # same name, but no ChxChx marker
    other_channel = _install(tmp_path, "dev-0123456789ab", 10)
    link = tmp_path / "chxchx-link"
    try:
        link.symlink_to(newest, target_is_directory=True)
    except OSError as error:
        if getattr(error, "winerror", None) != 1314:
            raise
        pytest.skip("crear enlaces simbólicos de directorio requiere privilegios de Windows")
    assert prune_stale_installs(newest, "stable") == []
    assert foreign.exists() and other_channel.exists() and link.is_symlink()


def test_unknown_channel_removes_nothing(tmp_path: Path) -> None:
    newest = _install(tmp_path, "stable-0.6.0", 300)
    assert prune_stale_installs(newest, "nightly") == []
