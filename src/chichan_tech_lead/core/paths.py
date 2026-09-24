from __future__ import annotations

import os
from pathlib import Path


def home_dir() -> Path:
    override = os.getenv("CHICHAN_HOME")
    if override:
        return Path(override).expanduser().resolve()
    return Path.home() / ".chichan-tech-lead"


def ensure_home() -> Path:
    home = home_dir()
    (home / "backups").mkdir(parents=True, exist_ok=True)
    (home / "logs").mkdir(parents=True, exist_ok=True)
    (home / "profiles").mkdir(parents=True, exist_ok=True)
    return home
