"""Image attachments of a chat turn: validation and staging inside the project."""

from __future__ import annotations

import shutil
from datetime import datetime
from pathlib import Path

from .service import WorkspaceOperationError

_IMAGE_SUFFIXES = {".png", ".jpg", ".jpeg", ".gif", ".webp"}
_MAX_IMAGE_BYTES = 20 * 1024 * 1024
ATTACHMENTS_DIR = Path(".ai") / "chat-attachments"


def validated_image(path: Path) -> Path:
    resolved = path.expanduser().resolve()
    if resolved.suffix.casefold() not in _IMAGE_SUFFIXES:
        raise WorkspaceOperationError(f"Formato de imagen no soportado: {resolved.name}")
    try:
        size = resolved.stat().st_size
    except OSError as exc:
        raise WorkspaceOperationError(f"No encuentro la imagen: {resolved}") from exc
    if size > _MAX_IMAGE_BYTES:
        raise WorkspaceOperationError(f"La imagen supera 20 MB: {resolved.name}")
    return resolved


def stage_images(project_root: Path, images: tuple[Path, ...]) -> list[Path]:
    """Copy attachments inside the project so every agent mode may read them."""
    folder = project_root / ATTACHMENTS_DIR
    folder.mkdir(parents=True, exist_ok=True)
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    staged = []
    for index, source in enumerate(images):
        target = folder / f"{stamp}-{index}-{source.name}"
        shutil.copy2(source, target)
        staged.append(target)
    return staged
