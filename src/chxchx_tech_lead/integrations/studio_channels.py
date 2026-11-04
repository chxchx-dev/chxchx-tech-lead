"""Release channels of Studio: where each one is published and how its installs are named."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re

OWNER = "chxchx-dev"
REPOSITORY = "chxchx-tech-lead"
RELEASES = f"https://github.com/{OWNER}/{REPOSITORY}/releases"
DEV_TAG = "studio-dev-preview"


@dataclass(frozen=True)
class Channel:
    name: str
    base: str
    manifests: tuple[str, ...]
    label: str
    launcher: str
    root_pattern: str
    app_pattern: str


DEV = Channel(
    "dev", f"{RELEASES}/download/{DEV_TAG}", ("studio-manifest.txt", "studio-preview.txt"),
    "ChxChx Studio Dev", "chxchx-studio-dev", r"dev-[0-9a-f]{12}", r"ChxChx Studio Dev-[0-9a-f]{12}\.app",
)
STABLE = Channel(
    "stable", f"{RELEASES}/latest/download", ("studio-manifest.txt",),
    "ChxChx Studio", "chxchx-studio", r"stable-[0-9]+\.[0-9]+\.[0-9]+", r"ChxChx Studio-[0-9]+\.[0-9]+\.[0-9]+\.app",
)
CHANNELS = {channel.name: channel for channel in (STABLE, DEV)}


def channel_of_root(root: Path) -> Channel | None:
    """The channel an installed root belongs to, judging only by its managed name."""
    for channel in CHANNELS.values():
        if re.fullmatch(channel.root_pattern, root.name) or re.fullmatch(channel.app_pattern, root.name, re.IGNORECASE):
            return channel
    return None


def root_name(channel: Channel, version: str, commit: str, *, mac: bool) -> str:
    suffix = commit[:12] if channel.name == "dev" else version
    return f"{channel.label}-{suffix}.app" if mac else f"{channel.name}-{suffix}"
