"""Small, non-interactive adapter for the official GitHub CLI."""

from __future__ import annotations

import json
import re
from pathlib import Path

from ..core.runner import CommandResult, executable, run

RUN_FIELDS = "databaseId,name,workflowName,headBranch,event,status,conclusion,createdAt,url"
MAX_LOG_OUTPUT = 96_000


class GitHubCliError(RuntimeError):
    pass


def authenticated(root: Path) -> bool:
    return _gh(root, ["auth", "status"], timeout=15).returncode == 0


def recent_runs(
    root: Path, limit: int = 8, *, workflow: str = "chxchx-ci.yml"
) -> list[dict[str, object]]:
    if not _valid_workflow(workflow):
        raise GitHubCliError("El nombre del workflow no es válido.")
    result = _gh(
        root,
        [
            "run", "list", "--workflow", workflow, "--json", RUN_FIELDS,
            "--limit", str(max(1, min(limit, 20))),
        ],
        timeout=30,
    )
    if result.returncode != 0:
        raise GitHubCliError("No pude consultar GitHub Actions. Revisa `gh auth status` y el repositorio remoto.")
    try:
        value = json.loads(result.stdout)
    except json.JSONDecodeError as exc:
        raise GitHubCliError("GitHub CLI devolvió un estado JSON inválido.") from exc
    if not isinstance(value, list):
        raise GitHubCliError("GitHub CLI devolvió una lista de ejecuciones inválida.")
    return [item for item in value if isinstance(item, dict)]


def dispatch_ci(root: Path, ref: str | None = None, *, dry_run: bool = False) -> CommandResult:
    return dispatch_workflow(root, "chxchx-ci.yml", ref, dry_run=dry_run)


def dispatch_workflow(
    root: Path,
    workflow: str,
    ref: str | None = None,
    *,
    fields: dict[str, str] | None = None,
    dry_run: bool = False,
) -> CommandResult:
    if not _valid_workflow(workflow):
        raise GitHubCliError("El nombre del workflow no es válido.")
    if ref is not None and not _valid_ref(ref):
        raise GitHubCliError("La referencia de Git debe ser un nombre de rama o tag válido.")
    args = ["workflow", "run", workflow]
    if ref:
        args.extend(("--ref", ref))
    for name, value in (fields or {}).items():
        if not re.fullmatch(r"[a-z][a-z0-9_]{0,63}", name) or not re.fullmatch(r"[A-Za-z0-9._/-]{1,255}", value):
            raise GitHubCliError("El campo de workflow no tiene un formato seguro.")
        args.extend(("-f", f"{name}={value}"))
    if dry_run:
        return CommandResult(
            ["gh", *args],
            0,
            "Previsualización; no se contactó GitHub ni se inició una ejecución.",
            "",
        )
    if not authenticated(root):
        raise GitHubCliError("GitHub CLI no tiene sesión activa. Ejecuta `gh auth login` y repite la acción.")
    result = _gh(root, args, timeout=45)
    if result.returncode != 0:
        raise GitHubCliError(
            f"No pude iniciar {workflow}. Confirma que el archivo esté en la rama seleccionada y que `gh` tenga acceso."
        )
    return result


def failed_logs(root: Path, run_id: str) -> CommandResult:
    if not re.fullmatch(r"[0-9]{1,20}", run_id):
        raise GitHubCliError("El ID de ejecución debe ser numérico.")
    if not authenticated(root):
        raise GitHubCliError("GitHub CLI no tiene sesión activa. Ejecuta `gh auth login` y repite la acción.")
    result = _gh(root, ["run", "view", run_id, "--log-failed"], timeout=90)
    if result.returncode != 0:
        raise GitHubCliError("No pude leer los logs fallidos de esta ejecución.")
    output = result.stdout
    if len(output) > MAX_LOG_OUTPUT:
        output = output[-MAX_LOG_OUTPUT:]
        output = "[Se muestra el tramo final; salida limitada a 96 KB]\n" + output
    return CommandResult(result.command, result.returncode, output, result.stderr)


def _gh(root: Path, args: list[str], *, timeout: float) -> CommandResult:
    path = executable("gh")
    if path is None:
        raise GitHubCliError("GitHub CLI (`gh`) no está instalado o no está en PATH.")
    return run([path, *args], cwd=root, timeout=timeout)


def _valid_ref(value: str) -> bool:
    return bool(
        value
        and len(value) <= 255
        and re.fullmatch(r"[A-Za-z0-9._/-]+", value)
        and not value.startswith(("-", "/"))
        and not value.endswith(("/", ".", ".lock"))
        and ".." not in value
        and "//" not in value
    )


def _valid_workflow(value: str) -> bool:
    return bool(re.fullmatch(r"chxchx-[a-z0-9-]+\.yml", value))
