"""Keep one Studio per channel: remove the managed installs an update left behind.

Every update installs beside the running version (so a failed update never breaks it) and
switches the launcher. Without cleanup each update left another `ChxChx Studio-X.Y.Z.app`
next to the previous ones. Only directories ChxChx itself installed are touched: they must
match the channel's naming and carry the `.chxchx-managed` marker.
"""

from __future__ import annotations

from pathlib import Path
import re
import shutil

from .studio_channels import CHANNELS, Channel


def _managed(parent: Path, channel: Channel) -> list[Path]:
    found = []
    for entry in parent.iterdir() if parent.is_dir() else ():
        if entry.is_symlink() or not entry.is_dir() or not (entry / ".chxchx-managed").is_file():
            continue
        if re.fullmatch(channel.root_pattern, entry.name) or re.fullmatch(channel.app_pattern, entry.name, re.IGNORECASE):
            found.append(entry)
    return found


def _installed_at(root: Path) -> float:
    return (root / ".chxchx-managed").stat().st_mtime


def prune_stale_installs(root: Path, channel_name: str, *, keep: tuple[Path, ...] = ()) -> list[Path]:
    """Delete the older managed installs beside `root` (any of them) and return what was removed.

    Kept: the newest install of the channel and every path in `keep` (the running one, which
    cannot be deleted on Windows and would pull the files from under a live Studio elsewhere).
    A directory that cannot be removed now (still in use) is left for the next update.
    """
    channel = CHANNELS.get(channel_name)
    if channel is None:
        return []
    candidates = _managed(root.parent, channel)
    if not candidates:
        return []
    protected = {path.resolve() for path in keep}
    protected.add(max(candidates, key=_installed_at).resolve())
    removed = []
    for entry in candidates:
        if entry.resolve() in protected:
            continue
        shutil.rmtree(entry, ignore_errors=True)
        if not entry.exists():
            removed.append(entry)
    return removed
