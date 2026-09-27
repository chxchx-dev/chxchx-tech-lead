from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path


@dataclass(frozen=True)
class MemoryNote:
    """A read-only view of a Markdown note belonging to one project."""

    path: Path
    title: str
    modified_at: datetime
    content: str

    @property
    def preview(self) -> str:
        for line in self.content.splitlines():
            text = line.strip().lstrip("#*- ").strip()
            if text:
                return text[:180]
        return "(nota vacía)"


def list_memory_notes(project_root: Path, query: str = "") -> list[MemoryNote]:
    """List notes under this project's `.ai/memory`, newest file updates first."""
    memory_root = (project_root / ".ai" / "memory").resolve()
    if not memory_root.is_dir():
        return []

    normalized_query = query.strip().casefold()
    notes: list[MemoryNote] = []
    for path in memory_root.rglob("*.md"):
        try:
            resolved = path.resolve(strict=True)
            if not resolved.is_relative_to(memory_root) or not resolved.is_file():
                continue
            content = resolved.read_text(encoding="utf-8")
            stat = resolved.stat()
        except (OSError, UnicodeError):
            continue

        title = next(
            (line.strip().lstrip("# ").strip() for line in content.splitlines() if line.startswith("# ")),
            resolved.stem,
        )
        if normalized_query and normalized_query not in f"{title}\n{content}".casefold():
            continue
        notes.append(
            MemoryNote(
                path=resolved,
                title=title,
                modified_at=datetime.fromtimestamp(stat.st_mtime).astimezone(),
                content=content,
            )
        )

    return sorted(notes, key=lambda note: note.modified_at, reverse=True)
