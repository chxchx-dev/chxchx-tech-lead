from __future__ import annotations

from ..workspace.manager import WorkspaceInspection


def workspace_start_blocker(
    inspection: WorkspaceInspection,
    *,
    require_trust: bool,
) -> str | None:
    if not inspection.config_path.is_file():
        return (
            "Falta .ai/chxchx-tech.toml. Inicializa el proyecto desde Más → Configuración "
            "antes de confiarlo o iniciarlo."
        )
    if inspection.config is None:
        return "La configuración del workspace no es válida. Revísala desde Más → Configuración."
    if require_trust and not inspection.trusted:
        return "El proyecto requiere confianza. Pulsa «Confiar proyecto» antes de iniciarlo."
    return None
