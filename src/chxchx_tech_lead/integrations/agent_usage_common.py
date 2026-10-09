"""Shared parsing helpers for local agent usage sources."""

from __future__ import annotations

import os
from pathlib import Path


def positive_int(value: object) -> int | None:
    try:
        number = int(value)
    except (TypeError, ValueError):
        return None
    return number if number > 0 else None


def number(value: object) -> float | None:
    try:
        result = float(value)
    except (TypeError, ValueError):
        return None
    return result if result >= 0 else None


def tail_lines(stream) -> list[bytes]:
    stream.seek(0, os.SEEK_END)
    stream.seek(max(0, stream.tell() - 1_000_000))
    if stream.tell():
        stream.readline()
    return stream.readlines()


def modified_at(path: Path) -> float:
    try:
        return path.stat().st_mtime
    except OSError:
        return 0.0


def same_path(path: Path, expected: Path) -> bool:
    try:
        return path.expanduser().resolve() == expected
    except OSError:
        return False
