from __future__ import annotations

from pathlib import Path, PurePosixPath, PureWindowsPath
from string import Formatter
from typing import Any


class WorkspaceConfigError(ValueError):
    """Indica que la configuración del workspace no es segura o válida."""


def as_bool(value: Any, field_name: str, default: bool = False) -> bool:
    if value is None:
        return default
    if not isinstance(value, bool):
        raise WorkspaceConfigError(f"{field_name} debe ser booleano")
    return value


def as_string(value: Any, field_name: str, *, default: str | None = None) -> str:
    if value is None and default is not None:
        return default
    if not isinstance(value, str) or not value.strip():
        raise WorkspaceConfigError(f"{field_name} debe ser un texto no vacío")
    return value.strip()


def relative_path(value: Any, field_name: str) -> str:
    path = as_string(value, field_name)
    candidate = Path(path)
    posix_candidate = PurePosixPath(path)
    windows_candidate = PureWindowsPath(path)
    if (
        candidate.is_absolute()
        or posix_candidate.is_absolute()
        or windows_candidate.is_absolute()
        or windows_candidate.drive
        or ".." in candidate.parts
        or ".." in posix_candidate.parts
        or ".." in windows_candidate.parts
    ):
        raise WorkspaceConfigError(f"{field_name} debe ser una ruta relativa dentro del proyecto")
    return path


def command(value: Any, field_name: str, shell: bool) -> list[str] | str:
    if isinstance(value, list) and value and all(isinstance(item, str) and item for item in value):
        return list(value)
    if shell and isinstance(value, str) and value.strip():
        return value
    if shell:
        raise WorkspaceConfigError(f"{field_name} debe ser un comando de texto no vacío")
    raise WorkspaceConfigError(f"{field_name} debe ser una lista de argumentos no vacía")


def port(value: Any, field_name: str) -> int | None:
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, int) or not 1 <= value <= 65535:
        raise WorkspaceConfigError(f"{field_name} debe ser un puerto entre 1 y 65535")
    return value


def validate_header_template(template: str) -> None:
    allowed = {"logo", "label", "project", "profile"}
    for _literal, field_name, _format_spec, _conversion in Formatter().parse(template):
        if field_name and field_name not in allowed:
            raise WorkspaceConfigError(
                f"workspace.header.template solo admite: {', '.join(sorted(allowed))}"
            )
