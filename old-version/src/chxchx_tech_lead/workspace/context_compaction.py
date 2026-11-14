"""Keep agent context files short by archiving their older sections.

Agents prepend a dated section to `.ai/CURRENT_STATE.md` and `.ai/HANDOFF.md`
after each piece of work, so both files grow into logs that every new chat has
to read. Compaction keeps the most recent sections plus the canonical ones
created by `init`, and moves the rest, newest first, to `.ai/history/`.
"""

from __future__ import annotations

import os
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path

from ..core.backup import backup_project

COMPACTED_FILES = ("CURRENT_STATE.md", "HANDOFF.md")
DEFAULT_KEEP_SECTIONS = 6
DEFAULT_BUDGET_CHARS = 12_000
# Sections created by `init` templates; they hold the canonical status.
PINNED_HEADINGS = frozenset(
    {"estado actual", "en progreso", "bloqueos", "último cambio", "pendiente", "validación"}
)
_MANAGED_START = "<!-- chxchx-tech:start"
_MANAGED_END = "<!-- chxchx-tech:end"
_ARCHIVE_INTRO = "Secciones archivadas automáticamente, de la más reciente a la más antigua.\n\n"


@dataclass(frozen=True, slots=True)
class CompactedFile:
    path: Path
    archive: Path
    moved: tuple[str, ...]
    chars_before: int
    chars_after: int


@dataclass(frozen=True, slots=True)
class CompactionResult:
    files: tuple[CompactedFile, ...]
    backup: Path | None

    @property
    def changed(self) -> bool:
        return any(item.moved for item in self.files)


def auto_compact_enabled(project_root: Path) -> bool:
    """Projects opt out with `auto_compact_context = false` in their TOML."""
    try:
        with (project_root / ".ai" / "chxchx-tech.toml").open("rb") as stream:
            config = tomllib.load(stream)
    except (OSError, tomllib.TOMLDecodeError):
        return True
    return config.get("auto_compact_context", True) is not False


def compact_context(
    project_root: Path,
    *,
    keep: int = DEFAULT_KEEP_SECTIONS,
    budget: int = DEFAULT_BUDGET_CHARS,
    dry_run: bool = False,
) -> CompactionResult:
    root = project_root.resolve()
    plans: list[tuple[CompactedFile, str, str]] = []
    for name in COMPACTED_FILES:
        path = root / ".ai" / name
        if not path.is_file():
            continue
        archive = root / ".ai" / "history" / name
        original = path.read_text(encoding="utf-8")
        compacted, moved = _compact_text(original, name, keep=max(keep, 1), budget=budget)
        headings = tuple(section.splitlines()[0].removeprefix("## ").strip() for section in moved)
        plans.append(
            (
                CompactedFile(path, archive, headings, len(original), len(compacted)),
                compacted,
                "".join(moved),
            )
        )
    files = tuple(plan[0] for plan in plans)
    if dry_run or not any(item.moved for item in files):
        return CompactionResult(files, None)
    backup = backup_project(root)
    for item, compacted, moved_text in plans:
        if not item.moved:
            continue
        _atomic_write(item.archive, _prepend_archive(item.archive, item.path.name, moved_text))
        _atomic_write(item.path, compacted)
    return CompactionResult(files, backup)


def _compact_text(text: str, name: str, *, keep: int, budget: int) -> tuple[str, list[str]]:
    head, sections = _split_sections(text)
    size = len(head)
    kept: list[str] = []
    moved: list[str] = []
    recent = 0
    for section in sections:
        if _is_pinned(section):
            kept.append(section)
            size += len(section)
            continue
        # Once one section is archived, every older one goes too, so the
        # remaining file stays a contiguous "most recent" window.
        fits = recent < keep and size + len(section) <= budget
        if not moved and (recent == 0 or fits):
            kept.append(section)
            size += len(section)
            recent += 1
        else:
            moved.append(_with_trailing_blank_line(section))
    if not moved:
        return text, []
    pointer = f"> Historial anterior: `.ai/history/{name}` (no se lee al iniciar un chat).\n"
    if pointer not in head:
        head = _with_trailing_blank_line(head.rstrip("\n") + "\n\n" + pointer)
    return (head + "".join(kept)).rstrip("\n") + "\n", moved


def _split_sections(text: str) -> tuple[str, list[str]]:
    head: list[str] = []
    sections: list[list[str]] = []
    in_fence = in_managed = False
    for line in text.splitlines(keepends=True):
        stripped = line.strip()
        starts_section = not in_fence and not in_managed and (
            line.startswith("## ") or stripped.startswith(_MANAGED_START)
        )
        if starts_section:
            sections.append([line])
        else:
            (sections[-1] if sections else head).append(line)
        if stripped.startswith("```"):
            in_fence = not in_fence
        elif stripped.startswith(_MANAGED_START):
            in_managed = True
        elif stripped.startswith(_MANAGED_END):
            in_managed = False
    return "".join(head), ["".join(section) for section in sections]


def _is_pinned(section: str) -> bool:
    first = section.splitlines()[0] if section else ""
    heading = first.removeprefix("## ").strip().casefold()
    return heading in PINNED_HEADINGS or _MANAGED_START in section


def _prepend_archive(archive: Path, name: str, moved_text: str) -> str:
    title = f"# Historial de {name.removesuffix('.md')}\n\n"
    existing = archive.read_text(encoding="utf-8") if archive.is_file() else ""
    body = existing.removeprefix(title).removeprefix(_ARCHIVE_INTRO)
    return (title + _ARCHIVE_INTRO + moved_text + body).rstrip("\n") + "\n"


def _with_trailing_blank_line(text: str) -> str:
    return text.rstrip("\n") + "\n\n"


def _atomic_write(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=".chxchx-context-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as stream:
            stream.write(content)
        os.replace(temporary, path)
    except OSError:
        try:
            os.unlink(temporary)
        except OSError:
            pass
        raise
